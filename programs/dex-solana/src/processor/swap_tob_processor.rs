use crate::constants::*;
use crate::error::ErrorCode;
use crate::processor::platform_fee_processor::PlatformFeeV3Processor;
use crate::processor::proxy_swap_processor::ProxySwapProcessor;
use crate::utils::*;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

pub struct SwapToBProcessor;

impl SwapToBProcessor {
    /// Calculate fees if commission is applied to from amount
    fn calculate_from_fees<'info>(
        &self,
        amount_in: u64,
        commission_rate: u32,
        commission_direction: bool,
        platform_fee_rate: Option<u16>,
    ) -> Result<(u64, u64, u64, bool)> {
        let mut commission_amount = 0;
        let mut platform_fee_amount = 0;
        let mut actual_amount_in = amount_in;

        if commission_direction && commission_rate > 0 {
            // Calculate commission and platform fee amounts
            (commission_amount, platform_fee_amount) = calculate_fee_amounts(
                amount_in,
                commission_rate,
                commission_direction,
                platform_fee_rate,
            )?;

            actual_amount_in = actual_amount_in
                .checked_add(commission_amount)
                .ok_or(ErrorCode::CalculationError)?
                .checked_add(platform_fee_amount)
                .ok_or(ErrorCode::CalculationError)?;
        }

        Ok((
            commission_amount,
            platform_fee_amount,
            actual_amount_in,
            commission_amount > 0 || platform_fee_amount > 0,
        ))
    }

    /// Calculate fees if commission is applied to to amount
    fn calculate_to_fees<'info>(
        &self,
        amount_out: u64,
        expected_amount_out: u64,
        commission_rate: u32,
        commission_direction: bool,
        platform_fee_rate: Option<u16>,
        trim_rate: Option<u8>,
        charge_rate: Option<u16>,
    ) -> Result<(u64, u64, u64, u64, u64, bool)> {
        let mut commission_amount = 0;
        let mut platform_fee_amount = 0;
        let mut actual_amount_out = amount_out;

        if !commission_direction && commission_rate > 0 {
            // Calculate commission and platform fee amounts
            (commission_amount, platform_fee_amount) = calculate_fee_amounts(
                amount_out,
                commission_rate,
                commission_direction,
                platform_fee_rate,
            )?;
        }

        // Calculate trim amount
        let (trim_amount, charge_amount) = calculate_trim_amount(
            amount_out,
            expected_amount_out,
            commission_amount,
            platform_fee_amount,
            commission_direction,
            trim_rate,
            charge_rate,
        )?;

        // Calculate actual amount out
        actual_amount_out = actual_amount_out
            .checked_sub(commission_amount)
            .ok_or(ErrorCode::CalculationError)?
            .checked_sub(platform_fee_amount)
            .ok_or(ErrorCode::CalculationError)?
            .checked_sub(trim_amount)
            .ok_or(ErrorCode::CalculationError)?
            .checked_sub(charge_amount)
            .ok_or(ErrorCode::CalculationError)?;

        Ok((
            commission_amount,
            platform_fee_amount,
            trim_amount,
            charge_amount,
            actual_amount_out,
            commission_amount > 0
                || platform_fee_amount > 0
                || trim_amount > 0
                || charge_amount > 0,
        ))
    }

    /// Unwrap wsol to sa
    fn unwrap_wsol_to_sa<'info>(
        &self,
        payer: &AccountInfo<'info>,
        sa_authority: &UncheckedAccount<'info>,
        destination_token_account: &mut InterfaceAccount<'info, TokenAccount>,
        destination_mint: &InterfaceAccount<'info, Mint>,
        destination_token_sa: &Option<UncheckedAccount<'info>>,
        destination_token_program: &Option<Interface<'info, TokenInterface>>,
        amount_out: u64,
    ) -> Result<()> {
        require!(destination_mint.key() == wsol_program::ID, ErrorCode::InvalidMint);
        require!(destination_token_program.is_some(), ErrorCode::DestinationTokenProgramIsNone);
        require!(
            destination_token_account.owner == payer.key(),
            ErrorCode::InvalidDestinationTokenAccount
        );
        let destination_token_program = destination_token_program.as_ref().unwrap();

        // If destination token sa is some, transfer WSOL to destination token account
        if destination_token_sa.is_some() {
            transfer_token(
                sa_authority.to_account_info(),
                destination_token_sa.as_ref().unwrap().to_account_info(),
                destination_token_account.to_account_info(),
                destination_mint.to_account_info(),
                destination_token_program.to_account_info(),
                amount_out,
                destination_mint.decimals,
                Some(SA_AUTHORITY_SEED),
            )?;
        }

        // Unwrap wsol to sa_authority
        close_token_account(
            destination_token_account.to_account_info(),
            sa_authority.to_account_info(),
            payer.to_account_info(),
            destination_token_program.to_account_info(),
            None,
        )?;
        Ok(())
    }

    /// Transfer from fees and log results
    fn transfer_from_fees_and_log<'info>(
        &self,
        sa_authority: &Option<UncheckedAccount<'info>>,
        source_token_sa: &Option<UncheckedAccount<'info>>,
        source_mint: &InterfaceAccount<'info, Mint>,
        source_token_program: &Option<Interface<'info, TokenInterface>>,
        commission_amount: u64,
        platform_fee_amount: u64,
        commission_account: &Option<AccountInfo<'info>>,
        platform_fee_account: &Option<AccountInfo<'info>>,
        is_charge_fee: bool,
        is_charge_sol: bool,
    ) -> Result<()> {
        if !is_charge_fee {
            return Ok(());
        }
        if commission_amount > 0 {
            require!(commission_account.is_some(), ErrorCode::CommissionAccountIsNone);
        }
        if platform_fee_amount > 0 {
            require!(platform_fee_account.is_some(), ErrorCode::PlatformFeeAccountIsNone);
        }
        require!(sa_authority.is_some(), ErrorCode::SaAuthorityIsNone);
        let sa_authority = sa_authority.as_ref().unwrap();

        if is_charge_sol {
            // Transfer SOL commission
            if commission_amount > 0 {
                let commission_account = commission_account.as_ref().unwrap();
                let adjust_amount = transfer_sol_fee(
                    sa_authority,
                    commission_account,
                    commission_amount,
                    Some(SA_AUTHORITY_SEED),
                )?;
                log_commission_info(true, commission_amount, adjust_amount);
                commission_account.key().log();
            }

            // Transfer SOL platform fee
            if platform_fee_amount > 0 {
                let platform_fee_account = platform_fee_account.as_ref().unwrap();
                let adjust_amount = transfer_sol_fee(
                    sa_authority,
                    platform_fee_account,
                    platform_fee_amount,
                    Some(SA_AUTHORITY_SEED),
                )?;
                log_platform_fee_info(
                    platform_fee_amount,
                    adjust_amount,
                    &platform_fee_account.key(),
                );
            }
        } else {
            require!(source_token_sa.is_some(), ErrorCode::SourceTokenSaIsNone);
            require!(source_token_program.is_some(), ErrorCode::SourceTokenProgramIsNone);
            let source_token_sa = source_token_sa.as_ref().unwrap();
            let source_token_program = source_token_program.as_ref().unwrap();

            // Transfer token commission
            if commission_amount > 0 {
                let commission_account = commission_account.as_ref().unwrap();
                transfer_token_fee(
                    sa_authority,
                    source_token_sa,
                    source_mint,
                    source_token_program,
                    commission_account,
                    commission_amount,
                    Some(SA_AUTHORITY_SEED),
                )?;
                log_commission_info(true, commission_amount, 0);
                commission_account.key().log();
            }

            // Transfer token platform fee
            if platform_fee_amount > 0 {
                let platform_fee_account = platform_fee_account.as_ref().unwrap();
                transfer_token_fee(
                    sa_authority,
                    source_token_sa,
                    source_mint,
                    source_token_program,
                    platform_fee_account,
                    platform_fee_amount,
                    Some(SA_AUTHORITY_SEED),
                )?;
                log_platform_fee_info(platform_fee_amount, 0, &platform_fee_account.key());
            }
        }

        Ok(())
    }

    /// Transfer to fees and log results
    fn transfer_to_fees_and_log<'info>(
        &self,
        payer: &AccountInfo<'info>,
        sa_authority: &UncheckedAccount<'info>,
        destination_token_account: &mut InterfaceAccount<'info, TokenAccount>,
        destination_mint: &InterfaceAccount<'info, Mint>,
        destination_token_sa: &Option<UncheckedAccount<'info>>,
        destination_token_program: &Option<Interface<'info, TokenInterface>>,
        amount_out: u64,
        commission_amount: u64,
        platform_fee_amount: u64,
        commission_account: &Option<AccountInfo<'info>>,
        platform_fee_account: &Option<AccountInfo<'info>>,
        acc_close_flag: bool,
    ) -> Result<bool> {
        if commission_amount == 0 && platform_fee_amount == 0 {
            return Ok(false);
        }
        if commission_amount > 0 {
            require!(commission_account.is_some(), ErrorCode::CommissionAccountIsNone);
            if acc_close_flag {
                require!(
                    is_system_account(commission_account.as_ref().unwrap()),
                    ErrorCode::InvalidCommissionAccount
                );
            }
        }
        if platform_fee_amount > 0 {
            require!(platform_fee_account.is_some(), ErrorCode::PlatformFeeAccountIsNone);
            if acc_close_flag {
                require!(
                    is_system_account(platform_fee_account.as_ref().unwrap()),
                    ErrorCode::InvalidPlatformFeeAccount
                );
            }
        }

        if is_charge_sol(commission_account, platform_fee_account, destination_mint) {
            // Unwrap wsol to sa_authority
            self.unwrap_wsol_to_sa(
                payer,
                sa_authority,
                destination_token_account,
                destination_mint,
                destination_token_sa,
                destination_token_program,
                amount_out,
            )?;

            // Transfer SOL fees
            if commission_amount > 0 {
                let commission_account = commission_account.as_ref().unwrap();
                let adjust_amount = transfer_sol_fee(
                    sa_authority,
                    commission_account,
                    commission_amount,
                    Some(SA_AUTHORITY_SEED),
                )?;
                log_commission_info(false, commission_amount, adjust_amount);
                commission_account.key().log();
            }

            if platform_fee_amount > 0 {
                let platform_fee_account = platform_fee_account.as_ref().unwrap();
                let adjust_amount = transfer_sol_fee(
                    sa_authority,
                    platform_fee_account,
                    platform_fee_amount,
                    Some(SA_AUTHORITY_SEED),
                )?;
                log_platform_fee_info(
                    platform_fee_amount,
                    adjust_amount,
                    &platform_fee_account.key(),
                );
            }

            return Ok(true);
        } else {
            require!(destination_token_sa.is_some(), ErrorCode::DestinationTokenSaIsNone);
            require!(destination_token_program.is_some(), ErrorCode::DestinationTokenProgramIsNone);
            let destination_token_sa = destination_token_sa.as_ref().unwrap();
            let destination_token_program = destination_token_program.as_ref().unwrap();

            // Transfer token fees
            if commission_amount > 0 {
                let commission_account = commission_account.as_ref().unwrap();
                transfer_token_fee(
                    sa_authority,
                    destination_token_sa,
                    destination_mint,
                    destination_token_program,
                    commission_account,
                    commission_amount,
                    Some(SA_AUTHORITY_SEED),
                )?;
                log_commission_info(false, commission_amount, 0);
                commission_account.key().log();
            }

            if platform_fee_amount > 0 {
                let platform_fee_account = platform_fee_account.as_ref().unwrap();
                transfer_token_fee(
                    sa_authority,
                    destination_token_sa,
                    destination_mint,
                    destination_token_program,
                    platform_fee_account,
                    platform_fee_amount,
                    Some(SA_AUTHORITY_SEED),
                )?;
                log_platform_fee_info(platform_fee_amount, 0, &platform_fee_account.key());
            }

            return Ok(false);
        }
    }

    /// Transfer trim and log results
    fn transfer_trim_and_log<'info>(
        &self,
        payer: &AccountInfo<'info>,
        sa_authority: &UncheckedAccount<'info>,
        destination_token_account: &mut InterfaceAccount<'info, TokenAccount>,
        destination_mint: &InterfaceAccount<'info, Mint>,
        destination_token_sa: &Option<UncheckedAccount<'info>>,
        destination_token_program: &Option<Interface<'info, TokenInterface>>,
        amount_out: u64,
        trim_amount: u64,
        charge_amount: u64,
        trim_account: Option<&AccountInfo<'info>>,
        charge_account: Option<&AccountInfo<'info>>,
        is_unwrap_wsol_to_sa: bool,
        acc_close_flag: bool,
    ) -> Result<bool> {
        if trim_amount == 0 && charge_amount == 0 {
            return Ok(is_unwrap_wsol_to_sa);
        }
        require!(trim_account.is_some(), ErrorCode::TrimAccountIsNone);
        let trim_account = trim_account.as_ref().unwrap();
        if acc_close_flag {
            require!(is_system_account(trim_account), ErrorCode::InvalidTrimAccount);
        }

        // Validate and extract charge_account if needed
        let charge_account = if charge_amount > 0 {
            require!(charge_account.is_some(), ErrorCode::ChargeAccountIsNone);
            let charge_acc = charge_account.as_ref().unwrap();
            if acc_close_flag || is_unwrap_wsol_to_sa {
                require!(is_system_account(charge_acc), ErrorCode::InvalidChargeAccount);
            }
            Some(charge_acc.to_account_info())
        } else {
            None
        };

        if is_charge_sol(&Some(trim_account.to_account_info()), &charge_account, destination_mint) {
            if !is_unwrap_wsol_to_sa {
                // Unwrap wsol to sa_authority
                self.unwrap_wsol_to_sa(
                    payer,
                    sa_authority,
                    destination_token_account,
                    destination_mint,
                    destination_token_sa,
                    destination_token_program,
                    amount_out,
                )?;
            }

            if trim_amount > 0 {
                // Transfer SOL trim fee
                let adjust_amount = transfer_sol_fee(
                    sa_authority,
                    trim_account,
                    trim_amount,
                    Some(SA_AUTHORITY_SEED),
                )?;
                log_trim_fee_info(trim_amount, adjust_amount, &trim_account.key());
            }

            if let Some(charge_acc) = charge_account {
                let adjust_amount = transfer_sol_fee(
                    sa_authority,
                    &charge_acc,
                    charge_amount,
                    Some(SA_AUTHORITY_SEED),
                )?;
                log_charge_fee_info(charge_amount, adjust_amount, &charge_acc.key());
            }
            return Ok(true);
        } else {
            require!(destination_token_sa.is_some(), ErrorCode::DestinationTokenSaIsNone);
            require!(destination_token_program.is_some(), ErrorCode::DestinationTokenProgramIsNone);
            let destination_token_sa = destination_token_sa.as_ref().unwrap();
            let destination_token_program = destination_token_program.as_ref().unwrap();

            if trim_amount > 0 {
                transfer_token_fee(
                    sa_authority,
                    destination_token_sa,
                    destination_mint,
                    destination_token_program,
                    trim_account,
                    trim_amount,
                    Some(SA_AUTHORITY_SEED),
                )?;
                log_trim_fee_info(trim_amount, 0, &trim_account.key());
            }

            if let Some(charge_acc) = charge_account {
                transfer_token_fee(
                    sa_authority,
                    destination_token_sa,
                    destination_mint,
                    destination_token_program,
                    &charge_acc,
                    charge_amount,
                    Some(SA_AUTHORITY_SEED),
                )?;
                log_charge_fee_info(charge_amount, 0, &charge_acc.key());
            }
            return Ok(false);
        }
    }

    /// Transfer remaining token or SOL to user
    fn transfer_to_user<'info>(
        &self,
        payer: &AccountInfo<'info>,
        sa_authority: &UncheckedAccount<'info>,
        destination_token_account: &mut InterfaceAccount<'info, TokenAccount>,
        destination_mint: &InterfaceAccount<'info, Mint>,
        destination_token_sa: &Option<UncheckedAccount<'info>>,
        destination_token_program: &Option<Interface<'info, TokenInterface>>,
        actual_amount_out: u64,
        is_unwrap_wsol_to_sa: bool,
        acc_close_flag: bool,
    ) -> Result<()> {
        if is_unwrap_wsol_to_sa {
            // Transfer remaining SOL & token account rent to payer
            transfer_sol(
                sa_authority.to_account_info(),
                payer.to_account_info(),
                actual_amount_out
                    .checked_add(TOKEN_ACCOUNT_RENT)
                    .ok_or(ErrorCode::CalculationError)?,
                Some(SA_AUTHORITY_SEED),
            )?;
        } else {
            require!(destination_token_sa.is_some(), ErrorCode::DestinationTokenSaIsNone);
            require!(destination_token_program.is_some(), ErrorCode::DestinationTokenProgramIsNone);
            let destination_token_sa = destination_token_sa.as_ref().unwrap();
            let destination_token_program = destination_token_program.as_ref().unwrap();

            // Transfer remaining tokens to destination account
            transfer_token(
                sa_authority.to_account_info(),
                destination_token_sa.to_account_info(),
                destination_token_account.to_account_info(),
                destination_mint.to_account_info(),
                destination_token_program.to_account_info(),
                actual_amount_out,
                destination_mint.decimals,
                Some(SA_AUTHORITY_SEED),
            )?;

            if acc_close_flag
                && destination_token_account.mint == wsol_program::ID
                && is_token_account_initialized(&destination_token_account.to_account_info())
            {
                close_token_account(
                    destination_token_account.to_account_info(),
                    payer.to_account_info(),
                    payer.to_account_info(),
                    destination_token_program.to_account_info(),
                    None,
                )?
            }
        }
        Ok(())
    }
}

