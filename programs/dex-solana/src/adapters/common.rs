use crate::error::ErrorCode;
use crate::{authority_pda, HopAccounts, SA_AUTHORITY_SEED, ZERO_ADDRESS};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::Instruction,
    program::{invoke, invoke_signed},
};
use anchor_spl::token_interface::TokenAccount;

pub trait DexProcessor {
    fn before_invoke(&self, _account_infos: &[AccountInfo]) -> Result<u64> {
        Ok(0)
    }

    fn after_invoke(
        &self,
        _account_infos: &[AccountInfo],
        _hop: usize,
        _owner_seeds: Option<&[&[&[u8]]]>,
    ) -> Result<u64> {
        Ok(0)
    }
}

pub fn before_check(
    swap_authority_pubkey: &AccountInfo,
    swap_source_token_account: &InterfaceAccount<TokenAccount>,
    swap_destination_token: Pubkey,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<()> {
    // check hop accounts
    let swap_source_token = swap_source_token_account.key();
    if hop_accounts.from_account != ZERO_ADDRESS {
        require_keys_eq!(
            swap_source_token,
            hop_accounts.from_account,
            ErrorCode::InvalidHopAccounts
        );
        require_keys_eq!(
            swap_destination_token,
            hop_accounts.to_account,
            ErrorCode::InvalidHopAccounts
        );
    }
    if hop_accounts.last_to_account != ZERO_ADDRESS {
        require_keys_eq!(
            swap_source_token,
            hop_accounts.last_to_account,
            ErrorCode::InvalidHopFromAccount
        );
    }

    // check swap authority
    require!(
        swap_authority_pubkey.key() == swap_source_token_account.owner,
        ErrorCode::InvalidSwapAuthority
    );
    if !proxy_swap && hop == 0 {
        if owner_seeds.is_none() {
            require!(
                swap_authority_pubkey.is_signer,
                ErrorCode::SwapAuthorityIsNotSigner
            );
        }
    } else {
        require_keys_eq!(
            swap_authority_pubkey.key(),
            authority_pda::id(),
            ErrorCode::InvalidAuthorityPda
        );
    }
    Ok(())
}

pub fn invoke_process<'info, T: DexProcessor>(
    dex_processor: &T,
    account_infos: &[AccountInfo],
    swap_source_token: Pubkey,
    swap_destination_account: &mut InterfaceAccount<'info, TokenAccount>,
    hop_accounts: &mut HopAccounts,
    instruction: Instruction,
    hop: usize,
    offset: &mut usize,
    accounts_len: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    // check if pumpfun swap
    let before_destination_balance = swap_destination_account.amount;
    dex_processor.before_invoke(account_infos)?;
    execute_instruction(&instruction, account_infos, proxy_swap, hop, owner_seeds)?;

    // check if pumpfun swap
    dex_processor.after_invoke(account_infos, hop, owner_seeds)?;
    post_swap_check(
        swap_destination_account,
        hop_accounts,
        swap_source_token,
        accounts_len,
        offset,
        before_destination_balance,
    )
}

pub fn invoke_processes<'info, T: DexProcessor>(
    dex_processor: &T,
    account_infos_arr: &[&[AccountInfo]],
    swap_source_token: Pubkey,
    swap_destination_account: &mut InterfaceAccount<'info, TokenAccount>,
    hop_accounts: &mut HopAccounts,
    instructions: &[Instruction],
    hop: usize,
    offset: &mut usize,
    accounts_len: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    require!(
        account_infos_arr.len() == instructions.len(),
        ErrorCode::InvalidBundleInput
    );
    // check if pumpfun swap
    let before_destination_balance = swap_destination_account.amount;

    let account_infos: Vec<_> = account_infos_arr
        .iter()
        .flat_map(|inner| inner.iter())
        .cloned()
        .collect();
    dex_processor.before_invoke(&account_infos)?;

    for i in 0..instructions.len() {
        execute_instruction(
            &instructions[i],
            account_infos_arr[i],
            proxy_swap,
            hop,
            owner_seeds,
        )?;
    }

    // check if pumpfun swap
    dex_processor.after_invoke(&account_infos, hop, owner_seeds)?;
    post_swap_check(
        swap_destination_account,
        hop_accounts,
        swap_source_token,
        accounts_len,
        offset,
        before_destination_balance,
    )
}

// Helper function to execute the instruction
fn execute_instruction(
    instruction: &Instruction,
    account_infos: &[AccountInfo],
    proxy_swap: bool,
    hop: usize,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<()> {
    if !proxy_swap && hop == 0 {
        if owner_seeds.is_none() {
            invoke(instruction, account_infos)?;
        } else {
            invoke_signed(instruction, account_infos, owner_seeds.unwrap())?;
        }
    } else {
        invoke_signed(instruction, account_infos, SA_AUTHORITY_SEED)?;
    }
    Ok(())
}

fn post_swap_check<'info>(
    swap_destination_account: &mut InterfaceAccount<'info, TokenAccount>,
    hop_accounts: &mut HopAccounts,
    swap_source_token: Pubkey,
    accounts_len: usize,
    offset: &mut usize,
    before_destination_balance: u64,
) -> Result<u64> {
    swap_destination_account.reload()?;
    let after_destination_balance = swap_destination_account.amount;
    *offset += accounts_len;
    hop_accounts.from_account = swap_source_token;
    hop_accounts.to_account = swap_destination_account.key();
    let amount_out = after_destination_balance
        .checked_sub(before_destination_balance)
        .ok_or(ErrorCode::CalculationError)?;
    Ok(amount_out)
}
