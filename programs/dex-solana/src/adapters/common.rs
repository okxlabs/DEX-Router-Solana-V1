use crate::constants::{ACTUAL_IN_LOWER_BOUND_DEN, ACTUAL_IN_LOWER_BOUND_NUM};
use crate::error::ErrorCode;
use crate::{HopAccounts, SA_AUTHORITY_SEED, ZERO_ADDRESS, authority_pda};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::program_pack::Pack;
use anchor_lang::solana_program::{
    instruction::Instruction,
    program::{invoke, invoke_signed},
};
use anchor_spl::token::spl_token::state::Account as SplTokenAccount;
use anchor_spl::token_2022::spl_token_2022::state::Account as SplToken2022Account;
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
        _before_sa_authority_lamports: u64,
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
            require!(swap_authority_pubkey.is_signer, ErrorCode::SwapAuthorityIsNotSigner);
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
    amount_in: u64,
    dex_processor: &T,
    account_infos: &[AccountInfo],
    swap_source_token: &mut InterfaceAccount<'info, TokenAccount>,
    swap_destination_token: &mut InterfaceAccount<'info, TokenAccount>,
    hop_accounts: &mut HopAccounts,
    instruction: Instruction,
    hop: usize,
    offset: &mut usize,
    accounts_len: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    // get before balances
    let before_source_balance = swap_source_token.amount;
    let before_destination_balance = swap_destination_token.amount;

    // before invoke hook
    let before_sa_authority_lamports = dex_processor.before_invoke(account_infos)?;

    // Harden: prevent CPI from touching unexpected SA-owned token accounts
    if proxy_swap || hop > 0 {
        enforce_sa_token_allowlist(
            account_infos,
            &[swap_source_token.key(), swap_destination_token.key()],
        )?;
    }

    // execute instruction
    execute_instruction(&instruction, account_infos, proxy_swap, hop, owner_seeds)?;

    // after invoke hook
    dex_processor.after_invoke(account_infos, hop, owner_seeds, before_sa_authority_lamports)?;

    // post swap check
    post_swap_check(
        swap_source_token,
        swap_destination_token,
        hop_accounts,
        accounts_len,
        offset,
        amount_in,
        before_source_balance,
        before_destination_balance,
    )
}

pub fn invoke_processes<'info, T: DexProcessor>(
    amount_in: u64,
    dex_processor: &T,
    account_infos_arr: &[&[AccountInfo]],
    swap_source_token: &mut InterfaceAccount<'info, TokenAccount>,
    swap_destination_token: &mut InterfaceAccount<'info, TokenAccount>,
    hop_accounts: &mut HopAccounts,
    instructions: &[Instruction],
    hop: usize,
    offset: &mut usize,
    accounts_len: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    // check accounts length
    require!(account_infos_arr.len() == instructions.len(), ErrorCode::InvalidBundleInput);

    // get before balances
    let before_source_balance = swap_source_token.amount;
    let before_destination_balance = swap_destination_token.amount;

    let account_infos: Vec<_> =
        account_infos_arr.iter().flat_map(|inner| inner.iter()).cloned().collect();

    // before invoke hook
    let before_sa_authority_lamports = dex_processor.before_invoke(&account_infos)?;

    // execute instructions
    for i in 0..instructions.len() {
        if proxy_swap || hop > 0 {
            enforce_sa_token_allowlist(
                account_infos_arr[i],
                &[swap_source_token.key(), swap_destination_token.key()],
            )?;
        }
        execute_instruction(&instructions[i], account_infos_arr[i], proxy_swap, hop, owner_seeds)?;
    }

    // after invoke hook
    dex_processor.after_invoke(&account_infos, hop, owner_seeds, before_sa_authority_lamports)?;

    // post swap check
    post_swap_check(
        swap_source_token,
        swap_destination_token,
        hop_accounts,
        accounts_len,
        offset,
        amount_in,
        before_source_balance,
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
    swap_source_token: &mut InterfaceAccount<'info, TokenAccount>,
    swap_destination_token: &mut InterfaceAccount<'info, TokenAccount>,
    hop_accounts: &mut HopAccounts,
    accounts_len: usize,
    offset: &mut usize,
    amount_in: u64,
    before_source_balance: u64,
    before_destination_balance: u64,
) -> Result<u64> {
    // 1. calculate & check actual amount in
    if swap_source_token.get_lamports() > 0 {
        // source token account may be closed in pumpfun buy
        swap_source_token.reload()?;
        let after_source_balance = swap_source_token.amount;
        let actual_amount_in = before_source_balance
            .checked_sub(after_source_balance)
            .ok_or(ErrorCode::CalculationError)?;

        // min_amount_in = 95% of amount_in
        let min_amount_in = u64::try_from(
            u128::from(amount_in)
                .checked_mul(ACTUAL_IN_LOWER_BOUND_NUM)
                .and_then(|v| v.checked_div(ACTUAL_IN_LOWER_BOUND_DEN))
                .ok_or(ErrorCode::CalculationError)?,
        )
        .map_err(|_| ErrorCode::CalculationError)?;
        if !(actual_amount_in <= amount_in && actual_amount_in >= min_amount_in) {
            msg!(
                "InvalidActualAmountIn: actual_amount_in={}, amount_in={}",
                actual_amount_in,
                amount_in,
            );
            return Err(ErrorCode::InvalidActualAmountIn.into());
        }
    }

    // 2. calculate & check actual amount out
    swap_destination_token.reload()?;
    let after_destination_balance = swap_destination_token.amount;
    let actual_amount_out = after_destination_balance
        .checked_sub(before_destination_balance)
        .ok_or(ErrorCode::CalculationError)?;
    require!(actual_amount_out > 0, ErrorCode::AmountOutMustBeGreaterThanZero);

    // 3. update offset & hop accounts
    *offset += accounts_len;
    hop_accounts.from_account = swap_source_token.key();
    hop_accounts.to_account = swap_destination_token.key();

    Ok(actual_amount_out)
}

fn enforce_sa_token_allowlist(
    account_infos: &[AccountInfo],
    allowed_sa_token_accounts: &[Pubkey],
) -> Result<()> {
    for ai in account_infos.iter() {
        // Only consider token accounts (spl-token or token-2022)
        if ai.owner == &anchor_spl::token::Token::id() {
            if let Ok(data) = ai.try_borrow_data() {
                if data.len() >= 165 {
                    if let Ok(ta) = SplTokenAccount::unpack(&data) {
                        if ta.owner == authority_pda::ID
                            && !allowed_sa_token_accounts.contains(ai.key)
                        {
                            return Err(ErrorCode::UnexpectedSaTokenAccount.into());
                        }
                    }
                }
            }
        } else if ai.owner == &anchor_spl::token_2022::Token2022::id() {
            if let Ok(data) = ai.try_borrow_data() {
                if data.len() >= 165 {
                    if let Ok(ta) = SplToken2022Account::unpack_from_slice(&data) {
                        if ta.owner == authority_pda::ID
                            && !allowed_sa_token_accounts.contains(ai.key)
                        {
                            return Err(ErrorCode::UnexpectedSaTokenAccount.into());
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
