//! permissionlessLST router adapter (eat.ag).
//!
//! Routes wrapped SOL into and out of any public SPL-family stake pool (SPL Stake Pool,
//! Sanctum SPL, Sanctum SPL Multi) through the permissionless LST router program
//! `9XPcSKi9zHn2eqZGUFEDpPC5PBF1oAu3NhyzTrkoe3h3`. The router bridges wSOL to native SOL,
//! calls the pool's `DepositSolWithSlippage` / `WithdrawSolWithSlippage`, and supports both
//! legacy SPL and Token-2022 LST mints (including transfer-fee mints), which the legacy
//! `stkitr…` Sanctum router cannot do. No registry, curator listing or admin signature is
//! required: any pool owned by one of the pinned stake-pool programs is routable.
//!
//! Instruction tags on the router program:
//!   34 = mint  (wSOL -> LST): `[34, amount_in: u64 LE, minimum_out: u64 LE]`
//!   35 = redeem (LST -> wSOL): `[35, amount_in: u64 LE, minimum_out: u64 LE]`
//! The router measures the destination credit after the pool CPI and enforces `minimum_out`
//! itself; this adapter passes `1` and relies on the aggregator's route-level min-out.
use super::common::DexProcessor;
use crate::HopAccounts;
use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{
    permissionless_lst_program, sanctum_spl_multi_stake_pool_program,
    sanctum_spl_stake_pool_program, spl_stake_pool_program,
};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token_interface::TokenAccount;
use arrayref::array_ref;

const MINT_TAG: u8 = 34;
const REDEEM_TAG: u8 = 35;
const ARGS_LEN: usize = 17;
/// The router enforces `minimum_out > 0`; the aggregator's own min-out check governs.
const ROUTER_MINIMUM_OUT: u64 = 1;

pub struct PermissionlessLstProcessor;
impl DexProcessor for PermissionlessLstProcessor {}

fn is_stake_pool_program(key: &Pubkey) -> bool {
    [
        spl_stake_pool_program::id(),
        sanctum_spl_stake_pool_program::id(),
        sanctum_spl_multi_stake_pool_program::id(),
    ]
    .contains(key)
}

fn args(tag: u8, amount_in: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(ARGS_LEN);
    data.push(tag);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&ROUTER_MINIMUM_OUT.to_le_bytes());
    data
}

/// wSOL -> LST. The swap authority is both the router payer and the token authority; the
/// temporary wSOL bridge rent it fronts is refunded inside the same instruction.
pub struct MintAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,
    /// wSOL mint (legacy `So111…112` or Token-2022 `9pan…Xejp`).
    pub wsol_mint: &'info AccountInfo<'info>,
    /// LST mint; writable because the pool mints to the destination.
    pub lst_mint: &'info AccountInfo<'info>,
    /// Referral token account for the LST mint. Use the router's `["fee", lst_mint]` PDA
    /// (created permissionlessly with tag 33) or any initialized LST token account.
    pub referral_token_account: &'info AccountInfo<'info>,
    pub native_token_program: &'info AccountInfo<'info>,
    pub lst_token_program: &'info AccountInfo<'info>,
    pub system_program: &'info AccountInfo<'info>,
    pub stake_pool_program: &'info AccountInfo<'info>,
    pub stake_pool: &'info AccountInfo<'info>,
    /// `[stake_pool, "withdraw"]` under the stake-pool program.
    pub withdraw_authority: &'info AccountInfo<'info>,
    pub reserve_stake: &'info AccountInfo<'info>,
    pub manager_fee_account: &'info AccountInfo<'info>,
    /// `["sol", swap_authority]` under the router program (system account).
    pub sol_bridge: &'info AccountInfo<'info>,
    /// `["wsol", swap_authority, wsol_mint]` under the router program (created and closed per swap).
    pub wsol_bridge: &'info AccountInfo<'info>,
}
pub const MINT_ACCOUNTS_LEN: usize = 17;

impl<'info> MintAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            wsol_mint,
            lst_mint,
            referral_token_account,
            native_token_program,
            lst_token_program,
            system_program,
            stake_pool_program,
            stake_pool,
            withdraw_authority,
            reserve_stake,
            manager_fee_account,
            sol_bridge,
            wsol_bridge,
        ]: &[AccountInfo<'info>; MINT_ACCOUNTS_LEN] = array_ref![accounts, offset, MINT_ACCOUNTS_LEN];
        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            wsol_mint,
            lst_mint,
            referral_token_account,
            native_token_program,
            lst_token_program,
            system_program,
            stake_pool_program,
            stake_pool,
            withdraw_authority,
            reserve_stake,
            manager_fee_account,
            sol_bridge,
            wsol_bridge,
        })
    }
}

