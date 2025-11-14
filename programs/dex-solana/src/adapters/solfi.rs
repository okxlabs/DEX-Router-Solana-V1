use crate::error::ErrorCode;
use crate::HopAccounts;
use anchor_lang::prelude::*;

pub fn swap<'a>(
    _remaining_accounts: &'a [AccountInfo<'a>],
    _amount_in: u64,
    _offset: &mut usize,
    _hop_accounts: &mut HopAccounts,
    _hop: usize,
    _proxy_swap: bool,
    _owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::Solfi ABORT");
    require!(true == false, ErrorCode::AdapterAbort);
    Ok(0)
}

pub fn swap_v2<'a>(
    _remaining_accounts: &'a [AccountInfo<'a>],
    _amount_in: u64,
    _offset: &mut usize,
    _hop_accounts: &mut HopAccounts,
    _hop: usize,
    _proxy_swap: bool,
    _owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::SolfiV2 ABORT");
    require!(true == false, ErrorCode::AdapterAbort);
    Ok(0)
}
