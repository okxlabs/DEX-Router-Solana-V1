use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::{token::Token, token_interface::TokenAccount};
use arrayref::array_ref;

use crate::ONE_DEX_SWAP_SELECTOR;
use crate::error::ErrorCode;
use crate::{
    HopAccounts,
    adapters::common::{before_check, invoke_process},
    one_dex_program,
};

use super::common::DexProcessor;

const ARGS_LEN: usize = 24;

pub struct OneDexSwapAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub metadata_state: &'info AccountInfo<'info>,
    pub pool_state: &'info AccountInfo<'info>,
    pub pool_auth_pubkey: &'info AccountInfo<'info>,
    pub pool_token_in_account: &'info AccountInfo<'info>,
    pub pool_token_out_account: &'info AccountInfo<'info>,
    pub metadata_swap_fee_account: &'info AccountInfo<'info>,
    pub referrer_token_account: &'info AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}

const SWAP_ACCOUNTS_LEN: usize = 12;

impl<'info> OneDexSwapAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            metadata_state,
            pool_state,
            pool_auth_pubkey,
            pool_token_in_account,
            pool_token_out_account,
            metadata_swap_fee_account,
            referrer_token_account,
            token_program,
        ]: &[AccountInfo<'info>; SWAP_ACCOUNTS_LEN] =
            array_ref![accounts, offset, SWAP_ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            metadata_state,
            pool_state,
            pool_auth_pubkey,
            pool_token_in_account,
            pool_token_out_account,
            metadata_swap_fee_account,
            referrer_token_account,
            token_program: Program::try_from(token_program)?,
        })
    }
}

pub struct OneDexSwapProcessor;
impl DexProcessor for OneDexSwapProcessor {}

pub fn swap<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::OneDexSwap amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + SWAP_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );

    let mut swap_accounts = OneDexSwapAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &one_dex_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    swap_accounts.pool_auth_pubkey.key().log();

    before_check(
        &swap_accounts.swap_authority_pubkey,
        &swap_accounts.swap_source_token,
        swap_accounts.swap_destination_token.key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(ONE_DEX_SWAP_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&1u64.to_le_bytes());

    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.metadata_state.key(), false),
        AccountMeta::new(swap_accounts.pool_state.key(), false),
        AccountMeta::new_readonly(swap_accounts.pool_auth_pubkey.key(), false),
        AccountMeta::new(swap_accounts.pool_token_in_account.key(), false),
        AccountMeta::new(swap_accounts.pool_token_out_account.key(), false),
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new(swap_accounts.swap_source_token.key(), false),
        AccountMeta::new(swap_accounts.swap_destination_token.key(), false),
        AccountMeta::new(swap_accounts.metadata_swap_fee_account.key(), false),
        AccountMeta::new(swap_accounts.referrer_token_account.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
    ];

    let account_info = vec![
        swap_accounts.metadata_state.to_account_info(),
        swap_accounts.pool_state.to_account_info(),
        swap_accounts.pool_auth_pubkey.to_account_info(),
        swap_accounts.pool_token_in_account.to_account_info(),
        swap_accounts.pool_token_out_account.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.metadata_swap_fee_account.to_account_info(),
        swap_accounts.referrer_token_account.to_account_info(),
        swap_accounts.token_program.to_account_info(),
    ];
    let instruction: Instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &OneDexSwapProcessor;
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
