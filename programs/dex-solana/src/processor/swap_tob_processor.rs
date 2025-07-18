use crate::constants::*;
use crate::error::ErrorCode;
use crate::processor::platform_fee_processor::PlatformFeeV3Processor;
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
    ) -> Result<(u64, u64, u64, u64, bool)> {
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
        let trim_amount = calculate_trim_amount(
            amount_out,
            expected_amount_out,
            commission_amount
                .checked_add(platform_fee_amount)
                .ok_or(ErrorCode::CalculationError)?,
            commission_direction,
            trim_rate,
        )?;

        // Calculate actual amount out
        actual_amount_out = actual_amount_out
            .checked_sub(commission_amount)
            .ok_or(ErrorCode::CalculationError)?
            .checked_sub(platform_fee_amount)
            .ok_or(ErrorCode::CalculationError)?
            .checked_sub(trim_amount)
            .ok_or(ErrorCode::CalculationError)?;

        Ok((
            commission_amount,
            platform_fee_amount,
            trim_amount,
            actual_amount_out,
            commission_amount > 0 || platform_fee_amount > 0 || trim_amount > 0,
        ))
    }

    /// Proxy handle before swap
    fn proxy_handle_before<'info>(
        &self,
        payer: &AccountInfo<'info>,
        source_token_account: &InterfaceAccount<'info, TokenAccount>,
        source_token_sa: &Option<UncheckedAccount<'info>>,
        source_mint: &InterfaceAccount<'info, Mint>,
        source_token_program: &Option<Interface<'info, TokenInterface>>,
        amount: u64,
    ) -> Result<()> {
        if source_token_sa.is_none() || source_token_program.is_none() {
            return Ok(());
        }
        let source_token_program = source_token_program.as_ref().unwrap();
        let source_token_sa =
            associate_convert_token_account(&source_token_sa.as_ref().unwrap().to_account_info())?;

        require!(
            source_token_sa.owner == authority_pda::ID,
            ErrorCode::InvalidSaAuthority
        );
        transfer_token(
            payer.to_account_info(),
            source_token_account.to_account_info(),
            source_token_sa.to_account_info(),
            source_mint.to_account_info(),
            source_token_program.to_account_info(),
            amount,
            source_mint.decimals,
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
            require!(
                commission_account.is_some(),
                ErrorCode::CommissionAccountIsNone
            );
        }
        if platform_fee_amount > 0 {
            require!(
                platform_fee_account.is_some(),
                ErrorCode::PlatformFeeAccountIsNone
            );
        }
        require!(sa_authority.is_some(), ErrorCode::SaAuthorityIsNone);
        let sa_authority = sa_authority.as_ref().unwrap();

        if is_charge_sol {
            // Transfer SOL commission
            if commission_amount > 0 {
                let commission_account = commission_account.as_ref().unwrap();
                transfer_sol_fee(
                    sa_authority,
                    commission_account,
                    commission_amount,
                    Some(SA_AUTHORITY_SEED),
                )?;
                log_commission_info(true, commission_amount);
                commission_account.key().log();
            }

            // Transfer SOL platform fee
            if platform_fee_amount > 0 {
                let platform_fee_account = platform_fee_account.as_ref().unwrap();
                transfer_sol_fee(
                    sa_authority,
                    platform_fee_account,
                    platform_fee_amount,
                    Some(SA_AUTHORITY_SEED),
                )?;
                log_platform_fee_info(platform_fee_amount, &platform_fee_account.key());
            }
        } else {
            require!(source_token_sa.is_some(), ErrorCode::SourceTokenSaIsNone);
            require!(
                source_token_program.is_some(),
                ErrorCode::SourceTokenProgramIsNone
            );
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
                log_commission_info(true, commission_amount);
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
                log_platform_fee_info(platform_fee_amount, &platform_fee_account.key());
            }
        }

        Ok(())
    }

    /// Transfer to fees and log results
    fn transfer_to_fees_and_log<'info>(
        &self,
        payer: &AccountInfo<'info>,
        sa_authority: &Option<UncheckedAccount<'info>>,
        destination_token_account: &mut InterfaceAccount<'info, TokenAccount>,
        destination_mint: &InterfaceAccount<'info, Mint>,
        destination_token_sa: &Option<UncheckedAccount<'info>>,
        destination_token_program: &Option<Interface<'info, TokenInterface>>,
        amount_out: u64,
        actual_amount_out: u64,
        commission_amount: u64,
        platform_fee_amount: u64,
        trim_amount: u64,
        commission_account: &Option<AccountInfo<'info>>,
        platform_fee_account: &Option<AccountInfo<'info>>,
        trim_account: Option<&AccountInfo<'info>>,
        is_charge_fee: bool,
    ) -> Result<()> {
        if commission_amount > 0 {
            require!(
                commission_account.is_some(),
                ErrorCode::CommissionAccountIsNone
            );
        }
        if platform_fee_amount > 0 {
            require!(
                platform_fee_account.is_some(),
                ErrorCode::PlatformFeeAccountIsNone
            );
        }
        if trim_amount > 0 {
            require!(trim_account.is_some(), ErrorCode::TrimAccountIsNone);
        }
        if is_charge_fee {
            require!(sa_authority.is_some(), ErrorCode::SaAuthorityIsNone);
        }

        if is_charge_sol(commission_account, platform_fee_account, destination_mint) {
            if sa_authority.is_some()
                && destination_token_sa.is_some()
                && destination_token_program.is_some()
            {
                let sa_authority = sa_authority.as_ref().unwrap();
                let destination_token_sa = destination_token_sa.as_ref().unwrap();
                let destination_token_program = destination_token_program.as_ref().unwrap();
                // Transfer WSOL to destination token account
                transfer_token(
                    sa_authority.to_account_info(),
                    destination_token_sa.to_account_info(),
                    destination_token_account.to_account_info(),
                    destination_mint.to_account_info(),
                    destination_token_program.to_account_info(),
                    amount_out,
                    destination_mint.decimals,
                    Some(SA_AUTHORITY_SEED),
                )?;
            }

            if is_charge_fee {
                // Close temp wsol token account
                require!(
                    destination_token_program.is_some(),
                    ErrorCode::DestinationTokenProgramIsNone
                );
                require!(
                    destination_token_account.owner == payer.key(),
                    ErrorCode::InvalidDestinationTokenAccount
                );
                let destination_token_program = destination_token_program.as_ref().unwrap();
                close_token_account(
                    destination_token_account.to_account_info(),
                    payer.to_account_info(),
                    payer.to_account_info(),
                    destination_token_program.to_account_info(),
                    None,
                )?;

                // Transfer fees
                if commission_amount > 0 {
                    let commission_account = commission_account.as_ref().unwrap();
                    transfer_sol_fee(
                        sa_authority.as_ref().unwrap(),
                        commission_account,
                        commission_amount,
                        Some(SA_AUTHORITY_SEED),
                    )?;
                    log_commission_info(false, commission_amount);
                    commission_account.key().log();
                }

                if platform_fee_amount > 0 {
                    let platform_fee_account = platform_fee_account.as_ref().unwrap();
                    transfer_sol_fee(
                        sa_authority.as_ref().unwrap(),
                        platform_fee_account,
                        platform_fee_amount,
                        Some(SA_AUTHORITY_SEED),
                    )?;
                    log_platform_fee_info(platform_fee_amount, &platform_fee_account.key());
                }

                if trim_amount > 0 {
                    let trim_account = trim_account.as_ref().unwrap();
                    transfer_sol_fee(
                        sa_authority.as_ref().unwrap(),
                        trim_account,
                        trim_amount,
                        Some(SA_AUTHORITY_SEED),
                    )?;
                    log_trim_fee_info(trim_amount, &trim_account.key());
                }

                // Transfer remaining SOL & token account rent to payer
                transfer_sol(
                    sa_authority.as_ref().unwrap().to_account_info(),
                    payer.to_account_info(),
                    actual_amount_out
                        .checked_add(TOKEN_ACCOUNT_RENT)
                        .ok_or(ErrorCode::CalculationError)?,
                    Some(SA_AUTHORITY_SEED),
                )?;
            }
        } else {
            require!(sa_authority.is_some(), ErrorCode::SaAuthorityIsNone);
            require!(
                destination_token_program.is_some(),
                ErrorCode::DestinationTokenProgramIsNone
            );
            require!(
                destination_token_sa.is_some(),
                ErrorCode::DestinationTokenSaIsNone
            );
            let sa_authority = sa_authority.as_ref().unwrap();
            let destination_token_program = destination_token_program.as_ref().unwrap();
            let destination_token_sa = destination_token_sa.as_ref().unwrap();

            // Regular tokens case - transfer fees first
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
                log_commission_info(false, commission_amount);
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
                log_platform_fee_info(platform_fee_amount, &platform_fee_account.key());
            }

            if trim_amount > 0 {
                let trim_account = trim_account.as_ref().unwrap();
                transfer_token_fee(
                    sa_authority,
                    destination_token_sa,
                    destination_mint,
                    destination_token_program,
                    trim_account,
                    trim_amount,
                    Some(SA_AUTHORITY_SEED),
                )?;
                log_trim_fee_info(trim_amount, &trim_account.key());
            }

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
    ) -> Result<(
        InterfaceAccount<'info, TokenAccount>,
        InterfaceAccount<'info, TokenAccount>,
    )> {
        let source_account = create_sa_if_needed(
            payer,
            source_mint,
            sa_authority,
            source_token_sa,
            source_token_program,
            associated_token_program,
            system_program,
        )?
        .unwrap_or_else(|| source_token_account.clone());

        let destination_account = create_sa_if_needed(
            payer,
            destination_mint,
            sa_authority,
            destination_token_sa,
            destination_token_program,
            associated_token_program,
            system_program,
        )?
        .unwrap_or_else(|| destination_token_account.clone());

        Ok((source_account, destination_account))
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
            self.proxy_handle_before(
                payer,
                source_token_account,
                source_token_sa,
                source_mint,
                source_token_program,
                amount_in,
            )?;
            if is_charge_fee {
                require!(sa_authority.is_some(), ErrorCode::SaAuthorityIsNone);
                let sa_authority = sa_authority.as_ref().unwrap();
                require!(
                    sa_authority.key() == authority_pda::ID,
                    ErrorCode::InvalidSaAuthority
                );
                let total_fee = commission_amount
                    .checked_add(platform_fee_amount)
                    .ok_or(ErrorCode::CalculationError)?;

                transfer_sol(
                    payer.to_account_info(),
                    sa_authority.to_account_info(),
                    total_fee,
                    None,
                )?;
            }
        } else {
            self.proxy_handle_before(
                payer,
                source_token_account,
                source_token_sa,
                source_mint,
                source_token_program,
                actual_amount_in,
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
        trim_account: Option<&AccountInfo<'info>>,
    ) -> Result<()> {
        // Calculate fees and actual amount out if commission is applied to to
        let (commission_amount, platform_fee_amount, trim_amount, actual_amount_out, is_charge_fee) =
            self.calculate_to_fees(
                amount_out,
                expected_amount_out,
                commission_rate,
                commission_direction,
                platform_fee_rate,
                trim_rate,
            )?;

        // Transfer to fees and log results
        self.transfer_to_fees_and_log(
            payer,
            sa_authority,
            destination_token_account,
            destination_mint,
            destination_token_sa,
            destination_token_program,
            amount_out,
            actual_amount_out,
            commission_amount,
            platform_fee_amount,
            trim_amount,
            commission_account,
            platform_fee_account,
            trim_account,
            is_charge_fee,
        )?;

        Ok(())
    }
}
