use crate::SwapArgs;
use crate::common_swap;
use crate::processor::swap_processor::SwapProcessor;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount};

#[derive(Accounts)]
pub struct SwapAccounts<'info> {
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
}

pub fn swap_handler<'a>(
    ctx: Context<'_, '_, 'a, 'a, SwapAccounts<'a>>,
    args: SwapArgs,
    order_id: u64,
) -> Result<()> {
    common_swap(
        &SwapProcessor,
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
        order_id,
        None,
        None,
        None,
    )?;
    Ok(())
}
