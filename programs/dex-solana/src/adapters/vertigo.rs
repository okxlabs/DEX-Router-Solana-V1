use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{HopAccounts, VERTIGO_BUY_SELECTOR, VERTIGO_SELL_SELECTOR, vertigo_program};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use arrayref::array_ref;

use super::common::DexProcessor;

const ARGS_LEN: usize = 24;

pub struct VertigoSwapAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub pool: &'info AccountInfo<'info>,
    pub owner: &'info AccountInfo<'info>,
    pub mint_a: InterfaceAccount<'info, Mint>,
    pub mint_b: InterfaceAccount<'info, Mint>,
    pub vault_a: InterfaceAccount<'info, TokenAccount>,
    pub vault_b: InterfaceAccount<'info, TokenAccount>,
    pub token_program_a: Interface<'info, TokenInterface>,
    pub token_program_b: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}
const ACCOUNTS_LEN: usize = 13;

impl<'info> VertigoSwapAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            pool,
            owner,
            mint_a,
            mint_b,
            vault_a,
            vault_b,
            token_program_a,
            token_program_b,
            system_program,
        ]: &[AccountInfo<'info>; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];
        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            pool,
            owner,
            mint_a: InterfaceAccount::try_from(mint_a)?,
            mint_b: InterfaceAccount::try_from(mint_b)?,
            vault_a: InterfaceAccount::try_from(vault_a)?,
            vault_b: InterfaceAccount::try_from(vault_b)?,
            token_program_a: Interface::try_from(token_program_a)?,
            token_program_b: Interface::try_from(token_program_b)?,
            system_program: Program::try_from(system_program)?,
        })
    }
}

pub struct VertigoProcessor;
impl DexProcessor for VertigoProcessor {}

// wsol -> anytoken
pub fn buy<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::Vertigo amount_in: {}, offset: {}", amount_in, offset);
    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);

    let mut swap_accounts = VertigoSwapAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &vertigo_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.pool.key().log();

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
    data.extend_from_slice(VERTIGO_BUY_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes()); // wsol token
    data.extend_from_slice(&0u64.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(swap_accounts.pool.key(), false),
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.owner.key(), false),
        AccountMeta::new_readonly(swap_accounts.mint_a.key(), false),
        AccountMeta::new_readonly(swap_accounts.mint_b.key(), false),
        AccountMeta::new(swap_accounts.swap_source_token.key(), false),
        AccountMeta::new(swap_accounts.swap_destination_token.key(), false),
        AccountMeta::new(swap_accounts.vault_a.key(), false),
        AccountMeta::new(swap_accounts.vault_b.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program_a.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program_b.key(), false),
        AccountMeta::new_readonly(swap_accounts.system_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.dex_program_id.key(), false),
    ];

    let account_info = vec![
        swap_accounts.pool.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.owner.to_account_info(),
        swap_accounts.mint_a.to_account_info(),
        swap_accounts.mint_b.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.vault_a.to_account_info(),
        swap_accounts.vault_b.to_account_info(),
        swap_accounts.token_program_a.to_account_info(),
        swap_accounts.token_program_b.to_account_info(),
        swap_accounts.system_program.to_account_info(),
        swap_accounts.dex_program_id.to_account_info(),
    ];

    let instruction: Instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &VertigoProcessor;
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
        ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )?;

    Ok(amount_out)
}

// anytoken -> wsol
pub fn sell<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::Vertigo amount_in: {}, offset: {}", amount_in, offset);
    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);

    let mut swap_accounts = VertigoSwapAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &vertigo_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.pool.key().log();

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
    data.extend_from_slice(VERTIGO_SELL_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes()); //  anytoken amount
    data.extend_from_slice(&0u64.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(swap_accounts.pool.key(), false),
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.owner.key(), false),
        AccountMeta::new_readonly(swap_accounts.mint_a.key(), false), // wsol
        AccountMeta::new_readonly(swap_accounts.mint_b.key(), false), // anytoken
        AccountMeta::new(swap_accounts.swap_destination_token.key(), false), // wsol
        AccountMeta::new(swap_accounts.swap_source_token.key(), false), // anytoken
        AccountMeta::new(swap_accounts.vault_a.key(), false),         // wsol
        AccountMeta::new(swap_accounts.vault_b.key(), false),         // anytoken
        AccountMeta::new_readonly(swap_accounts.token_program_a.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program_b.key(), false),
        AccountMeta::new_readonly(swap_accounts.system_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.dex_program_id.key(), false),
    ];

    let account_info = vec![
        swap_accounts.pool.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.owner.to_account_info(),
        swap_accounts.mint_a.to_account_info(),
        swap_accounts.mint_b.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.vault_a.to_account_info(),
        swap_accounts.vault_b.to_account_info(),
        swap_accounts.token_program_a.to_account_info(),
        swap_accounts.token_program_b.to_account_info(),
        swap_accounts.system_program.to_account_info(),
        swap_accounts.dex_program_id.to_account_info(),
    ];

    let instruction: Instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &VertigoProcessor;
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
        ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )?;

    Ok(amount_out)
}
