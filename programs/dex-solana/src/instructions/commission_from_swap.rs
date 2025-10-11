use crate::instructions::from_swap::cpi_bridge_to_log;
use crate::processor::swap_processor::SwapProcessor;
use crate::{
    BridgeToArgs, CommissionProcessor, SwapArgs, common_commission_sol_swap,
    common_commission_token_swap,
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::Token,
    token_2022::Token2022,
    token_interface::{Mint, TokenAccount},
};

#[derive(Accounts)]
pub struct CommissionSOLFromSwapAccounts<'info> {
    #[account(mut)]
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

    #[account(mut)]
    pub destination_mint: InterfaceAccount<'info, Mint>,

    /// CHECK: bridge_program
    #[account(address = crate::okx_bridge_program::id())]
    pub bridge_program: AccountInfo<'info>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub token_2022_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,

    #[account(mut)]
    pub commission_account: SystemAccount<'info>,
}

pub fn commission_sol_from_swap_handler<'a>(
    ctx: Context<'_, '_, 'a, 'a, CommissionSOLFromSwapAccounts<'a>>,
    args: SwapArgs,
    commission_rate: u16,
    bridge_to_args: BridgeToArgs,
    offset: u8,
    len: u8,
) -> Result<()> {
    // 1. Commission swap
    let swap_processor = &SwapProcessor;
    let commission_processor = &CommissionProcessor;
    let amount_out = common_commission_sol_swap(
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
        args,
        bridge_to_args.order_id,
        commission_rate,
        true,
        &ctx.accounts.commission_account,
        None, // trim_account
        None, // platform_fee_rate
    )?;

    // 3. CPI bridge_to_log
    cpi_bridge_to_log(
        bridge_to_args,
        amount_out,
        offset,
        len,
        &ctx.accounts.bridge_program,
        &ctx.accounts.payer,
        &ctx.accounts.destination_token_account,
        &ctx.accounts.destination_mint,
        &ctx.accounts.associated_token_program,
        &ctx.accounts.token_program,
        &ctx.accounts.token_2022_program,
        &ctx.accounts.system_program,
        &ctx.remaining_accounts,
    )?;
    Ok(())
}

#[derive(Accounts)]
pub struct CommissionSPLFromSwapAccounts<'info> {
    #[account(mut)]
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

    #[account(mut)]
    pub destination_mint: InterfaceAccount<'info, Mint>,

    /// CHECK: bridge_program
    #[account(address = crate::okx_bridge_program::id())]
    pub bridge_program: AccountInfo<'info>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub token_2022_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,

    #[account(
        mut,
        token::token_program = token_program,
    )]
    pub commission_token_account: InterfaceAccount<'info, TokenAccount>,
}

pub fn commission_spl_from_swap_handler<'a>(
    ctx: Context<'_, '_, 'a, 'a, CommissionSPLFromSwapAccounts<'a>>,
    args: SwapArgs,
    commission_rate: u16,
    bridge_to_args: BridgeToArgs,
    offset: u8,
    len: u8,
) -> Result<()> {
    let commission_token_program =
        if *ctx.accounts.source_mint.to_account_info().owner == Token2022::id() {
            ctx.accounts.token_2022_program.to_account_info()
        } else {
            ctx.accounts.token_program.to_account_info()
        };

    // 1. Commission swap
    let swap_processor = &SwapProcessor;
    let commission_processor = &CommissionProcessor;
    let amount_out = common_commission_token_swap(
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
        args,
        bridge_to_args.order_id,
        commission_rate,
        true,
        &ctx.accounts.commission_token_account,
        None, // trim_token_account
        commission_token_program,
        None, // trim_token_program
        None, // platform_fee_rate
    )?;

    // 2. CPI bridge_to_log
    cpi_bridge_to_log(
        bridge_to_args,
        amount_out,
        offset,
        len,
        &ctx.accounts.bridge_program,
        &ctx.accounts.payer,
        &ctx.accounts.destination_token_account,
        &ctx.accounts.destination_mint,
        &ctx.accounts.associated_token_program,
        &ctx.accounts.token_program,
        &ctx.accounts.token_2022_program,
        &ctx.accounts.system_program,
        &ctx.remaining_accounts,
    )?;
    Ok(())
}
