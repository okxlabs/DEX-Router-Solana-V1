use crate::constants::*;
use crate::processor::proxy_swap_processor::ProxySwapProcessor;
use crate::{SwapArgs, common_swap};
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct ProxySwapAccounts<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        token::mint = source_mint,
        token::authority = payer,
        token::token_program = source_token_program,
    )]
    pub source_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = destination_mint,
        token::token_program = destination_token_program,
    )]
    pub destination_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    pub source_mint: Box<InterfaceAccount<'info, Mint>>,
    pub destination_mint: Box<InterfaceAccount<'info, Mint>>,

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

pub fn proxy_swap_handler<'a>(
    ctx: Context<'_, '_, 'a, 'a, ProxySwapAccounts<'a>>,
    args: SwapArgs,
    order_id: u64,
) -> Result<()> {
    common_swap(
        &ProxySwapProcessor,
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
        None,
        None,
        None,
    )?;
    Ok(())
}
