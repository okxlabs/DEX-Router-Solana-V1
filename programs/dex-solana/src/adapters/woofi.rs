use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{HopAccounts, WOOFI_SWAP_SELECTOR, woofi_program};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_spl::{token::Token, token_interface::TokenAccount};
use arrayref::array_ref;

use super::common::DexProcessor;

const ARGS_LEN: usize = 40;

pub struct WoofiAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub wooficonfig: &'info AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
    pub token_a_wooracle: &'info AccountInfo<'info>,
    pub token_a_woopool: &'info AccountInfo<'info>,
    pub a_token_vault: InterfaceAccount<'info, TokenAccount>,
    pub token_a_price_update: &'info AccountInfo<'info>,
    pub token_b_wooracle: &'info AccountInfo<'info>,
    pub token_b_woopool: &'info AccountInfo<'info>,
    pub b_token_vault: InterfaceAccount<'info, TokenAccount>,
    pub token_b_price_update: &'info AccountInfo<'info>,
    pub quote_pool: &'info AccountInfo<'info>,
    pub quote_price_update: &'info AccountInfo<'info>,
    pub quote_token_vault: InterfaceAccount<'info, TokenAccount>,
    pub rebate_to: &'info AccountInfo<'info>,
}

const ACCOUNTS_LEN: usize = 18;

impl<'info> WoofiAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            wooficonfig,
            token_program,
            token_a_wooracle,
            token_a_woopool,
            a_token_vault,
            token_a_price_update,
            token_b_wooracle,
            token_b_woopool,
            b_token_vault,
            token_b_price_update,
            quote_pool,
            quote_price_update,
            quote_token_vault,
            rebate_to,
        ]: &[AccountInfo<'info>; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            wooficonfig,
            token_program: Program::try_from(token_program)?,
            token_a_wooracle,
            token_a_woopool,
            a_token_vault: InterfaceAccount::try_from(a_token_vault)?,
            token_a_price_update,
            token_b_wooracle,
            token_b_woopool,
            b_token_vault: InterfaceAccount::try_from(b_token_vault)?,
            token_b_price_update,
            quote_pool,
            quote_price_update,
            quote_token_vault: InterfaceAccount::try_from(quote_token_vault)?,
            rebate_to,
        })
    }
}

pub struct WoofiProcessor;
impl DexProcessor for WoofiProcessor {}

pub fn swap<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::Woofi amount_in: {}, offset: {}", amount_in, offset);
    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);

    let mut swap_accounts = WoofiAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &woofi_program::id() {
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

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(WOOFI_SWAP_SELECTOR);
    data.extend_from_slice(&(amount_in as u128).to_le_bytes()); //amount_in
    data.extend_from_slice(&1u128.to_le_bytes()); //mini_amout_out

    let accounts = vec![
        AccountMeta::new(swap_accounts.wooficonfig.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new(swap_accounts.token_a_wooracle.key(), false),
        AccountMeta::new(swap_accounts.token_a_woopool.key(), false),
        AccountMeta::new(swap_accounts.swap_source_token.key(), false),
        AccountMeta::new(swap_accounts.a_token_vault.key(), false),
        AccountMeta::new(swap_accounts.token_a_price_update.key(), false),
        AccountMeta::new(swap_accounts.token_b_wooracle.key(), false),
        AccountMeta::new(swap_accounts.token_b_woopool.key(), false),
        AccountMeta::new(swap_accounts.swap_destination_token.key(), false),
        AccountMeta::new(swap_accounts.b_token_vault.key(), false),
        AccountMeta::new(swap_accounts.token_b_price_update.key(), false),
        AccountMeta::new(swap_accounts.quote_pool.key(), false),
        AccountMeta::new(swap_accounts.quote_price_update.key(), false),
        AccountMeta::new(swap_accounts.quote_token_vault.key(), false),
        AccountMeta::new(swap_accounts.rebate_to.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.wooficonfig.to_account_info(),
        swap_accounts.token_program.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.token_a_wooracle.to_account_info(),
        swap_accounts.token_a_woopool.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.a_token_vault.to_account_info(),
        swap_accounts.token_a_price_update.to_account_info(),
        swap_accounts.token_b_wooracle.to_account_info(),
        swap_accounts.token_b_woopool.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.b_token_vault.to_account_info(),
        swap_accounts.token_b_price_update.to_account_info(),
        swap_accounts.quote_pool.to_account_info(),
        swap_accounts.quote_price_update.to_account_info(),
        swap_accounts.quote_token_vault.to_account_info(),
        swap_accounts.rebate_to.to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &WoofiProcessor;
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
