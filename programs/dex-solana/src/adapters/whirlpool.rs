use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{HopAccounts, SWAP_SELECTOR, SWAP_V2_SELECTOR, whirlpool_program};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token::Token;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use arrayref::array_ref;

use super::common::DexProcessor;

const ARGS_LEN: usize = 42;
const ARGS_V2_LEN: usize = 43;

pub struct WhirlpoolProcessor;
impl DexProcessor for WhirlpoolProcessor {}

pub struct WhirlpoolAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub whirlpool: &'info AccountInfo<'info>,
    pub token_vault_a: InterfaceAccount<'info, TokenAccount>,
    pub token_vault_b: InterfaceAccount<'info, TokenAccount>,
    pub tick_array0: &'info AccountInfo<'info>,
    pub tick_array1: &'info AccountInfo<'info>,
    pub tick_array2: &'info AccountInfo<'info>,
    pub oracle: &'info AccountInfo<'info>,
}
const ACCOUNTS_LEN: usize = 12;

pub struct WhirlpoolV2Accounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub token_program_a: Interface<'info, TokenInterface>,
    pub token_program_b: Interface<'info, TokenInterface>,
    pub memo_program: &'info AccountInfo<'info>,
    pub whirlpool: &'info AccountInfo<'info>,
    pub token_mint_a: InterfaceAccount<'info, Mint>,
    pub token_mint_b: InterfaceAccount<'info, Mint>,
    pub token_vault_a: InterfaceAccount<'info, TokenAccount>,
    pub token_vault_b: InterfaceAccount<'info, TokenAccount>,
    pub tick_array0: &'info AccountInfo<'info>,
    pub tick_array1: &'info AccountInfo<'info>,
    pub tick_array2: &'info AccountInfo<'info>,
    pub oracle: &'info AccountInfo<'info>,
}
const ACCOUNTS_V2_LEN: usize = 16;

impl<'info> WhirlpoolAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            token_program,
            whirlpool,
            token_vault_a,
            token_vault_b,
            tick_array0,
            tick_array1,
            tick_array2,
            oracle,
        ]: &[AccountInfo<'info>; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];
        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            token_program: Program::try_from(token_program)?,
            whirlpool,
            token_vault_a: InterfaceAccount::try_from(token_vault_a)?,
            token_vault_b: InterfaceAccount::try_from(token_vault_b)?,
            tick_array0,
            tick_array1,
            tick_array2,
            oracle,
        })
    }
}

impl<'info> WhirlpoolV2Accounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            token_program_a,
            token_program_b,
            memo_program,
            whirlpool,
            token_mint_a,
            token_mint_b,
            token_vault_a,
            token_vault_b,
            tick_array0,
            tick_array1,
            tick_array2,
            oracle,
        ]: &[AccountInfo<'info>; ACCOUNTS_V2_LEN] = array_ref![accounts, offset, ACCOUNTS_V2_LEN];
        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            token_program_a: Interface::try_from(token_program_a)?,
            token_program_b: Interface::try_from(token_program_b)?,
            memo_program,
            whirlpool,
            token_mint_a: InterfaceAccount::try_from(token_mint_a)?,
            token_mint_b: InterfaceAccount::try_from(token_mint_b)?,
            token_vault_a: InterfaceAccount::try_from(token_vault_a)?,
            token_vault_b: InterfaceAccount::try_from(token_vault_b)?,
            tick_array0,
            tick_array1,
            tick_array2,
            oracle,
        })
    }
}

