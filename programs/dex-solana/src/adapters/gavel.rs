use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_spl::token_interface::TokenAccount;

use super::common::DexProcessor;
use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{HopAccounts, gavel_program};
use arrayref::array_ref;

const ARGS_LEN: usize = 19;

pub struct GavelSwapAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub log_authority: &'info AccountInfo<'info>,
    pub pool: &'info AccountInfo<'info>,
    pub base_vault: InterfaceAccount<'info, TokenAccount>,
    pub quote_vault: InterfaceAccount<'info, TokenAccount>,
    pub token_program: &'info AccountInfo<'info>,
}

const SWAP_ACCOUNTS_LEN: usize = 9;

impl<'info> GavelSwapAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            log_authority,
            pool,
            base_vault,
            quote_vault,
            token_program,
        ]: &[AccountInfo<'info>; SWAP_ACCOUNTS_LEN] =
            array_ref![accounts, offset, SWAP_ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            log_authority,
            pool,
            base_vault: InterfaceAccount::try_from(base_vault)?,
            quote_vault: InterfaceAccount::try_from(quote_vault)?,
            token_program,
        })
    }
}

pub struct GavelProcessor;
impl DexProcessor for GavelProcessor {}

pub fn swap<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::GavelSwap amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + SWAP_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );

    let mut swap_accounts = GavelSwapAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &gavel_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }

    swap_accounts.pool.key().log();

    // check hop accounts & swap authority
    before_check(
        &swap_accounts.swap_authority_pubkey,
        &swap_accounts.swap_source_token,
        swap_accounts.swap_destination_token.key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let (direction, user_base_token_account, user_quote_token_account) = if swap_accounts
        .swap_source_token
        .mint
        == swap_accounts.quote_vault.mint
        && swap_accounts.swap_destination_token.mint == swap_accounts.base_vault.mint
    {
        (0u8, swap_accounts.swap_destination_token.clone(), swap_accounts.swap_source_token.clone())
    } else if swap_accounts.swap_source_token.mint == swap_accounts.base_vault.mint
        && swap_accounts.swap_destination_token.mint == swap_accounts.quote_vault.mint
    {
        (1u8, swap_accounts.swap_source_token.clone(), swap_accounts.swap_destination_token.clone())
    } else {
        return Err(ErrorCode::InvalidTokenMint.into());
    };

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.push(0u8); //discriminator
    data.push(direction); //direction
    data.push(0u8); //exact_in
    data.extend_from_slice(&amount_in.to_le_bytes()); //amount_in
    data.extend_from_slice(&1u64.to_le_bytes()); // min_amount_out

    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.dex_program_id.key(), false),
        AccountMeta::new_readonly(swap_accounts.log_authority.key(), false),
        AccountMeta::new(swap_accounts.pool.key(), false),
        AccountMeta::new_readonly(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new(user_base_token_account.key(), false),
        AccountMeta::new(user_quote_token_account.key(), false),
        AccountMeta::new(swap_accounts.base_vault.key(), false),
        AccountMeta::new(swap_accounts.quote_vault.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.dex_program_id.to_account_info(),
        swap_accounts.log_authority.to_account_info(),
        swap_accounts.pool.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        user_base_token_account.to_account_info(),
        user_quote_token_account.to_account_info(),
        swap_accounts.base_vault.to_account_info(),
        swap_accounts.quote_vault.to_account_info(),
        swap_accounts.token_program.to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &GavelProcessor;
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
        SWAP_ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )?;
    Ok(amount_out)
}
