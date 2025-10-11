use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{GAMMA_ORACLE_SWAP_SELECTOR, HopAccounts, goosefx_gamma_program};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use arrayref::array_ref;

use super::common::DexProcessor;

const ARGS_LEN: usize = 24;

pub struct GooseFxProcessor;
impl DexProcessor for GooseFxProcessor {}

pub struct GooseFxAccount<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    // Gamma pool accounts
    pub authority: &'info AccountInfo<'info>,
    pub amm_config: &'info AccountInfo<'info>,
    pub pool_state: &'info AccountInfo<'info>,
    pub input_vault: InterfaceAccount<'info, TokenAccount>,
    pub output_vault: InterfaceAccount<'info, TokenAccount>,
    pub input_token_program: Interface<'info, TokenInterface>,
    pub output_token_program: Interface<'info, TokenInterface>,
    pub input_token_mint: InterfaceAccount<'info, Mint>,
    pub output_token_mint: InterfaceAccount<'info, Mint>,
    pub observation_state: &'info AccountInfo<'info>,
}
const ACCOUNTS_LEN: usize = 14;

impl<'info> GooseFxAccount<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            authority,
            amm_config,
            pool_state,
            input_vault,
            output_vault,
            input_token_program,
            output_token_program,
            input_token_mint,
            output_token_mint,
            observation_state,
        ]: &[AccountInfo<'info>; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            authority,
            amm_config,
            pool_state,
            input_vault: InterfaceAccount::try_from(input_vault)?,
            output_vault: InterfaceAccount::try_from(output_vault)?,
            input_token_program: Interface::try_from(input_token_program)?,
            output_token_program: Interface::try_from(output_token_program)?,
            input_token_mint: InterfaceAccount::try_from(input_token_mint)?,
            output_token_mint: InterfaceAccount::try_from(output_token_mint)?,
            observation_state,
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
    msg!("Dex::goosefx_gamma amount_in: {}, offset: {}", amount_in, offset);
    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);
    let mut swap_accounts = GooseFxAccount::parse_accounts(remaining_accounts, *offset)?;

    if swap_accounts.dex_program_id.key != &goosefx_gamma_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }

    // log pool address
    swap_accounts.pool_state.key().log();

    // check hop accounts & swap authority
    let swap_source_token_account = swap_accounts.swap_source_token.clone();
    let swap_destination_token = swap_accounts.swap_destination_token.key();
    before_check(
        &swap_accounts.swap_authority_pubkey,
        &swap_source_token_account,
        swap_destination_token,
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    // Prepare instruction data for gamma oracle_based_swap_base_input
    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(GAMMA_ORACLE_SWAP_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes()); // amount_in
    data.extend_from_slice(&0u64.to_le_bytes()); // minimum_amount_out (set to 0, allow any slippage)

    // Build account info
    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.swap_authority_pubkey.key(), true), // payer (signer)
        AccountMeta::new_readonly(swap_accounts.authority.key(), false),            // authority
        AccountMeta::new_readonly(swap_accounts.amm_config.key(), false),           // amm_config
        AccountMeta::new(swap_accounts.pool_state.key(), false),                    // pool_state
        AccountMeta::new(swap_accounts.swap_source_token.key(), false), // input_token_account
        AccountMeta::new(swap_accounts.swap_destination_token.key(), false), // output_token_account
        AccountMeta::new(swap_accounts.input_vault.key(), false),       // input_vault
        AccountMeta::new(swap_accounts.output_vault.key(), false),      // output_vault
        AccountMeta::new_readonly(swap_accounts.input_token_program.key(), false), // input_token_program
        AccountMeta::new_readonly(swap_accounts.output_token_program.key(), false), // output_token_program
        AccountMeta::new_readonly(swap_accounts.input_token_mint.key(), false), // input_token_mint
        AccountMeta::new_readonly(swap_accounts.output_token_mint.key(), false), // output_token_mint
        AccountMeta::new(swap_accounts.observation_state.key(), false), // observation_state
    ];

    let account_infos = vec![
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.authority.to_account_info(),
        swap_accounts.amm_config.to_account_info(),
        swap_accounts.pool_state.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.input_vault.to_account_info(),
        swap_accounts.output_vault.to_account_info(),
        swap_accounts.input_token_program.to_account_info(),
        swap_accounts.output_token_program.to_account_info(),
        swap_accounts.input_token_mint.to_account_info(),
        swap_accounts.output_token_mint.to_account_info(),
        swap_accounts.observation_state.to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &GooseFxProcessor;
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