pub fn mint<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::PermissionlessLstMint amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + MINT_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );
    let mut swap_accounts = MintAccounts::parse_accounts(remaining_accounts, *offset)?;
    require_keys_eq!(
        swap_accounts.dex_program_id.key(),
        permissionless_lst_program::id(),
        ErrorCode::InvalidProgramId
    );
    require!(
        is_stake_pool_program(swap_accounts.stake_pool_program.key),
        ErrorCode::InvalidProgramId
    );
    require_keys_eq!(
        swap_accounts.swap_source_token.mint,
        swap_accounts.wsol_mint.key(),
        ErrorCode::InvalidPool
    );
    require_keys_eq!(
        swap_accounts.swap_destination_token.mint,
        swap_accounts.lst_mint.key(),
        ErrorCode::InvalidPool
    );
    // log pool address
    swap_accounts.stake_pool.key().log();

    let swap_source_token = swap_accounts.swap_source_token.key();
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

    let authority = swap_accounts.swap_authority_pubkey.key();
    let accounts = vec![
        AccountMeta::new(authority, true),
        AccountMeta::new_readonly(authority, true),
        AccountMeta::new(swap_source_token, false),
        AccountMeta::new(swap_destination_token, false),
        AccountMeta::new_readonly(swap_accounts.wsol_mint.key(), false),
        AccountMeta::new(swap_accounts.lst_mint.key(), false),
        AccountMeta::new(swap_accounts.referral_token_account.key(), false),
        AccountMeta::new_readonly(swap_accounts.native_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.lst_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.system_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.stake_pool_program.key(), false),
        AccountMeta::new(swap_accounts.stake_pool.key(), false),
        AccountMeta::new_readonly(swap_accounts.withdraw_authority.key(), false),
        AccountMeta::new(swap_accounts.reserve_stake.key(), false),
        AccountMeta::new(swap_accounts.manager_fee_account.key(), false),
        AccountMeta::new(swap_accounts.sol_bridge.key(), false),
        AccountMeta::new(swap_accounts.wsol_bridge.key(), false),
    ];
    let account_infos = [
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.wsol_mint.to_account_info(),
        swap_accounts.lst_mint.to_account_info(),
        swap_accounts.referral_token_account.to_account_info(),
        swap_accounts.native_token_program.to_account_info(),
        swap_accounts.lst_token_program.to_account_info(),
        swap_accounts.system_program.to_account_info(),
        swap_accounts.stake_pool_program.to_account_info(),
        swap_accounts.stake_pool.to_account_info(),
        swap_accounts.withdraw_authority.to_account_info(),
        swap_accounts.reserve_stake.to_account_info(),
        swap_accounts.manager_fee_account.to_account_info(),
        swap_accounts.sol_bridge.to_account_info(),
        swap_accounts.wsol_bridge.to_account_info(),
    ];
    let instruction = Instruction {
        program_id: swap_accounts.dex_program_id.key(),
        accounts,
        data: args(MINT_TAG, amount_in),
    };

    invoke_process(
        amount_in,
        &PermissionlessLstProcessor,
        &account_infos,
        &mut swap_accounts.swap_source_token,
        &mut swap_accounts.swap_destination_token,
        hop_accounts,
        instruction,
        hop,
        offset,
        MINT_ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )
}

/// LST -> wSOL through the pool reserve (`WithdrawSolWithSlippage`).
pub struct RedeemAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,
    pub wsol_mint: &'info AccountInfo<'info>,
    /// LST mint; writable because the pool burns from the source.
    pub lst_mint: &'info AccountInfo<'info>,
    pub lst_token_program: &'info AccountInfo<'info>,
    pub native_token_program: &'info AccountInfo<'info>,
    pub stake_pool_program: &'info AccountInfo<'info>,
    pub stake_pool: &'info AccountInfo<'info>,
    pub withdraw_authority: &'info AccountInfo<'info>,
    pub reserve_stake: &'info AccountInfo<'info>,
    pub manager_fee_account: &'info AccountInfo<'info>,
    pub clock: &'info AccountInfo<'info>,
    pub stake_history: &'info AccountInfo<'info>,
    pub stake_program: &'info AccountInfo<'info>,
}
pub const REDEEM_ACCOUNTS_LEN: usize = 16;

