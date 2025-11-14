use super::{
    CommissionSOLProxySwapAccounts, CommissionSPLProxySwapAccounts, CommonCommissionProcessorV2,
    common_commission_sol_swap_v2, common_commission_token_swap_v2,
};
use crate::SwapArgs;
use crate::constants::*;
use crate::error::ErrorCode;
use crate::processor::proxy_swap_processor::ProxySwapProcessor;
use crate::utils::*;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

pub struct PlatformFeeProcessorV2;
impl<'info> CommonCommissionProcessorV2<'info> for PlatformFeeProcessorV2 {
    fn commission_sol_process_v2(
        &self,
        amount_in: u64,
        amount_out: u64,
        expected_amount_out: u64,
        commission_rate: u32,
        commission_direction: bool,
        payer: &AccountInfo<'info>,
        commission_account: &AccountInfo<'info>,
        trim_account: Option<&AccountInfo<'info>>,
        source_mint: &InterfaceAccount<'info, Mint>,
        destination_mint: &InterfaceAccount<'info, Mint>,
        destination_token_account: &mut InterfaceAccount<'info, TokenAccount>,
        destination_token_program: &Option<Interface<'info, TokenInterface>>,
        source_token_sa: &mut Option<UncheckedAccount<'info>>,
        destination_token_sa: &mut Option<UncheckedAccount<'info>>,
        platform_fee_rate: Option<u32>,
        trim_rate: Option<u8>,
    ) -> Result<()> {
        if platform_fee_rate.is_some() && platform_fee_rate.unwrap() > 0 {
            require!(
                source_token_sa.is_some() || destination_token_sa.is_some(),
                ErrorCode::MissingSaAccount
            );
            require!(commission_rate > 0, ErrorCode::InvalidCommissionRate);
        }

        require!(commission_rate <= COMMISSION_RATE_LIMIT_V2, ErrorCode::InvalidCommissionRate);
        require!(
            platform_fee_rate.is_some()
                && platform_fee_rate.unwrap() as u64 <= PLATFORM_FEE_RATE_LIMIT_V2,
            ErrorCode::InvalidPlatformFeeRate
        );

        let commission_amount = if commission_direction {
            // Commission direction: true-fromToken
            require!(
                source_mint.key() == wsol_program::ID,
                ErrorCode::InvalidCommissionTokenAccount
            );
            u64::try_from(
                u128::from(amount_in)
                    .checked_mul(commission_rate as u128)
                    .ok_or(ErrorCode::CalculationError)?
                    .checked_div(COMMISSION_DENOMINATOR_V2 as u128 - commission_rate as u128)
                    .ok_or(ErrorCode::CalculationError)?,
            )
            .unwrap()
        } else {
            // Commission direction: false-toToken
            require!(
                destination_mint.key() == wsol_program::ID,
                ErrorCode::InvalidCommissionTokenAccount
            );

            // Close temp wsol account for unwrap sol
            if destination_token_program.is_some() {
                close_token_account(
                    destination_token_account.to_account_info(),
                    payer.to_account_info(),
                    payer.to_account_info(),
                    destination_token_program.as_ref().unwrap().to_account_info(),
                    None,
                )?;
            }

            u64::try_from(
                u128::from(amount_out)
                    .checked_mul(commission_rate as u128)
                    .ok_or(ErrorCode::CalculationError)?
                    .checked_div(COMMISSION_DENOMINATOR_V2 as u128)
                    .ok_or(ErrorCode::CalculationError)?,
            )
            .unwrap()
        };

        let mut platform_fee_amount: u64 = 0;

        if platform_fee_rate.is_some() && platform_fee_rate.unwrap() > 0 {
            // Platform fee for fromToken
            platform_fee_amount = u64::try_from(
                u128::from(commission_amount)
                    .checked_mul(platform_fee_rate.unwrap() as u128)
                    .ok_or(ErrorCode::CalculationError)?
                    .checked_div(PLATFORM_FEE_DENOMINATOR_V2 as u128)
                    .ok_or(ErrorCode::CalculationError)?,
            )
            .unwrap();

            let sa_account = if commission_direction {
                source_token_sa.clone()
            } else {
                destination_token_sa.clone()
            };

            let sa_account_key = if sa_account.as_ref().unwrap().owner == &crate::token_program::ID
            {
                let sa_account_box =
                    Box::leak(Box::new(sa_account.as_ref().unwrap().to_account_info()));
                InterfaceAccount::<TokenAccount>::try_from(sa_account_box).unwrap().owner
            } else {
                sa_account.as_ref().unwrap().key()
            };

            // Transfer platform_fee_amount
            transfer_sol(
                payer.to_account_info(),
                sa_account.unwrap().to_account_info(),
                platform_fee_amount,
                None,
            )?;
            log_platform_fee_info(platform_fee_amount, 0, &sa_account_key);
        }

        // Transfer commission_amount
        transfer_sol(
            payer.to_account_info(),
            commission_account.to_account_info(),
            commission_amount.checked_sub(platform_fee_amount).unwrap(),
            None,
        )?;

        log_commission_info(
            commission_direction,
            commission_amount.checked_sub(platform_fee_amount).unwrap(),
            0,
        );
        commission_account.key().log();

        // Trim destionation token
        if trim_account.is_some()
            && trim_rate.is_some()
            && trim_account.unwrap().key() != crate::ID
            && trim_rate.unwrap() > 0
        {
            require!(trim_rate.unwrap() <= TRIM_RATE_LIMIT_V2, ErrorCode::InvalidTrimRate);
            let trim_limit = u64::try_from(
                u128::from(amount_out)
                    .saturating_mul(trim_rate.unwrap() as u128)
                    .saturating_div(TRIM_DENOMINATOR_V2 as u128),
            )
            .unwrap();

            let trim_amount = if commission_direction {
                (amount_out.saturating_sub(expected_amount_out)).min(trim_limit)
            } else {
                (amount_out.saturating_sub(commission_amount).saturating_sub(expected_amount_out))
                    .min(trim_limit)
            };
            // Transfer trim_amount
            if trim_amount > 0 {
                if destination_mint.key() == wsol_program::ID {
                    transfer_sol(
                        payer.to_account_info(),
                        trim_account.unwrap().to_account_info(),
                        trim_amount,
                        None,
                    )?;
                } else {
                    require!(
                        destination_token_program.is_some(),
                        ErrorCode::DestinationTokenProgramIsNone
                    );
                    transfer_token(
                        payer.to_account_info(),
                        destination_token_account.to_account_info(),
                        trim_account.unwrap().to_account_info(),
                        destination_mint.to_account_info(),
                        destination_token_program.as_ref().unwrap().to_account_info(),
                        trim_amount,
                        destination_mint.decimals,
                        None,
                    )?;
                }
                msg!("trim_amount: {:?}", trim_amount);
                trim_account.unwrap().key().log();
            }
        }
        Ok(())
    }