impl<'info> PlatformFeeV3Processor<'info> for SwapToBProcessor {
    fn get_swap_accounts(
        &self,
        payer: &AccountInfo<'info>,
        source_token_account: &mut InterfaceAccount<'info, TokenAccount>,
        destination_token_account: &mut InterfaceAccount<'info, TokenAccount>,
        source_mint: &InterfaceAccount<'info, Mint>,
        destination_mint: &InterfaceAccount<'info, Mint>,
        sa_authority: &Option<UncheckedAccount<'info>>,
        source_token_sa: &mut Option<UncheckedAccount<'info>>,
        destination_token_sa: &mut Option<UncheckedAccount<'info>>,
        source_token_program: &Option<Interface<'info, TokenInterface>>,
        destination_token_program: &Option<Interface<'info, TokenInterface>>,
        associated_token_program: &Option<Program<'info, AssociatedToken>>,
        system_program: &Option<Program<'info, System>>,
    ) -> Result<(InterfaceAccount<'info, TokenAccount>, InterfaceAccount<'info, TokenAccount>)>
    {
        ProxySwapProcessor.get_swap_accounts(
            payer,
            source_token_account,
            destination_token_account,
            source_mint,
            destination_mint,
            sa_authority,
            source_token_sa,
            destination_token_sa,
            source_token_program,
            destination_token_program,
            associated_token_program,
            system_program,
        )
    }

