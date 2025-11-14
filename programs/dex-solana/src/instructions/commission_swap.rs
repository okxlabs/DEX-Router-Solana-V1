use crate::constants::*;
use crate::error::ErrorCode;
use crate::processor::swap_processor::SwapProcessor;
use crate::utils::token::{close_token_account, transfer_sol, transfer_token};
use crate::{
    COMMISSION_DENOMINATOR, COMMISSION_RATE_LIMIT, CommissionSwapArgs, CommonCommissionProcessor,
    SwapArgs, common_commission_sol_swap,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use super::common_commission_token_swap;
use crate::utils::logging::log_commission_info;

pub struct CommissionProcessor;
impl<'info> CommonCommissionProcessor<'info> for CommissionProcessor {
    fn commission_sol_process(
        &self,
        amount_in: u64,
        amount_out: u64,
        _expected_amount_out: u64,
        commission_rate: u16,
        commission_direction: bool,
        payer: &AccountInfo<'info>,
        commission_account: &AccountInfo<'info>,
        _trim_account: Option<&AccountInfo<'info>>,
        source_mint: &InterfaceAccount<'info, Mint>,
        destination_mint: &InterfaceAccount<'info, Mint>,
        destination_token_account: &mut InterfaceAccount<'info, TokenAccount>,
        destination_token_program: &Option<Interface<'info, TokenInterface>>,
        _source_token_sa: &mut Option<UncheckedAccount<'info>>, // is not required
        _destination_token_sa: &mut Option<UncheckedAccount<'info>>, // is not required
        _platform_fee_rate: Option<u16>,                        // is not required
    ) -> Result<()> {
        require!(
            commission_rate > 0 && commission_rate <= COMMISSION_RATE_LIMIT,
            ErrorCode::InvalidCommissionRate
        );
        let commission_amount = if commission_direction {
            // Commission direction: true-fromToken
            require!(
                source_mint.key() == wsol_program::ID,
                ErrorCode::InvalidCommissionTokenAccount
            );
            amount_in
                .checked_mul(commission_rate as u64)
                .ok_or(ErrorCode::CalculationError)?
                .checked_div(COMMISSION_DENOMINATOR - commission_rate as u64)
                .ok_or(ErrorCode::CalculationError)?
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

            amount_out
                .checked_mul(commission_rate as u64)
                .ok_or(ErrorCode::CalculationError)?
                .checked_div(COMMISSION_DENOMINATOR)
                .ok_or(ErrorCode::CalculationError)?
        };

        // Transfer commission_amount
        transfer_sol(
            payer.to_account_info(),
            commission_account.to_account_info(),
            commission_amount,
            None,
        )?;
        log_commission_info(commission_direction, commission_amount, 0);
        Ok(())
    }

    fn commission_token_process(
        &self,
        amount_in: u64,
        amount_out: u64,
        _expected_amount_out: u64,
        commission_rate: u16,
        commission_direction: bool,
        payer: &AccountInfo<'info>,
        commission_token_account: &InterfaceAccount<'info, TokenAccount>,
        _trim_token_account: Option<&AccountInfo<'info>>,
        source_token_account: &InterfaceAccount<'info, TokenAccount>,
        destination_token_account: &InterfaceAccount<'info, TokenAccount>,
        source_mint: &InterfaceAccount<'info, Mint>,
        destination_mint: &InterfaceAccount<'info, Mint>,
        commission_token_program: AccountInfo<'info>,
        _trim_token_program: Option<AccountInfo<'info>>,
        _source_token_sa: &mut Option<UncheckedAccount<'info>>, // is not required
        _destination_token_sa: &mut Option<UncheckedAccount<'info>>, // is not required
        _platform_fee_rate: Option<u16>,
    ) -> Result<()> {
        require!(
            commission_rate > 0 && commission_rate <= COMMISSION_RATE_LIMIT,
            ErrorCode::InvalidCommissionRate
        );
        let commission_amount = if commission_direction {
            // Commission direction: true-fromToken
            require!(
                commission_token_account.mint == source_mint.key(),
                ErrorCode::InvalidCommissionTokenAccount
            );
            let commission_amount = amount_in
                .checked_mul(commission_rate as u64)
                .ok_or(ErrorCode::CalculationError)?
                .checked_div(COMMISSION_DENOMINATOR - commission_rate as u64)
                .ok_or(ErrorCode::CalculationError)?;

            transfer_token(
                payer.to_account_info(),
                source_token_account.to_account_info(),
                commission_token_account.to_account_info(),
                source_mint.to_account_info(),
                commission_token_program,
                commission_amount,
                source_mint.decimals,
                None,
            )?;
            commission_amount
        } else {
            // Commission direction: false-toToken
            require!(
                commission_token_account.mint == destination_mint.key(),
                ErrorCode::InvalidCommissionTokenAccount
            );
            let commission_amount = amount_out
                .checked_mul(commission_rate as u64)
                .ok_or(ErrorCode::CalculationError)?
                .checked_div(COMMISSION_DENOMINATOR)
                .ok_or(ErrorCode::CalculationError)?;

            transfer_token(
                payer.to_account_info(),
                destination_token_account.to_account_info(),
                commission_token_account.to_account_info(),
                destination_mint.to_account_info(),
                commission_token_program,
                commission_amount,
                destination_mint.decimals,
                None,
            )?;
            commission_amount
        };
        log_commission_info(commission_direction, commission_amount, 0);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CommissionSOLAccounts<'info> {
    pub payer: Signer<'info>,

    #[account(
        mut,
        token::mint = source_mint,
        token::authority = payer,
    )]
    pub source_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = destination_mint,
    )]
    pub destination_token_account: InterfaceAccount<'info, TokenAccount>,

    pub source_mint: InterfaceAccount<'info, Mint>,

    pub destination_mint: InterfaceAccount<'info, Mint>,

    /// CHECK: commission account
    #[account(mut)]
    pub commission_account: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