    fn commission_token_process_v2(
        &self,
        amount_in: u64,
        amount_out: u64,
        expected_amount_out: u64,
        commission_rate: u32,
        commission_direction: bool,
        payer: &AccountInfo<'info>,
        commission_token_account: &InterfaceAccount<'info, TokenAccount>,
        trim_token_account: Option<&AccountInfo<'info>>,
        source_token_account: &InterfaceAccount<'info, TokenAccount>,
        destination_token_account: &InterfaceAccount<'info, TokenAccount>,
        source_mint: &InterfaceAccount<'info, Mint>,
        destination_mint: &InterfaceAccount<'info, Mint>,
        commission_token_program: AccountInfo<'info>,
        trim_token_program: Option<AccountInfo<'info>>,
        source_token_sa: &mut Option<UncheckedAccount<'info>>,
        destination_token_sa: &mut Option<UncheckedAccount<'info>>,
        platform_fee_rate: Option<u32>,
        trim_rate: Option<u8>,
    ) -> Result<()> {
        if platform_fee_rate.is_some() && platform_fee_rate.unwrap() > 0 {
            require!(
                source_token_sa.is_some() || destination_token_sa.is_some(),
                ErrorCode::MissingSaAccount
            );
            require!(commission_rate > 0, ErrorCode::InvalidCommissionRate);
        }
        require!(
            platform_fee_rate.is_some()
                && platform_fee_rate.unwrap() as u64 <= PLATFORM_FEE_RATE_LIMIT_V2,
            ErrorCode::InvalidPlatformFeeRate
        );
        require!(commission_rate <= COMMISSION_RATE_LIMIT_V2, ErrorCode::InvalidCommissionRate);
        let (commission_amount, platform_fee_amount) = if commission_direction {
            // Commission direction: true-fromToken
            require!(
                commission_token_account.mint == source_mint.key(),
                ErrorCode::InvalidCommissionTokenAccount
            );
            let commission_amount = u64::try_from(
                u128::from(amount_in)
                    .checked_mul(commission_rate as u128)
                    .ok_or(ErrorCode::CalculationError)?
                    .checked_div(COMMISSION_DENOMINATOR_V2 as u128 - commission_rate as u128)
                    .ok_or(ErrorCode::CalculationError)?,
            )
            .unwrap();

            let platform_fee_amount = u64::try_from(
                u128::from(commission_amount)
                    .checked_mul(platform_fee_rate.unwrap() as u128)
                    .ok_or(ErrorCode::CalculationError)?
                    .checked_div(PLATFORM_FEE_DENOMINATOR_V2 as u128)
                    .ok_or(ErrorCode::CalculationError)?,
            )
            .unwrap();

            if platform_fee_amount > 0 {
                transfer_token(
                    payer.to_account_info(),
                    source_token_account.to_account_info(),
                    source_token_sa.as_ref().unwrap().to_account_info(),
                    source_mint.to_account_info(),
                    commission_token_program.clone(),
                    platform_fee_amount,
                    source_mint.decimals,
                    None,
                )?;
                let sa_account_key = if source_token_sa.as_ref().unwrap().owner
                    == &crate::token_program::ID
                {
                    let sa_account_box =
                        Box::leak(Box::new(source_token_sa.as_ref().unwrap().to_account_info()));
                    InterfaceAccount::<TokenAccount>::try_from(sa_account_box).unwrap().owner
                } else {
                    source_token_sa.as_ref().unwrap().key()
                };
                log_platform_fee_info(platform_fee_amount, 0, &sa_account_key);
            }

            transfer_token(
                payer.to_account_info(),
                source_token_account.to_account_info(),
                commission_token_account.to_account_info(),
                source_mint.to_account_info(),
                commission_token_program,
                commission_amount.checked_sub(platform_fee_amount).unwrap(),
                source_mint.decimals,
                None,
            )?;
            (commission_amount, platform_fee_amount)
        } else {
            // Commission direction: false-toToken
            require!(
                commission_token_account.mint == destination_mint.key(),
                ErrorCode::InvalidCommissionTokenAccount
            );
            let commission_amount = u64::try_from(
                u128::from(amount_out)
                    .checked_mul(commission_rate as u128)
                    .ok_or(ErrorCode::CalculationError)?
                    .checked_div(COMMISSION_DENOMINATOR_V2 as u128)
                    .ok_or(ErrorCode::CalculationError)?,
            )
            .unwrap();

            let platform_fee_amount = u64::try_from(
                u128::from(commission_amount)
                    .checked_mul(platform_fee_rate.unwrap() as u128)
                    .ok_or(ErrorCode::CalculationError)?
                    .checked_div(PLATFORM_FEE_DENOMINATOR_V2 as u128)
                    .ok_or(ErrorCode::CalculationError)?,
            )
            .unwrap();

            if platform_fee_amount > 0 {
                transfer_token(
                    payer.to_account_info(),
                    destination_token_account.to_account_info(),
                    destination_token_sa.as_ref().unwrap().to_account_info(),
                    destination_mint.to_account_info(),
                    commission_token_program.clone(),
                    platform_fee_amount,
                    destination_mint.decimals,
                    None,
                )?;
                let sa_account_key =
                    if destination_token_sa.as_ref().unwrap().owner == &crate::token_program::ID {
                        let sa_account_box = Box::leak(Box::new(
                            destination_token_sa.as_ref().unwrap().to_account_info(),
                        ));
                        InterfaceAccount::<TokenAccount>::try_from(sa_account_box).unwrap().owner
                    } else {
                        destination_token_sa.as_ref().unwrap().key()
                    };
                log_platform_fee_info(platform_fee_amount, 0, &sa_account_key);
            }

            transfer_token(
                payer.to_account_info(),
                destination_token_account.to_account_info(),
                commission_token_account.to_account_info(),
                destination_mint.to_account_info(),
                commission_token_program,
                commission_amount.checked_sub(platform_fee_amount).unwrap(),
                destination_mint.decimals,
                None,
            )?;

            (commission_amount, platform_fee_amount)
        };

        log_commission_info(
            commission_direction,
            commission_amount.checked_sub(platform_fee_amount).unwrap(),
            0,
        );
        commission_token_account.key().log();

        // Trim token
        if trim_token_account.is_some()
            && trim_rate.is_some()
            && trim_token_account.unwrap().key() != crate::ID
            && trim_rate.unwrap() > 0
        {
            require!(trim_rate.unwrap() <= TRIM_RATE_LIMIT_V2, ErrorCode::InvalidTrimRate);
            let trim_limit = u64::try_from(
                u128::from(amount_out)
                    .saturating_mul(trim_rate.unwrap() as u128)
                    .saturating_div(TRIM_DENOMINATOR_V2 as u128),
            )
            .unwrap();

            let trim_amount = if commission_direction {
                (amount_out.saturating_sub(expected_amount_out)).min(trim_limit)
            } else {
                (amount_out.saturating_sub(commission_amount).saturating_sub(expected_amount_out))
                    .min(trim_limit)
            };

            if trim_amount > 0 {
                transfer_token(
                    payer.to_account_info(),
                    destination_token_account.to_account_info(),
                    trim_token_account.unwrap().to_account_info(),
                    destination_mint.to_account_info(),
                    trim_token_program.unwrap(),
                    trim_amount,
                    destination_mint.decimals,
                    None,
                )?;
                msg!("trim_amount: {:?}", trim_amount);
                trim_token_account.unwrap().to_account_info().key().log();
            }
        }
        Ok(())
    }
}