    fn before_swap(
        &self,
        payer: &AccountInfo<'info>,
        sa_authority: &Option<UncheckedAccount<'info>>,
        source_token_account: &mut InterfaceAccount<'info, TokenAccount>,
        source_mint: &InterfaceAccount<'info, Mint>,
        source_token_sa: &mut Option<UncheckedAccount<'info>>,
        source_token_program: &Option<Interface<'info, TokenInterface>>,
        amount_in: u64,
        commission_rate: u32,
        commission_direction: bool,
        commission_account: &Option<AccountInfo<'info>>,
        platform_fee_rate: Option<u16>,
        platform_fee_account: &Option<AccountInfo<'info>>,
    ) -> Result<u64> {
        // Check if commission is SOL
        let is_charge_sol = is_charge_sol(commission_account, platform_fee_account, source_mint);

        // Calculate fees if commission is applied to from
        let (commission_amount, platform_fee_amount, actual_amount_in, is_charge_fee) = self
            .calculate_from_fees(
                amount_in,
                commission_rate,
                commission_direction,
                platform_fee_rate,
            )?;

        // Proxy handle before swap
        if is_charge_sol {
            ProxySwapProcessor.proxy_handle_before(
                payer,
                source_token_account,
                source_token_sa,
                source_mint,
                source_token_program,
                amount_in,
                None,
            )?;
            if is_charge_fee {
                require!(sa_authority.is_some(), ErrorCode::SaAuthorityIsNone);
                let sa_authority = sa_authority.as_ref().unwrap();
                require!(sa_authority.key() == authority_pda::ID, ErrorCode::InvalidSaAuthority);
                let total_fee = commission_amount
                    .checked_add(platform_fee_amount)
                    .ok_or(ErrorCode::CalculationError)?;

                // Transfer SOL fees to sa_authority
                transfer_sol(
                    payer.to_account_info(),
                    sa_authority.to_account_info(),
                    total_fee,
                    None,
                )?;
            }
        } else {
            ProxySwapProcessor.proxy_handle_before(
                payer,
                source_token_account,
                source_token_sa,
                source_mint,
                source_token_program,
                actual_amount_in,
                None,
            )?;
        }

        // Transfer from fees and log results
        self.transfer_from_fees_and_log(
            sa_authority,
            source_token_sa,
            source_mint,
            source_token_program,
            commission_amount,
            platform_fee_amount,
            commission_account,
            platform_fee_account,
            is_charge_fee,
            is_charge_sol,
        )?;

        Ok(amount_in)
    }

