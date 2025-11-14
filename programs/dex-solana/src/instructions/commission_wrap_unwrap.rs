use crate::error::ErrorCode;
use crate::utils::logging::{
    log_commission_info, log_swap_balance_before, log_swap_basic_info, log_swap_end,
};
use crate::utils::token::{close_token_account, sync_wsol_account, transfer_sol, transfer_token};
use crate::{
    COMMISSION_DENOMINATOR, COMMISSION_RATE_LIMIT, SEED_TEMP_WSOL, system_program, wsol_program,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(AnchorDeserialize, AnchorSerialize, Clone)]
pub struct CommissionWrapUnwrapArgs {
    pub amount_in: u64,
    pub wrap_direction: bool, // Wrap direction: true-wrap, false-unwrap
    pub commission_rate: u16, // Commission rate
    pub commission_direction: bool, // Commission direction: true-fromToken, false-toToken
}

#[derive(Accounts)]
pub struct CommissionWrapUnwrapAccounts<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        token::mint = wsol_mint,
        token::authority = payer,
        token::token_program = token_program,
    )]
    pub payer_wsol_account: InterfaceAccount<'info, TokenAccount>,

    #[account(address = wsol_program::id())]
    pub wsol_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init_if_needed,
        payer = payer,
        token::mint = wsol_mint,
        token::authority = payer,
        token::token_program = token_program,
        seeds = [SEED_TEMP_WSOL, payer.key().as_ref()],
        bump
    )]
    pub temp_wsol_account: Option<Box<InterfaceAccount<'info, TokenAccount>>>,

    /// CHECK: system account or pda
    #[account(mut)]
    pub commission_sol_account: UncheckedAccount<'info>,

    #[account(
        mut,
        token::mint = wsol_mint,
        token::token_program = token_program,
    )]
    pub commission_wsol_account: InterfaceAccount<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
}

pub fn commission_wrap_unwrap_handler<'a>(
    ctx: Context<'_, '_, 'a, 'a, CommissionWrapUnwrapAccounts<'a>>,
    args: CommissionWrapUnwrapArgs,
    order_id: u64,
) -> Result<()> {
    // CHECK: CommissionSwapArgs
    require!(args.commission_rate <= COMMISSION_RATE_LIMIT, ErrorCode::InvalidCommissionRate);

    require!(
        ctx.accounts.wsol_mint.key() == wsol_program::id(),
        ErrorCode::InvalidCommissionTokenAccount
    );

    let (before_source_balance, before_destination_balance) = log_wrap_unwrap_initial_info(
        &ctx.accounts.wsol_mint,
        &ctx.accounts.payer,
        &ctx.accounts.payer_wsol_account,
        args.wrap_direction,
        args.amount_in,
        order_id,
    )?;

    let amount_out = args.amount_in;

    let commission_amount: u64;
    if args.commission_direction {
        // Commission direction: true-fromToken
        // Commission for fromToken
        commission_amount = args
            .amount_in
            .checked_mul(args.commission_rate as u64)
            .ok_or(ErrorCode::CalculationError)?
            .checked_div(COMMISSION_DENOMINATOR - args.commission_rate as u64)
            .ok_or(ErrorCode::CalculationError)?;
    } else {
        // Commission direction: false-toToken
        // Commission for toToken
        commission_amount = amount_out
            .checked_mul(args.commission_rate as u64)
            .ok_or(ErrorCode::CalculationError)?
            .checked_div(COMMISSION_DENOMINATOR)
            .ok_or(ErrorCode::CalculationError)?;
    }

    if args.wrap_direction {
        wrap_process(
            ctx.accounts.payer.clone(),
            ctx.accounts.payer_wsol_account.clone(),
            args.amount_in,
            ctx.accounts.token_program.clone(),
        )?;
    } else {
        if let Some(ref temp_wsol_account) = ctx.accounts.temp_wsol_account {
            unwrap_process(
                ctx.accounts.payer.clone(),
                ctx.accounts.wsol_mint.clone(),
                ctx.accounts.payer_wsol_account.clone(),
                (**temp_wsol_account).clone(),
                args.amount_in,
                ctx.accounts.token_program.clone(),
            )?;
        } else {
            return err!(ErrorCode::InvalidCommissionTemporaryTokenAccount);
        }
    }

    log_wrap_unwrap_final_info(
        &ctx.accounts.payer,
        &mut ctx.accounts.payer_wsol_account,
        args.wrap_direction,
        before_source_balance,
        before_destination_balance,
    )?;

    if (args.commission_direction && args.wrap_direction)
        || (!args.commission_direction && !args.wrap_direction)
    {
        transfer_sol(
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.commission_sol_account.to_account_info(),
            commission_amount,
            None,
        )?;
    } else {
        transfer_token(
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.payer_wsol_account.to_account_info(),
            ctx.accounts.commission_wsol_account.to_account_info(),
            ctx.accounts.wsol_mint.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            commission_amount,
            ctx.accounts.wsol_mint.decimals,
            None,
        )?;
    }

    log_commission_info(args.commission_direction, commission_amount, 0);

    Ok(())
}

