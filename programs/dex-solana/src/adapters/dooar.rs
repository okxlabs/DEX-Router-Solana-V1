use std::vec;

use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{HopAccounts, dooar_program};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token_interface::TokenAccount;
use arrayref::array_ref;

use super::common::DexProcessor;

const ARGS_LEN: usize = 17;
pub struct DooarProcessor;
impl DexProcessor for DooarProcessor {}
pub struct DooarAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub token_swap: &'info AccountInfo<'info>,
    pub authority: &'info AccountInfo<'info>,
    pub user_transfer_authority: &'info AccountInfo<'info>,
    pub user_source: &'info AccountInfo<'info>,
    pub pool_source: &'info AccountInfo<'info>,
    pub pool_destination: &'info AccountInfo<'info>,
    pub user_destination: &'info AccountInfo<'info>,
    pub pool_mint: &'info AccountInfo<'info>,
    pub fee_account: &'info AccountInfo<'info>,
    pub refund_to: &'info AccountInfo<'info>,
}

const ACCOUNTS_LEN: usize = 14;

impl<'info> DooarAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            token_swap,
            authority,
            user_transfer_authority,
            user_source,
            pool_source,
            pool_destination,
            user_destination,
            pool_mint,
            fee_account,
            refund_to,
        ]: &[AccountInfo; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            token_swap,
            authority,
            user_transfer_authority,
            user_source,
            pool_source,
            pool_destination,
            user_destination,
            pool_mint,
            fee_account,
            refund_to,
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
    msg!("Dex::Dooar amount_in: {}, offset: {}", amount_in, offset);
    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);
    let mut swap_accounts = DooarAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &dooar_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.token_swap.key().log();

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

    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.token_swap.key(), false),
        AccountMeta::new_readonly(swap_accounts.authority.key(), false),
        AccountMeta::new_readonly(swap_accounts.user_transfer_authority.key(), true),
        AccountMeta::new(swap_accounts.user_source.key(), false),
        AccountMeta::new(swap_accounts.pool_source.key(), false),
        AccountMeta::new(swap_accounts.pool_destination.key(), false),
        AccountMeta::new(swap_accounts.user_destination.key(), false),
        AccountMeta::new(swap_accounts.pool_mint.key(), false),
        AccountMeta::new(swap_accounts.fee_account.key(), false),
        AccountMeta::new_readonly(swap_accounts.refund_to.key(), false),
    ];
    let account_infos = [
        swap_accounts.token_swap.to_account_info(),
        swap_accounts.authority.to_account_info(),
        swap_accounts.user_transfer_authority.to_account_info(),
        swap_accounts.user_source.to_account_info(),
        swap_accounts.pool_source.to_account_info(),
        swap_accounts.pool_destination.to_account_info(),
        swap_accounts.user_destination.to_account_info(),
        swap_accounts.pool_mint.to_account_info(),
        swap_accounts.fee_account.to_account_info(),
        swap_accounts.refund_to.to_account_info(),
    ];

    let mut data = vec![0u8; ARGS_LEN];
    data[0] = 1; // selector
    data[1..9].copy_from_slice(&amount_in.to_le_bytes());
    data[9..17].copy_from_slice(&1u64.to_le_bytes());

    let instruction = Instruction { program_id: *swap_accounts.dex_program_id.key, accounts, data };

    let dex_processor = DooarProcessor {};
    let amount_out = invoke_process(
        amount_in,
        &dex_processor,
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
