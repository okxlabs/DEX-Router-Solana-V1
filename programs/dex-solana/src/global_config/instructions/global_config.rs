use crate::constants::*;
use crate::program::DexSolana;
use crate::state::{config::*, event::*};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct InitGlobalConfig<'info> {
    /// Address to be set as protocol owner.
    #[account(mut)]
    pub admin: Signer<'info>,

    /// Initialize config state account to store protocol owner address and fee rates.
    #[account(
        init,
        seeds = [
            GLOBAL_CONFIG_SEED.as_bytes(),
        ],
        bump,
        payer = admin,
        space = GlobalConfig::LEN
    )]
    pub global_config: AccountLoader<'info, GlobalConfig>,

    #[account(constraint = program.programdata_address()? == Some(program_data.key()))]
    pub program: Program<'info, DexSolana>,

    #[account(constraint = program_data.upgrade_authority_address == Some(admin.key()))]
    pub program_data: Account<'info, ProgramData>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateGlobalConfig<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [
            GLOBAL_CONFIG_SEED.as_bytes(),
        ],
        has_one = admin,
        bump = global_config.load()?.bump,
    )]
    pub global_config: AccountLoader<'info, GlobalConfig>,
}

pub fn init_global_config_handler(ctx: Context<InitGlobalConfig>, trade_fee: u64) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config.load_init()?;
    let admin = *ctx.accounts.admin.key;
    global_config.bump = ctx.bumps.global_config;
    global_config.admin = admin;
    global_config.fee_multiplier = 10;
    global_config.padding = [0u8; 127];
    global_config.set_trade_fee(trade_fee)?;
    emit!(InitGlobalConfigEvent { admin, trade_fee });
    Ok(())
}

pub fn set_admin_handler(ctx: Context<UpdateGlobalConfig>, admin: Pubkey) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config.load_mut()?;
    global_config.set_admin(admin)?;
    emit!(SetAdminEvent { admin });
    Ok(())
}

pub fn add_resolver_handler(ctx: Context<UpdateGlobalConfig>, resolver: Pubkey) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config.load_mut()?;
    global_config.add_resolver(resolver)?;
    emit!(AddResolverEvent { resolver });
    Ok(())
}

pub fn remove_resolver_handler(ctx: Context<UpdateGlobalConfig>, resolver: Pubkey) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config.load_mut()?;
    global_config.remove_resolver(resolver)?;
    emit!(RemoveResolverEvent { resolver });
    Ok(())
}

pub fn set_trade_fee_handler(ctx: Context<UpdateGlobalConfig>, trade_fee: u64) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config.load_mut()?;
    global_config.set_trade_fee(trade_fee)?;
    emit!(SetTradeFeeEvent { trade_fee });
    Ok(())
}

pub fn pause_trading_handler(ctx: Context<UpdateGlobalConfig>) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config.load_mut()?;
    global_config.set_paused(true)?;
    emit!(PauseTradingEvent { paused: true });
    Ok(())
}

pub fn unpause_trading_handler(ctx: Context<UpdateGlobalConfig>) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config.load_mut()?;
    global_config.set_paused(false)?;
    emit!(PauseTradingEvent { paused: false });
    Ok(())
}

pub fn set_fee_multiplier_handler(
    ctx: Context<UpdateGlobalConfig>,
    fee_multiplier: u8,
) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config.load_mut()?;
    global_config.set_fee_multiplier(fee_multiplier)?;
    emit!(SetFeeMultiplierEvent { fee_multiplier });
    Ok(())
}
