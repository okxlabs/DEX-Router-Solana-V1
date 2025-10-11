use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{HopAccounts, SWAP_SELECTOR, lifinity_v2pool_program};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token::Token;
use anchor_spl::token_interface::{Mint, TokenAccount};
use arrayref::array_ref;

use super::common::DexProcessor;

const ARGS_LEN: usize = 24;

pub struct LifinityProcessor;
impl DexProcessor for LifinityProcessor {}

pub struct LifinitySwapAccountsV2<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub authority: &'info AccountInfo<'info>,
    pub amm_info: &'info AccountInfo<'info>,
    pub swap_source: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination: InterfaceAccount<'info, TokenAccount>,
    pub pool_mint: InterfaceAccount<'info, Mint>,
    pub fee_account: &'info AccountInfo<'info>,
    pub oracle_main_account: &'info AccountInfo<'info>,
    pub oracle_sub_account: &'info AccountInfo<'info>,
    pub oracle_pc_account: &'info AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}
const ACCOUNTS_LEN: usize = 14;

impl<'info> LifinitySwapAccountsV2<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            authority,
            amm_info,
            swap_source,
            swap_destination,
            pool_mint,
            fee_account,
            oracle_main_account,
            oracle_sub_account,
            oracle_pc_account,
            token_program,
        ]: &[AccountInfo<'info>; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];
        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            authority,
            amm_info,
            swap_source: InterfaceAccount::try_from(swap_source)?,
            swap_destination: InterfaceAccount::try_from(swap_destination)?,
            pool_mint: InterfaceAccount::try_from(pool_mint)?,
            fee_account,
            oracle_main_account,
            oracle_sub_account,
            oracle_pc_account,
            token_program: Program::try_from(token_program)?,
        })
    }
}

pub fn swap_v1<'a>(
    _remaining_accounts: &'a [AccountInfo<'a>],
    _amount_in: u64,
    _offset: &mut usize,
    _hop_accounts: &mut HopAccounts,
    _hop: usize,
    _proxy_swap: bool,
    _owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    require!(true == false, ErrorCode::AdapterAbort);
    Ok(0)
}

pub fn swap_v2<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::LifinitySwapV2 amount_in: {}, offset: {}", amount_in, offset);
    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);
    let mut swap_accounts = LifinitySwapAccountsV2::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &lifinity_v2pool_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.amm_info.key().log();

    // check hop accounts & swap authority
    let swap_source_token = swap_accounts.swap_source_token.key();
    let swap_destination_token = swap_accounts.swap_destination_token.key();
    before_check(
        &swap_accounts.swap_authority_pubkey,
        &swap_accounts.swap_source_token,
        swap_destination_token,
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(SWAP_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&1u64.to_le_bytes());

    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.authority.key(), false),
        AccountMeta::new(swap_accounts.amm_info.key(), false),
        AccountMeta::new_readonly(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new(swap_source_token, false),
        AccountMeta::new(swap_destination_token, false),
        AccountMeta::new(swap_accounts.swap_source.key(), false),
        AccountMeta::new(swap_accounts.swap_destination.key(), false),
        AccountMeta::new(swap_accounts.pool_mint.key(), false),
        AccountMeta::new(swap_accounts.fee_account.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.oracle_main_account.key(), false),
        AccountMeta::new_readonly(swap_accounts.oracle_sub_account.key(), false),
        AccountMeta::new_readonly(swap_accounts.oracle_pc_account.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.authority.to_account_info(),
        swap_accounts.amm_info.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.swap_source.to_account_info(),
        swap_accounts.swap_destination.to_account_info(),
        swap_accounts.pool_mint.to_account_info(),
        swap_accounts.fee_account.to_account_info(),
        swap_accounts.token_program.to_account_info(),
        swap_accounts.oracle_main_account.to_account_info(),
        swap_accounts.oracle_sub_account.to_account_info(),
        swap_accounts.oracle_pc_account.to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &LifinityProcessor;
    let amount_out = invoke_process(
        amount_in,
        dex_processor,
        &account_infos,
        &mut swap_accounts.swap_source_token,
        &mut swap_accounts.swap_destination_token,
        hop_accounts,
        instruction,
        hop,
        offset,
        ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )?;
    Ok(amount_out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_pack_swap_instruction() {
        let amount_in = 100u64;
        let mut data = Vec::with_capacity(ARGS_LEN);
        data.extend_from_slice(SWAP_SELECTOR);
        data.extend_from_slice(&amount_in.to_le_bytes());
        data.extend_from_slice(&1u64.to_le_bytes());

        msg!("data.len: {}", data.len());
        assert!(data.len() == ARGS_LEN);
    }
}