pub fn wrap_process<'info>(
    payer: Signer<'info>,
    wsol_account: InterfaceAccount<'info, TokenAccount>,
    amount: u64,
    token_program: Interface<'info, TokenInterface>,
) -> Result<()> {
    transfer_sol(payer.to_account_info(), wsol_account.to_account_info(), amount, None)?;

    sync_wsol_account(wsol_account.to_account_info(), token_program.to_account_info(), None)?;

    Ok(())
}

pub fn unwrap_process<'info>(
    payer: Signer<'info>,
    wsol_mint: InterfaceAccount<'info, Mint>,
    wsol_account: InterfaceAccount<'info, TokenAccount>,
    temp_wsol_account: InterfaceAccount<'info, TokenAccount>,
    amount: u64,
    token_program: Interface<'info, TokenInterface>,
) -> Result<()> {
    transfer_token(
        payer.to_account_info(),
        wsol_account.to_account_info(),
        temp_wsol_account.to_account_info(),
        wsol_mint.to_account_info(),
        token_program.to_account_info(),
        amount,
        wsol_mint.decimals,
        None,
    )?;

    close_token_account(
        temp_wsol_account.to_account_info(),
        payer.to_account_info(),
        payer.to_account_info(),
        token_program.to_account_info(),
        None,
    )?;

    Ok(())
}

pub fn log_wrap_unwrap_initial_info<'info>(
    wsol_mint: &InterfaceAccount<'info, Mint>,
    payer: &Signer<'info>,
    payer_wsol_account: &InterfaceAccount<'info, TokenAccount>,
    wrap_direction: bool,
    amount_in: u64,
    order_id: u64,
) -> Result<(u64, u64)> {
    let wsol_mint_id = wsol_mint.key();
    let sol_mint_id = system_program::id();

    let (source_mint, destination_mint) = if wrap_direction {
        // Wrap: SOL -> WSOL
        (&sol_mint_id, &wsol_mint_id)
    } else {
        // Unwrap: WSOL -> SOL
        (&wsol_mint_id, &sol_mint_id)
    };

    log_swap_basic_info(
        order_id,
        source_mint,
        destination_mint,
        &payer.key(), // SOL address is wallet address
        &payer.key(), // destination token account owner address is wallet address
    );

    let sol_balance = payer.lamports();
    let wsol_balance = payer_wsol_account.amount;

    let (before_source_balance, before_destination_balance) = if wrap_direction {
        // Wrap: SOL -> WSOL
        (sol_balance, wsol_balance)
    } else {
        // Unwrap: WSOL -> SOL
        (wsol_balance, sol_balance)
    };

    log_swap_balance_before(
        before_source_balance,
        before_destination_balance,
        amount_in,
        amount_in, // for wrap/unwrap, expect_amount_out equals amount_in
        amount_in, // min_return also equals amount_in
    );

    Ok((before_source_balance, before_destination_balance))
}

pub fn log_wrap_unwrap_final_info<'info>(
    payer: &Signer<'info>,
    payer_wsol_account: &mut InterfaceAccount<'info, TokenAccount>,
    wrap_direction: bool,
    before_source_balance: u64,
    before_destination_balance: u64,
) -> Result<()> {
    let sol_balance_after = payer.lamports();

    payer_wsol_account.reload()?;
    let wsol_balance_after = payer_wsol_account.amount;

    let (after_source_balance, after_destination_balance) = if wrap_direction {
        // Wrap: SOL -> WSOL
        (sol_balance_after, wsol_balance_after)
    } else {
        // Unwrap: WSOL -> SOL
        (wsol_balance_after, sol_balance_after)
    };

    let source_token_change = before_source_balance
        .checked_sub(after_source_balance)
        .ok_or(ErrorCode::CalculationError)?;
    let destination_token_change = after_destination_balance
        .checked_sub(before_destination_balance)
        .ok_or(ErrorCode::CalculationError)?;

    log_swap_end(
        after_source_balance,
        after_destination_balance,
        source_token_change,
        destination_token_change,
    );

    Ok(())
}