pub fn platform_fee_sol_proxy_swap_handler_v2<'a>(
    ctx: Context<'_, '_, 'a, 'a, CommissionSOLProxySwapAccounts<'a>>,
    args: SwapArgs,
    commission_info: u32,
    order_id: u64,
    platform_fee_rate: u32,
    trim_rate: u8,
) -> Result<()> {
    let swap_processor = &ProxySwapProcessor;
    let platform_fee_processor = &PlatformFeeProcessorV2;

    let commission_direction = commission_info >> 31 == 1;
    let commission_rate = commission_info & ((1 << 30) - 1);

    log_rate_info(commission_rate, platform_fee_rate, Some(trim_rate));

    let trim_account = if trim_rate > 0 {
        Some(&ctx.remaining_accounts[ctx.remaining_accounts.len() - 1])
    } else {
        None
    };

    common_commission_sol_swap_v2(
        swap_processor,
        platform_fee_processor,
        &ctx.accounts.payer,
        &ctx.accounts.payer,
        None,
        &mut ctx.accounts.source_token_account,
        &mut ctx.accounts.destination_token_account,
        &ctx.accounts.source_mint,
        &ctx.accounts.destination_mint,
        &ctx.accounts.sa_authority,
        &mut ctx.accounts.source_token_sa,
        &mut ctx.accounts.destination_token_sa,
        &ctx.accounts.source_token_program,
        &ctx.accounts.destination_token_program,
        &ctx.accounts.associated_token_program,
        &ctx.accounts.system_program,
        ctx.remaining_accounts,
        args,
        order_id,
        commission_rate,
        commission_direction,
        &ctx.accounts.commission_account,
        trim_account,
        Some(platform_fee_rate), // platform_fee_rate
        Some(trim_rate),         // trim_rate
    )?;
    Ok(())
}

