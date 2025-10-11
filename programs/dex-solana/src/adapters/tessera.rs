use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use arrayref::array_ref;
use borsh::{BorshDeserialize, BorshSerialize};

use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{HopAccounts, TESSERA_SWAP_SELECTOR, tessera_program};

use super::common::DexProcessor;

const ARGS_LEN: usize = 18;

pub struct TesseraProcessor;
impl DexProcessor for TesseraProcessor {}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct SwapParams {
    pub side: u8,
    pub amount_in: u64,
    pub min_amount_out: u64,
}

pub struct TesseraAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub global_state: &'info AccountInfo<'info>,
    pub pool_state: &'info AccountInfo<'info>,
    pub base_vault: &'info AccountInfo<'info>,
    pub quote_vault: &'info AccountInfo<'info>,
    pub base_mint: InterfaceAccount<'info, Mint>,
    pub quote_mint: InterfaceAccount<'info, Mint>,
    pub base_token_program: Interface<'info, TokenInterface>,
    pub quote_token_program: Interface<'info, TokenInterface>,
    pub sysvar_instructions: &'info AccountInfo<'info>,
}
const ACCOUNTS_LEN: usize = 13;

impl<'info> TesseraAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            global_state,
            pool_state,
            base_vault,
            quote_vault,
            base_mint,
            quote_mint,
            base_token_program,
            quote_token_program,
            sysvar_instructions,
        ]: &[AccountInfo<'info>; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            global_state,
            pool_state,
            base_vault,
            quote_vault,
            base_mint: InterfaceAccount::try_from(base_mint)?,
            quote_mint: InterfaceAccount::try_from(quote_mint)?,
            base_token_program: Interface::try_from(base_token_program)?,
            quote_token_program: Interface::try_from(quote_token_program)?,
            sysvar_instructions,
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
    msg!("Dex::Tessera amount_in: {}, offset: {}", amount_in, offset);

    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);

    let mut swap_accounts = TesseraAccounts::parse_accounts(remaining_accounts, *offset)?;

    if swap_accounts.dex_program_id.key != &tessera_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }

    swap_accounts.pool_state.key().log();

    before_check(
        &swap_accounts.swap_authority_pubkey,
        &swap_accounts.swap_source_token,
        swap_accounts.swap_destination_token.key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let is_base_in = if swap_accounts.swap_source_token.mint == swap_accounts.base_mint.key() {
        true
    } else if swap_accounts.swap_source_token.mint == swap_accounts.quote_mint.key() {
        false
    } else {
        return Err(ErrorCode::InvalidTokenMint.into());
    };

    let expected_destination_mint =
        if is_base_in { swap_accounts.quote_mint.key() } else { swap_accounts.base_mint.key() };

    if swap_accounts.swap_destination_token.mint != expected_destination_mint {
        return Err(ErrorCode::InvalidTokenMint.into());
    }

    // Map source and destination to base and quote accounts correctly
    let (base_account, quote_account) = if is_base_in {
        (swap_accounts.swap_source_token.clone(), swap_accounts.swap_destination_token.clone())
    } else {
        (swap_accounts.swap_destination_token.clone(), swap_accounts.swap_source_token.clone())
    };

    let swap_params: SwapParams = SwapParams {
        side: if is_base_in {
            1 // base → quote
        } else {
            0 // quote → base
        },
        amount_in,
        min_amount_out: 1,
    };

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(TESSERA_SWAP_SELECTOR);
    data.extend_from_slice(&swap_params.try_to_vec()?);

    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.global_state.key(), false),
        AccountMeta::new(swap_accounts.pool_state.key(), false),
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new(swap_accounts.base_vault.key(), false),
        AccountMeta::new(swap_accounts.quote_vault.key(), false),
        AccountMeta::new(base_account.key(), false),
        AccountMeta::new(quote_account.key(), false),
        AccountMeta::new_readonly(swap_accounts.base_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.quote_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.base_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.quote_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.sysvar_instructions.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.global_state.to_account_info(),
        swap_accounts.pool_state.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.base_vault.to_account_info(),
        swap_accounts.quote_vault.to_account_info(),
        base_account.to_account_info(),
        quote_account.to_account_info(),
        swap_accounts.base_mint.to_account_info(),
        swap_accounts.quote_mint.to_account_info(),
        swap_accounts.base_token_program.to_account_info(),
        swap_accounts.quote_token_program.to_account_info(),
        swap_accounts.sysvar_instructions.to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &TesseraProcessor;
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
