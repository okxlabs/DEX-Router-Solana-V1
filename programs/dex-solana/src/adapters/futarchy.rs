use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{HopAccounts, SPOT_SWAP_SELECTOR, futarchy_amm_program};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token_interface::{TokenAccount, TokenInterface};
use arrayref::array_ref;
use borsh::{BorshDeserialize, BorshSerialize};

use super::common::DexProcessor;

pub struct FutarchyAmmProcessor;
impl DexProcessor for FutarchyAmmProcessor {}

const ARGS_LEN: usize = 25;

#[derive(BorshDeserialize, BorshSerialize)]
pub enum SwapType {
    Buy,
    Sell,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct SpotSwapParams {
    pub input_amount: u64,
    pub swap_type: SwapType,
    pub min_output_amount: u64,
}

pub struct FutarchyAmmAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub dao: &'info AccountInfo<'info>,
    pub amm_base_vault: InterfaceAccount<'info, TokenAccount>,
    pub amm_quote_vault: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
    pub event_authority: &'info AccountInfo<'info>,
}
const ACCOUNTS_LEN: usize = 9;

impl<'info> FutarchyAmmAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            dao,
            amm_base_vault,
            amm_quote_vault,
            token_program,
            event_authority,
        ]: &[AccountInfo<'info>; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            dao,
            amm_base_vault: InterfaceAccount::try_from(amm_base_vault)?,
            amm_quote_vault: InterfaceAccount::try_from(amm_quote_vault)?,
            token_program: Interface::try_from(token_program)?,
            event_authority,
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
    msg!("Dex::FutarchyAmm amount_in: {}, offset: {}", amount_in, offset);
    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);
    let mut swap_accounts = FutarchyAmmAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &futarchy_amm_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.dao.key().log();

    before_check(
        &swap_accounts.swap_authority_pubkey,
        &swap_accounts.swap_source_token,
        swap_accounts.swap_destination_token.key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let quote_mint = swap_accounts.amm_quote_vault.mint;
    let base_mint = swap_accounts.amm_base_vault.mint;
    let (swap_params, user_base_account, user_quote_account) =
        if swap_accounts.swap_source_token.mint == quote_mint
            && swap_accounts.swap_destination_token.mint == base_mint
        {
            (
                SpotSwapParams {
                    input_amount: amount_in,
                    swap_type: SwapType::Buy,
                    min_output_amount: 1,
                },
                swap_accounts.swap_destination_token.clone(),
                swap_accounts.swap_source_token.clone(),
            )
        } else if swap_accounts.swap_source_token.mint == base_mint
            && swap_accounts.swap_destination_token.mint == quote_mint
        {
            (
                SpotSwapParams {
                    input_amount: amount_in,
                    swap_type: SwapType::Sell,
                    min_output_amount: 1,
                },
                swap_accounts.swap_source_token.clone(),
                swap_accounts.swap_destination_token.clone(),
            )
        } else {
            return Err(ErrorCode::InvalidTokenMint.into());
        };

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(SPOT_SWAP_SELECTOR);
    data.extend_from_slice(&swap_params.try_to_vec()?);

    let mut accounts = Vec::with_capacity(ACCOUNTS_LEN);
    accounts.push(AccountMeta::new(swap_accounts.dao.key(), false));
    accounts.push(AccountMeta::new(user_base_account.key(), false));
    accounts.push(AccountMeta::new(user_quote_account.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.amm_base_vault.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.amm_quote_vault.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true));
    accounts.push(AccountMeta::new_readonly(swap_accounts.token_program.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.event_authority.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.dex_program_id.key(), false));

    let mut account_infos = Vec::with_capacity(ACCOUNTS_LEN);
    account_infos.push(swap_accounts.dao.to_account_info());
    account_infos.push(user_base_account.to_account_info());
    account_infos.push(user_quote_account.to_account_info());
    account_infos.push(swap_accounts.amm_base_vault.to_account_info());
    account_infos.push(swap_accounts.amm_quote_vault.to_account_info());
    account_infos.push(swap_accounts.swap_authority_pubkey.to_account_info());
    account_infos.push(swap_accounts.token_program.to_account_info());
    account_infos.push(swap_accounts.event_authority.to_account_info());
    account_infos.push(swap_accounts.dex_program_id.to_account_info());

    let instruction = Instruction { program_id: futarchy_amm_program::id(), accounts, data };

    let amount_out = invoke_process(
        amount_in,
        &FutarchyAmmProcessor,
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