impl<'info> RedeemAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            wsol_mint,
            lst_mint,
            lst_token_program,
            native_token_program,
            stake_pool_program,
            stake_pool,
            withdraw_authority,
            reserve_stake,
            manager_fee_account,
            clock,
            stake_history,
            stake_program,
        ]: &[AccountInfo<'info>; REDEEM_ACCOUNTS_LEN] =
            array_ref![accounts, offset, REDEEM_ACCOUNTS_LEN];
        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            wsol_mint,
            lst_mint,
            lst_token_program,
            native_token_program,
            stake_pool_program,
            stake_pool,
            withdraw_authority,
            reserve_stake,
            manager_fee_account,
            clock,
            stake_history,
            stake_program,
        })
    }
}

pub fn redeem<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::PermissionlessLstRedeem amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + REDEEM_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );
    let mut swap_accounts = RedeemAccounts::parse_accounts(remaining_accounts, *offset)?;
    require_keys_eq!(
        swap_accounts.dex_program_id.key(),
        permissionless_lst_program::id(),
        ErrorCode::InvalidProgramId
    );
    require!(
        is_stake_pool_program(swap_accounts.stake_pool_program.key),
        ErrorCode::InvalidProgramId
    );
    require_keys_eq!(
        swap_accounts.swap_source_token.mint,
        swap_accounts.lst_mint.key(),
        ErrorCode::InvalidPool
    );
    require_keys_eq!(
        swap_accounts.swap_destination_token.mint,
        swap_accounts.wsol_mint.key(),
        ErrorCode::InvalidPool
    );
    // log pool address
    swap_accounts.stake_pool.key().log();

    let swap_source_token = swap_accounts.swap_source_token.key();
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
        AccountMeta::new_readonly(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new(swap_source_token, false),
        AccountMeta::new(swap_destination_token, false),
        AccountMeta::new_readonly(swap_accounts.wsol_mint.key(), false),
        AccountMeta::new(swap_accounts.lst_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.lst_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.native_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.stake_pool_program.key(), false),
        AccountMeta::new(swap_accounts.stake_pool.key(), false),
        AccountMeta::new_readonly(swap_accounts.withdraw_authority.key(), false),
        AccountMeta::new(swap_accounts.reserve_stake.key(), false),
        AccountMeta::new(swap_accounts.manager_fee_account.key(), false),
        AccountMeta::new_readonly(swap_accounts.clock.key(), false),
        AccountMeta::new_readonly(swap_accounts.stake_history.key(), false),
        AccountMeta::new_readonly(swap_accounts.stake_program.key(), false),
    ];
    let account_infos = [
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.wsol_mint.to_account_info(),
        swap_accounts.lst_mint.to_account_info(),
        swap_accounts.lst_token_program.to_account_info(),
        swap_accounts.native_token_program.to_account_info(),
        swap_accounts.stake_pool_program.to_account_info(),
        swap_accounts.stake_pool.to_account_info(),
        swap_accounts.withdraw_authority.to_account_info(),
        swap_accounts.reserve_stake.to_account_info(),
        swap_accounts.manager_fee_account.to_account_info(),
        swap_accounts.clock.to_account_info(),
        swap_accounts.stake_history.to_account_info(),
        swap_accounts.stake_program.to_account_info(),
    ];
    let instruction = Instruction {
        program_id: swap_accounts.dex_program_id.key(),
        accounts,
        data: args(REDEEM_TAG, amount_in),
    };

    invoke_process(
        amount_in,
        &PermissionlessLstProcessor,
        &account_infos,
        &mut swap_accounts.swap_source_token,
        &mut swap_accounts.swap_destination_token,
        hop_accounts,
        instruction,
        hop,
        offset,
        REDEEM_ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_mint_and_redeem_args() {
        let mint_data = args(MINT_TAG, 1_000_000_000);
        assert_eq!(mint_data.len(), ARGS_LEN);
        assert_eq!(mint_data[0], 34);
        assert_eq!(u64::from_le_bytes(mint_data[1..9].try_into().unwrap()), 1_000_000_000);
        assert_eq!(u64::from_le_bytes(mint_data[9..17].try_into().unwrap()), 1);
        let redeem_data = args(REDEEM_TAG, 7);
        assert_eq!(redeem_data[0], 35);
        assert_eq!(u64::from_le_bytes(redeem_data[1..9].try_into().unwrap()), 7);
    }

    #[test]
    fn stake_pool_programs_are_pinned() {
        assert!(is_stake_pool_program(&spl_stake_pool_program::id()));
        assert!(is_stake_pool_program(&sanctum_spl_stake_pool_program::id()));
        assert!(is_stake_pool_program(&sanctum_spl_multi_stake_pool_program::id()));
        assert!(!is_stake_pool_program(&permissionless_lst_program::id()));
    }
}