pub fn swap<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::Whirlpool amount_in: {}, offset: {}", amount_in, offset);
    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);
    let mut swap_accounts = WhirlpoolAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &whirlpool_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.whirlpool.key().log();

    // check hop accounts & swap authority
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

    let amount_specified_is_input = true;
    let other_amount_threshold = 1u64;
    let a_to_b: bool;
    let sqrt_price_limit: i128;
    if swap_accounts.swap_source_token.mint == swap_accounts.token_vault_a.mint
        && swap_accounts.swap_destination_token.mint == swap_accounts.token_vault_b.mint
    {
        a_to_b = true;
        sqrt_price_limit = 4295048016; //The minimum sqrt-price supported by the Whirlpool program.
    } else if swap_accounts.swap_source_token.mint == swap_accounts.token_vault_b.mint
        && swap_accounts.swap_destination_token.mint == swap_accounts.token_vault_a.mint
    {
        a_to_b = false;
        sqrt_price_limit = 79226673515401279992447579055; //The maximum sqrt-price supported by the Whirlpool program.
    } else {
        return Err(ErrorCode::InvalidTokenMint.into());
    }
    let (token_owner_account_a, token_owner_account_b) = if a_to_b {
        (swap_accounts.swap_source_token.clone(), swap_accounts.swap_destination_token.clone())
    } else {
        (swap_accounts.swap_destination_token.clone(), swap_accounts.swap_source_token.clone())
    };

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(SWAP_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&other_amount_threshold.to_le_bytes());
    data.extend_from_slice(&sqrt_price_limit.to_le_bytes());
    data.extend_from_slice(&(amount_specified_is_input as u8).to_le_bytes());
    data.extend_from_slice(&(a_to_b as u8).to_le_bytes());

    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new(swap_accounts.whirlpool.key(), false),
        AccountMeta::new(token_owner_account_a.key(), false),
        AccountMeta::new(swap_accounts.token_vault_a.key(), false),
        AccountMeta::new(token_owner_account_b.key(), false),
        AccountMeta::new(swap_accounts.token_vault_b.key(), false),
        AccountMeta::new(swap_accounts.tick_array0.key(), false),
        AccountMeta::new(swap_accounts.tick_array1.key(), false),
        AccountMeta::new(swap_accounts.tick_array2.key(), false),
        AccountMeta::new_readonly(swap_accounts.oracle.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.token_program.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.whirlpool.to_account_info(),
        token_owner_account_a.to_account_info(),
        swap_accounts.token_vault_a.to_account_info(),
        token_owner_account_b.to_account_info(),
        swap_accounts.token_vault_b.to_account_info(),
        swap_accounts.tick_array0.to_account_info(),
        swap_accounts.tick_array1.to_account_info(),
        swap_accounts.tick_array2.to_account_info(),
        swap_accounts.oracle.to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &WhirlpoolProcessor;
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

pub fn swap_v2<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::WhirlpoolV2 amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + ACCOUNTS_V2_LEN,
        ErrorCode::InvalidAccountsLength
    );
    let mut swap_accounts = WhirlpoolV2Accounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &whirlpool_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.whirlpool.key().log();

    // check hop accounts & swap authority
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

    let amount_specified_is_input = true;
    let other_amount_threshold = 1u64;
    let a_to_b: bool;
    let sqrt_price_limit: i128;
    if swap_accounts.swap_source_token.mint == swap_accounts.token_vault_a.mint
        && swap_accounts.swap_destination_token.mint == swap_accounts.token_vault_b.mint
    {
        a_to_b = true;
        sqrt_price_limit = 4295048016; //The minimum sqrt-price supported by the Whirlpool program.
    } else if swap_accounts.swap_source_token.mint == swap_accounts.token_vault_b.mint
        && swap_accounts.swap_destination_token.mint == swap_accounts.token_vault_a.mint
    {
        a_to_b = false;
        sqrt_price_limit = 79226673515401279992447579055; //The maximum sqrt-price supported by the Whirlpool program.
    } else {
        return Err(ErrorCode::InvalidTokenMint.into());
    }
    let (token_owner_account_a, token_owner_account_b) = if a_to_b {
        (swap_accounts.swap_source_token.clone(), swap_accounts.swap_destination_token.clone())
    } else {
        (swap_accounts.swap_destination_token.clone(), swap_accounts.swap_source_token.clone())
    };

    let mut data = Vec::with_capacity(ARGS_V2_LEN);
    data.extend_from_slice(SWAP_V2_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&other_amount_threshold.to_le_bytes());
    data.extend_from_slice(&sqrt_price_limit.to_le_bytes());
    data.extend_from_slice(&(amount_specified_is_input as u8).to_le_bytes());
    data.extend_from_slice(&(a_to_b as u8).to_le_bytes());
    data.extend_from_slice(&(0u8).to_le_bytes());

    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.token_program_a.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program_b.key(), false),
        AccountMeta::new_readonly(swap_accounts.memo_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new(swap_accounts.whirlpool.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_mint_a.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_mint_b.key(), false),
        AccountMeta::new(token_owner_account_a.key(), false),
        AccountMeta::new(swap_accounts.token_vault_a.key(), false),
        AccountMeta::new(token_owner_account_b.key(), false),
        AccountMeta::new(swap_accounts.token_vault_b.key(), false),
        AccountMeta::new(swap_accounts.tick_array0.key(), false),
        AccountMeta::new(swap_accounts.tick_array1.key(), false),
        AccountMeta::new(swap_accounts.tick_array2.key(), false),
        AccountMeta::new(swap_accounts.oracle.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.token_program_a.to_account_info(),
        swap_accounts.token_program_b.to_account_info(),
        swap_accounts.memo_program.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.whirlpool.to_account_info(),
        swap_accounts.token_mint_a.to_account_info(),
        swap_accounts.token_mint_b.to_account_info(),
        token_owner_account_a.to_account_info(),
        swap_accounts.token_vault_a.to_account_info(),
        token_owner_account_b.to_account_info(),
        swap_accounts.token_vault_b.to_account_info(),
        swap_accounts.tick_array0.to_account_info(),
        swap_accounts.tick_array1.to_account_info(),
        swap_accounts.tick_array2.to_account_info(),
        swap_accounts.oracle.to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &WhirlpoolProcessor;
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
        ACCOUNTS_V2_LEN,
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
        let amount_specified_is_input = true;
        let other_amount_threshold = 1u64;
        let a_to_b = true;
        let sqrt_price_limit = 0u128;

        let mut data = Vec::with_capacity(ARGS_LEN);
        data.extend_from_slice(SWAP_SELECTOR);
        data.extend_from_slice(&amount_in.to_le_bytes());
        data.extend_from_slice(&other_amount_threshold.to_le_bytes());
        data.extend_from_slice(&sqrt_price_limit.to_le_bytes());
        data.extend_from_slice(&(amount_specified_is_input as u8).to_le_bytes());
        data.extend_from_slice(&(a_to_b as u8).to_le_bytes());

        msg!("data.len: {}", data.len());
        assert!(data.len() == ARGS_LEN);
    }
}