    fn after_swap(
        &self,
        payer: &AccountInfo<'info>,
        sa_authority: &Option<UncheckedAccount<'info>>,
        destination_token_account: &mut InterfaceAccount<'info, TokenAccount>,
        destination_mint: &InterfaceAccount<'info, Mint>,
        destination_token_sa: &mut Option<UncheckedAccount<'info>>,
        destination_token_program: &Option<Interface<'info, TokenInterface>>,
        expected_amount_out: u64,
        amount_out: u64,
        commission_rate: u32,
        commission_direction: bool,
        commission_account: &Option<AccountInfo<'info>>,
        platform_fee_rate: Option<u16>,
        platform_fee_account: &Option<AccountInfo<'info>>,
        trim_rate: Option<u8>,
        charge_rate: Option<u16>,
        trim_account: Option<&AccountInfo<'info>>,
        charge_account: Option<&AccountInfo<'info>>,
        acc_close_flag: bool,
    ) -> Result<u64> {
        // Calculate fees and actual amount out if commission is applied to to
        let (
            commission_amount,
            platform_fee_amount,
            trim_amount,
            charge_amount,
            actual_amount_out,
            is_charge_fee,
        ) = self.calculate_to_fees(
            amount_out,
            expected_amount_out,
            commission_rate,
            commission_direction,
            platform_fee_rate,
            trim_rate,
            charge_rate,
        )?;

        // Proxy handle after swap
        if !is_charge_fee {
            ProxySwapProcessor.proxy_handle_after(
                sa_authority,
                destination_token_account,
                destination_mint,
                destination_token_sa,
                destination_token_program,
                amount_out,
                Some(SA_AUTHORITY_SEED),
            )?;

            if acc_close_flag
                && destination_token_account.mint == wsol_program::ID
                && is_token_account_initialized(&destination_token_account.to_account_info())
            {
                require!(
                    destination_token_program.is_some(),
                    ErrorCode::DestinationTokenProgramIsNone
                );
                close_token_account(
                    destination_token_account.to_account_info(),
                    payer.to_account_info(),
                    payer.to_account_info(),
                    destination_token_program.as_ref().unwrap().to_account_info(),
                    None,
                )?
            }
        } else {
            require!(sa_authority.is_some(), ErrorCode::SaAuthorityIsNone);
            let sa_authority = sa_authority.as_ref().unwrap();

            // Transfer to fees and log results
            let mut is_unwrap_wsol_to_sa = self.transfer_to_fees_and_log(
                payer,
                sa_authority,
                destination_token_account,
                destination_mint,
                destination_token_sa,
                destination_token_program,
                amount_out,
                commission_amount,
                platform_fee_amount,
                commission_account,
                platform_fee_account,
                acc_close_flag,
            )?;

            is_unwrap_wsol_to_sa = self.transfer_trim_and_log(
                payer,
                sa_authority,
                destination_token_account,
                destination_mint,
                destination_token_sa,
                destination_token_program,
                amount_out,
                trim_amount,
                charge_amount,
                trim_account,
                charge_account,
                is_unwrap_wsol_to_sa,
                acc_close_flag,
            )?;

            self.transfer_to_user(
                payer,
                sa_authority,
                destination_token_account,
                destination_mint,
                destination_token_sa,
                destination_token_program,
                actual_amount_out,
                is_unwrap_wsol_to_sa,
                acc_close_flag,
            )?;
        }

        Ok(actual_amount_out)
    }
}
