use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{
    HopAccounts, SABER_DECIMAL_DEPOSIT_SELECTOR, SABER_DECIMAL_WITHDRAW_SELECTOR,
    saber_decimal_wrapper_program,
};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token_interface::TokenAccount;
use arrayref::array_ref;

use super::common::DexProcessor;

const ARGS_LEN: usize = 16;
pub struct SaberDecimalProcessor;
impl DexProcessor for SaberDecimalProcessor {}
pub struct SaberDecimalWrapperAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub wrapper: &'info AccountInfo<'info>,
    pub wrapper_mint: &'info AccountInfo<'info>,
    pub wrapper_underlying_tokens: &'info AccountInfo<'info>,
    pub owner: &'info AccountInfo<'info>,
    pub user_underlying_tokens: &'info AccountInfo<'info>,
    pub user_wrapped_tokens: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
}

const ACCOUNTS_LEN: usize = 11;

impl<'info> SaberDecimalWrapperAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            wrapper,
            wrapper_mint,
            wrapper_underlying_tokens,
            owner,
            user_underlying_tokens,
            user_wrapped_tokens,
            token_program,
        ]: &[AccountInfo; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            wrapper,
            wrapper_mint,
            wrapper_underlying_tokens,
            owner,
            user_underlying_tokens,
            user_wrapped_tokens,
            token_program,
        })
    }
}

pub fn deposit<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::SaberDecimalWrapperDeposit amount_in: {}, offset: {}", amount_in, offset);
    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);
    let mut swap_accounts =
        SaberDecimalWrapperAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &saber_decimal_wrapper_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.wrapper.key().log();

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

    let account_meta = vec![
        AccountMeta::new_readonly(swap_accounts.wrapper.key(), false),
        AccountMeta::new(swap_accounts.wrapper_mint.key(), false),
        AccountMeta::new(swap_accounts.wrapper_underlying_tokens.key(), false),
        AccountMeta::new(swap_accounts.owner.key(), true),
        AccountMeta::new(swap_accounts.user_underlying_tokens.key(), false),
        AccountMeta::new(swap_accounts.user_wrapped_tokens.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
    ];

    let account_info = [
        swap_accounts.wrapper.to_account_info(),
        swap_accounts.wrapper_mint.to_account_info(),
        swap_accounts.wrapper_underlying_tokens.to_account_info(),
        swap_accounts.owner.to_account_info(),
        swap_accounts.user_underlying_tokens.to_account_info(),
        swap_accounts.user_wrapped_tokens.to_account_info(),
        swap_accounts.token_program.to_account_info(),
    ];

    let mut data = vec![0; ARGS_LEN];
    data[0..8].copy_from_slice(&SABER_DECIMAL_DEPOSIT_SELECTOR[..]);
    data[8..16].copy_from_slice(&amount_in.to_le_bytes());

    let instruction =
        Instruction { program_id: *swap_accounts.dex_program_id.key, accounts: account_meta, data };

    let dex_processor = SaberDecimalProcessor {};
    let amount_out = invoke_process(
        amount_in,
        &dex_processor,
        &account_info,
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

pub fn withdraw<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::SaberDecimalWrapperWithdraw amount_in: {}, offset: {}", amount_in, offset);
    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);
    let mut swap_accounts =
        SaberDecimalWrapperAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &saber_decimal_wrapper_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.wrapper.key().log();

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

    let account_meta = vec![
        AccountMeta::new_readonly(swap_accounts.wrapper.key(), false),
        AccountMeta::new(swap_accounts.wrapper_mint.key(), false),
        AccountMeta::new(swap_accounts.wrapper_underlying_tokens.key(), false),
        AccountMeta::new(swap_accounts.owner.key(), true),
        AccountMeta::new(swap_accounts.user_underlying_tokens.key(), false),
        AccountMeta::new(swap_accounts.user_wrapped_tokens.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
    ];

    let account_info = [
        swap_accounts.wrapper.to_account_info(),
        swap_accounts.wrapper_mint.to_account_info(),
        swap_accounts.wrapper_underlying_tokens.to_account_info(),
        swap_accounts.owner.to_account_info(),
        swap_accounts.user_underlying_tokens.to_account_info(),
        swap_accounts.user_wrapped_tokens.to_account_info(),
        swap_accounts.token_program.to_account_info(),
    ];

    let mut data = vec![0; ARGS_LEN];
    data[0..8].copy_from_slice(&SABER_DECIMAL_WITHDRAW_SELECTOR[..]);
    data[8..16].copy_from_slice(&amount_in.to_le_bytes());

    let instruction =
        Instruction { program_id: *swap_accounts.dex_program_id.key, accounts: account_meta, data };

    let dex_processor = SaberDecimalProcessor {};
    invoke_process(
        amount_in,
        &dex_processor,
        &account_info,
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

    Ok(amount_in)
}
