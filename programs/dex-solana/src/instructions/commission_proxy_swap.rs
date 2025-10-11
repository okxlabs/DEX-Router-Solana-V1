use crate::constants::*;
use crate::processor::proxy_swap_processor::ProxySwapProcessor;
use crate::{
    CommissionProcessor, SwapArgs, common_commission_sol_swap, common_commission_token_swap,
};
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct CommissionSOLProxySwapAccounts<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        token::mint = source_mint,
        token::authority = payer,
    )]
    pub source_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = destination_mint,
    )]
    pub destination_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    pub source_mint: Box<InterfaceAccount<'info, Mint>>,

    pub destination_mint: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: commission account
    #[account(mut)]
    pub commission_account: AccountInfo<'info>,

    /// CHECK: sa_authority
    #[account(
        seeds = [
            SEED_SA,
        ],
        bump = BUMP_SA,
    )]
    pub sa_authority: Option<UncheckedAccount<'info>>,

    #[account(mut)]
    pub source_token_sa: Option<UncheckedAccount<'info>>,

    #[account(mut)]
    pub destination_token_sa: Option<UncheckedAccount<'info>>,

    pub source_token_program: Option<Interface<'info, TokenInterface>>,
    pub destination_token_program: Option<Interface<'info, TokenInterface>>,
    pub associated_token_program: Option<Program<'info, AssociatedToken>>,
    pub system_program: Option<Program<'info, System>>,
}

pub fn commission_sol_proxy_swap_handler<'a>(
    ctx: Context<'_, '_, 'a, 'a, CommissionSOLProxySwapAccounts<'a>>,
    args: SwapArgs,
    commission_rate: u16,
    commission_direction: bool,
    order_id: u64,
) -> Result<()> {
    let swap_processor = &ProxySwapProcessor;
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
        None, // trim_account
        None, // platform_fee_rate
    )?;
    Ok(())
}

#[derive(Accounts)]
pub struct CommissionSPLProxySwapAccounts<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        token::mint = source_mint,
        token::authority = payer,
    )]
    pub source_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = destination_mint,
    )]
    pub destination_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    pub source_mint: Box<InterfaceAccount<'info, Mint>>,

    pub destination_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut)]
    pub commission_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: sa_authority
    #[account(
        seeds = [
            SEED_SA,
        ],
        bump = BUMP_SA,
    )]
    pub sa_authority: Option<UncheckedAccount<'info>>,

    #[account(mut)]
    pub source_token_sa: Option<UncheckedAccount<'info>>,

    #[account(mut)]
    pub destination_token_sa: Option<UncheckedAccount<'info>>,

    pub source_token_program: Option<Interface<'info, TokenInterface>>,
    pub destination_token_program: Option<Interface<'info, TokenInterface>>,
    pub associated_token_program: Option<Program<'info, AssociatedToken>>,
    pub system_program: Option<Program<'info, System>>,
}

pub fn commission_spl_proxy_swap_handler<'a>(
    ctx: Context<'_, '_, 'a, 'a, CommissionSPLProxySwapAccounts<'a>>,
    args: SwapArgs,
    commission_rate: u16,
    commission_direction: bool,
    order_id: u64,
) -> Result<()> {
    let commission_token_program = if commission_direction {
        ctx.accounts.source_token_program.as_ref().unwrap().to_account_info()
    } else {
        ctx.accounts.destination_token_program.as_ref().unwrap().to_account_info()
    };
    let swap_processor = &ProxySwapProcessor;
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
        None, // trim_token_account
        commission_token_program,
        None, // trim_token_program
        None, // platform_fee_rate
    )?;
    Ok(())
}