pub fn platform_fee_spl_proxy_swap_handler_v2<'a>(
    ctx: Context<'_, '_, 'a, 'a, CommissionSPLProxySwapAccounts<'a>>,
    args: SwapArgs,
    commission_info: u32,
    order_id: u64,
    platform_fee_rate: u32,
    trim_rate: u8,
) -> Result<()> {
    let commission_direction = commission_info >> 31 == 1;
    let commission_rate = commission_info & ((1 << 30) - 1);

    log_rate_info(commission_rate, platform_fee_rate, Some(trim_rate));

    let (trim_token_account, trim_token_program) = if trim_rate > 0 {
        (
            Some(&ctx.remaining_accounts[ctx.remaining_accounts.len() - 2]),
            Some(&ctx.remaining_accounts[ctx.remaining_accounts.len() - 1]),
        )
    } else {
        (None, None)
    };

    let commission_token_program = if commission_direction {
        ctx.accounts.source_token_program.as_ref().unwrap().to_account_info()
    } else {
        ctx.accounts.destination_token_program.as_ref().unwrap().to_account_info()
    };
    let swap_processor = &ProxySwapProcessor;
    let commission_processor = &PlatformFeeProcessorV2;

    common_commission_token_swap_v2(
        swap_processor,
        commission_processor,
        &ctx.accounts.payer,
        &ctx.accounts.payer,
        None,
        &mut ctx.accounts.source_token_account,
        &mut ctx.accounts.destination_token_account,
        &ctx.accounts.source_mint,
        &ctx.accounts.destination_mint,
        &ctx.accounts.sa_authority,
        &mut ctx.accounts.source_token_sa,
        &mut ctx.accounts.destination_token_sa,
        &ctx.accounts.source_token_program,
        &ctx.accounts.destination_token_program,
        &ctx.accounts.associated_token_program,
        &ctx.accounts.system_program,
        ctx.remaining_accounts,
        args,
        order_id,
        commission_rate,
        commission_direction,
        &ctx.accounts.commission_token_account,
        trim_token_account,
        commission_token_program,
        trim_token_program.map(|info| info.to_account_info()),
        Some(platform_fee_rate),
        Some(trim_rate),
    )?;
    Ok(())
}