pub fn commission_sol_swap_handler<'a>(
    ctx: Context<'_, '_, 'a, 'a, CommissionSOLAccounts<'a>>,
    args: CommissionSwapArgs,
    order_id: u64,
) -> Result<()> {
    let swap_args = SwapArgs {
        amount_in: args.amount_in,
        expect_amount_out: args.expect_amount_out,
        min_return: args.min_return,
        amounts: args.amounts,
        routes: args.routes,
    };
    let swap_processor = &SwapProcessor;
    let commission_processor = &CommissionProcessor;

    common_commission_sol_swap(
        swap_processor,
        commission_processor,
        &ctx.accounts.payer,
        &ctx.accounts.payer,
        None,
        &mut ctx.accounts.source_token_account,
        &mut ctx.accounts.destination_token_account,
        &ctx.accounts.source_mint,
        &ctx.accounts.destination_mint,
        &None,
        &mut None,
        &mut None,
        &None,
        &None,
        &None,
        &None,
        ctx.remaining_accounts,
        swap_args,
        order_id,
        args.commission_rate,
        args.commission_direction,
        &ctx.accounts.commission_account,
        None, // trim_account
        None, // platform_fee_rate
    )?;
    Ok(())
}

#[derive(Accounts)]
pub struct CommissionSPLAccounts<'info> {
    pub payer: Signer<'info>,

    #[account(
        mut,
        token::mint = source_mint,
        token::authority = payer,
    )]
    pub source_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = destination_mint,
    )]
    pub destination_token_account: InterfaceAccount<'info, TokenAccount>,

    pub source_mint: InterfaceAccount<'info, Mint>,

    pub destination_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        token::token_program = token_program,
    )]
    pub commission_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn commission_spl_swap_handler<'a>(
    ctx: Context<'_, '_, 'a, 'a, CommissionSPLAccounts<'a>>,
    args: CommissionSwapArgs,
    order_id: u64,
) -> Result<()> {
    let swap_args = SwapArgs {
        amount_in: args.amount_in,
        expect_amount_out: args.expect_amount_out,
        min_return: args.min_return,
        amounts: args.amounts,
        routes: args.routes,
    };
    let swap_processor = &SwapProcessor;
    let commission_processor = &CommissionProcessor;

    common_commission_token_swap(
        swap_processor,
        commission_processor,
        &ctx.accounts.payer,
        &ctx.accounts.payer,
        None,
        &mut ctx.accounts.source_token_account,
        &mut ctx.accounts.destination_token_account,
        &ctx.accounts.source_mint,
        &ctx.accounts.destination_mint,
        &None,
        &mut None,
        &mut None,
        &None,
        &None,
        &None,
        &None,
        ctx.remaining_accounts,
        swap_args,
        order_id,
        args.commission_rate,
        args.commission_direction,
        &ctx.accounts.commission_token_account,
        None, // trim_token_account
        ctx.accounts.token_program.to_account_info(),
        None, // trim_token_program
        None, // platform_fee_rate
    )?;
    Ok(())
}
