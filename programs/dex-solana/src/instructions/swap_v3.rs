use super::{common_swap_v3, SwapArgs};
use crate::processor::*;
use crate::utils::*;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

#[derive(Accounts)]
pub struct CommissionProxySwapAccountsV3<'info> {
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
    pub commission_account: Option<AccountInfo<'info>>,

    /// CHECK: platform fee account
    #[account(mut)]
    pub platform_fee_account: Option<AccountInfo<'info>>,

    /// CHECK: sa_authority
    #[account(mut)]
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

pub fn swap_tob_handler<'a>(
    ctx: Context<'_, '_, 'a, 'a, CommissionProxySwapAccountsV3<'a>>,
    args: SwapArgs,
    commission_info: u32,
    order_id: u64,
    trim_rate: Option<u8>,
    platform_fee_rate: Option<u16>,
) -> Result<()> {
    let commission_direction = commission_info >> 31 == 1;
    let acc_close_flag = ((commission_info & (1 << 30)) >> 30) == 1;
    let commission_rate = commission_info & ((1 << 30) - 1);
    log_rate_info_v3(
        commission_rate,
        platform_fee_rate,
        trim_rate,
        commission_direction,
        acc_close_flag,
    );

    let trim_account = if trim_rate.is_some() && trim_rate.unwrap() > 0 {
        Some(&ctx.remaining_accounts[ctx.remaining_accounts.len() - 1])
    } else {
        None
    };
    common_swap_v3(
        &SwapToBProcessor,
        &ctx.accounts.payer,
        &mut ctx.accounts.source_token_account,
        &mut ctx.accounts.destination_token_account,
        &ctx.accounts.source_mint,
        &ctx.accounts.destination_mint,
        &mut ctx.accounts.sa_authority,
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
        platform_fee_rate,
        &ctx.accounts.platform_fee_account,
        trim_rate,
        trim_account,
        acc_close_flag,
    )?;
    Ok(())
}

pub fn swap_toc_handler<'a>(
    ctx: Context<'_, '_, 'a, 'a, CommissionProxySwapAccountsV3<'a>>,
    args: SwapArgs,
    commission_info: u32,
    order_id: u64,
    platform_fee_rate: Option<u16>,
) -> Result<()> {
    let commission_direction = commission_info >> 31 == 1;
    let commission_rate = commission_info & ((1 << 30) - 1);
    log_rate_info_v3(
        commission_rate,
        platform_fee_rate,
        None,
        commission_direction,
        false,
    );

    common_swap_v3(
        &SwapToCProcessor,
        &ctx.accounts.payer,
        &mut ctx.accounts.source_token_account,
        &mut ctx.accounts.destination_token_account,
        &ctx.accounts.source_mint,
        &ctx.accounts.destination_mint,
        &mut ctx.accounts.sa_authority,
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
        platform_fee_rate,
        &ctx.accounts.platform_fee_account,
        None,
        None,
        false,
    )?;
    Ok(())
}
