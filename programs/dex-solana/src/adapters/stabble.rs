use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::{
    token::Token,
    token_2022::Token2022,
    token_interface::{Mint, TokenAccount},
};
use arrayref::array_ref;

use crate::error::ErrorCode;
use crate::{
    HopAccounts, STABBLE_SWAP_SELECTOR,
    adapters::common::{before_check, invoke_process},
    stabble_stable_program, stabble_weighted_program,
};

use super::common::DexProcessor;

const ARGS_LEN: usize = 25;

pub struct StabbleSwapAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub mint_in: InterfaceAccount<'info, Mint>,
    pub mint_out: InterfaceAccount<'info, Mint>,
    pub vault_token_in: InterfaceAccount<'info, TokenAccount>,
    pub vault_token_out: InterfaceAccount<'info, TokenAccount>,
    pub beneficiary_token_out: &'info AccountInfo<'info>,
    pub pool_token_in: &'info AccountInfo<'info>,
    pub withdraw_authority: &'info AccountInfo<'info>,
    pub vault: &'info AccountInfo<'info>,
    pub vault_authority: &'info AccountInfo<'info>,
    pub vault_program: &'info AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
    pub token_2022_program: Program<'info, Token2022>,
}

const SWAP_ACCOUNTS_LEN: usize = 16;

impl<'info> StabbleSwapAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            mint_in,
            mint_out,
            vault_token_in,
            vault_token_out,
            beneficiary_token_out,
            pool_token_in,
            withdraw_authority,
            vault,
            vault_authority,
            vault_program,
            token_program,
            token_2022_program,
        ]: &[AccountInfo<'info>; SWAP_ACCOUNTS_LEN] =
            array_ref![accounts, offset, SWAP_ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            mint_in: InterfaceAccount::try_from(mint_in)?,
            mint_out: InterfaceAccount::try_from(mint_out)?,
            vault_token_in: InterfaceAccount::try_from(vault_token_in)?,
            vault_token_out: InterfaceAccount::try_from(vault_token_out)?,
            beneficiary_token_out,
            pool_token_in,
            withdraw_authority,
            vault,
            vault_authority,
            vault_program,
            token_program: Program::try_from(token_program)?,
            token_2022_program: Program::try_from(token_2022_program)?,
        })
    }
}

pub struct StabbleSwapProcessor;
impl DexProcessor for StabbleSwapProcessor {}

pub fn swap<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::StabbleSwap amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + SWAP_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );

    let mut swap_accounts = StabbleSwapAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &stabble_stable_program::id()
        && swap_accounts.dex_program_id.key != &stabble_weighted_program::id()
    {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.pool_token_in.key().log();

    before_check(
        swap_accounts.swap_authority_pubkey,
        &swap_accounts.swap_source_token,
        swap_accounts.swap_destination_token.key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(STABBLE_SWAP_SELECTOR);
    data.extend_from_slice(&(Some(amount_in)).try_to_vec().unwrap());
    data.extend_from_slice(&1u64.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.mint_in.key(), false),
        AccountMeta::new_readonly(swap_accounts.mint_out.key(), false),
        AccountMeta::new(swap_accounts.swap_source_token.key(), false),
        AccountMeta::new(swap_accounts.swap_destination_token.key(), false),
        AccountMeta::new(swap_accounts.vault_token_in.key(), false),
        AccountMeta::new(swap_accounts.vault_token_out.key(), false),
        AccountMeta::new(swap_accounts.beneficiary_token_out.key(), false),
        AccountMeta::new(swap_accounts.pool_token_in.key(), false),
        AccountMeta::new_readonly(swap_accounts.withdraw_authority.key(), false),
        AccountMeta::new_readonly(swap_accounts.vault.key(), false),
        AccountMeta::new_readonly(swap_accounts.vault_authority.key(), false),
        AccountMeta::new_readonly(swap_accounts.vault_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_2022_program.key(), false),
    ];

    let account_info = vec![
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.mint_in.to_account_info(),
        swap_accounts.mint_out.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.vault_token_in.to_account_info(),
        swap_accounts.vault_token_out.to_account_info(),
        swap_accounts.beneficiary_token_out.to_account_info(),
        swap_accounts.pool_token_in.to_account_info(),
        swap_accounts.withdraw_authority.to_account_info(),
        swap_accounts.vault.to_account_info(),
        swap_accounts.vault_authority.to_account_info(),
        swap_accounts.vault_program.to_account_info(),
        swap_accounts.token_program.to_account_info(),
        swap_accounts.token_2022_program.to_account_info(),
    ];

    let instruction: Instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &StabbleSwapProcessor;
    let amount_out = invoke_process(
        amount_in,
        dex_processor,
        &account_info,
        &mut swap_accounts.swap_source_token,
        &mut swap_accounts.swap_destination_token,
        hop_accounts,
        instruction,
        hop,
        offset,
        SWAP_ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )?;

    Ok(amount_out)
}
