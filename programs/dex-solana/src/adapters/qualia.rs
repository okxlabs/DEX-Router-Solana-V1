use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use anchor_lang::prelude::*;
use anchor_lang::prelude::{AccountInfo, InterfaceAccount, Program};
use anchor_lang::solana_program::instruction::Instruction;
use anchor_spl::{token::Token, token_interface::TokenAccount};
use arrayref::array_ref;

use crate::{HopAccounts, qualia_program};

use super::common::DexProcessor;

const ARGS_LEN: usize = 74;

pub struct QualiaSwapAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub globale_state_id: &'info AccountInfo<'info>,
    pub log_account_id: &'info AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
    pub token_in_vault: InterfaceAccount<'info, TokenAccount>,
    pub token_out_vault: InterfaceAccount<'info, TokenAccount>,
    pub sysvar: &'info AccountInfo<'info>,
}

const SWAP_ACCOUNTS_LEN: usize = 10;

impl<'info> QualiaSwapAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            globale_state_id,
            log_account_id,
            token_program,
            token_in_vault,
            token_out_vault,
            sysvar,
        ]: &[AccountInfo<'info>; SWAP_ACCOUNTS_LEN] =
            array_ref![accounts, offset, SWAP_ACCOUNTS_LEN];
        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            globale_state_id,
            log_account_id,
            token_program: Program::try_from(token_program)?,
            token_in_vault: InterfaceAccount::try_from(token_in_vault)?,
            token_out_vault: InterfaceAccount::try_from(token_out_vault)?,
            sysvar,
        })
    }
}

pub struct QualiaSwapProcessor;
impl DexProcessor for QualiaSwapProcessor {}

pub fn swap<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::Qualia amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + SWAP_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );

    let mut swap_accounts = QualiaSwapAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &qualia_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }

    before_check(
        swap_accounts.swap_authority_pubkey,
        &swap_accounts.swap_source_token,
        swap_accounts.swap_destination_token.key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let input_mint = swap_accounts.token_in_vault.mint;
    let output_mint = swap_accounts.token_out_vault.mint;

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.push(4);
    data.extend_from_slice(&amount_in.to_le_bytes()); // amount
    data.extend_from_slice(&input_mint.to_bytes()); // input mint pubkey
    data.extend_from_slice(&output_mint.to_bytes()); // output mint pubkey
    data.extend_from_slice(&[0u8]); // swap mode

    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.dex_program_id.key(), false),
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new(swap_accounts.globale_state_id.key(), false),
        AccountMeta::new_readonly(swap_accounts.log_account_id.key(), false),
        AccountMeta::new(swap_accounts.swap_source_token.key(), false),
        AccountMeta::new(swap_accounts.swap_destination_token.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
        AccountMeta::new(swap_accounts.token_in_vault.key(), false),
        AccountMeta::new(swap_accounts.token_out_vault.key(), false),
        AccountMeta::new_readonly(swap_accounts.sysvar.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.dex_program_id.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.globale_state_id.to_account_info(),
        swap_accounts.log_account_id.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.token_program.to_account_info(),
        swap_accounts.token_in_vault.to_account_info(),
        swap_accounts.token_out_vault.to_account_info(),
        swap_accounts.sysvar.to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &QualiaSwapProcessor;
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
