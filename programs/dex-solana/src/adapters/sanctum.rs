use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{HopAccounts, sanctum_program, wsol_program};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token::Token;
use anchor_spl::token_interface::{Mint, TokenAccount};
use arrayref::array_ref;
use bytemuck::{AnyBitPattern, Pod, Zeroable};

use super::common::DexProcessor;

pub struct SanctumProcessor;
impl DexProcessor for SanctumProcessor {}

const ADD_LST_LIQ_ACCOUNTS_LEN: usize = 18;
const ADD_WSOL_LIQ_ACCOUNTS_LEN: usize = 14;
const ADD_REMOVE_LIQ_ARGS_LEN: usize = 1 //discriminant
                            + 1 //lst_value_calc_accs
                            + 4 //lst_index
                            + 8 //lst_amount
                            + 8; //min_lp_out
const SWAP_EXACT_IN_ARGS_LEN: usize = 1 //discriminant
                            + 1 //src_lst_value_calc_accs
                            + 1 //dst_lst_value_calc_accs
                            + 4 //src_lst_index
                            + 4 //dst_lst_index
                            + 8 //min_lp_out
                            + 8; //lst_amount

const REMOVE_LST_LIQ_ACCOUNTS_LEN: usize = 19;
const REMOVE_WSOL_LIQ_ACCOUNTS_LEN: usize = 15;
const SWAP_LST_SOL_ACCOUNTS_LEN: usize = 22;
const SWAP_LST_LST_ACCOUNTS_LEN: usize = 26;

pub enum SanctumRemoveLiqAccounts<'info> {
    WSOL(SanctumRemoveWsolLiqAccounts<'info>),
    LST(SanctumRemoveLstLiqAccounts<'info>),
}
pub enum SanctumSwapAccounts<'info> {
    LstWsol(SanctumLstWsolSwapAccounts<'info>),
    WsolLst(SanctumWsolLstSwapAccounts<'info>),
    LstLst(SanctumLstLstSwapAccounts<'info>),
}

pub enum SanctumAddLiqAccounts<'info> {
    WSOL(SanctumAddWsolLiqAccounts<'info>),
    LST(SanctumAddLstLiqAccounts<'info>),
}

pub struct SanctumAddLstLiqAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub lst_mint: Box<InterfaceAccount<'info, Mint>>,
    pub source_lst_acc: Box<InterfaceAccount<'info, TokenAccount>>,
    pub dst_lp_acc: Box<InterfaceAccount<'info, TokenAccount>>,
    pub lp_token_mint: Box<InterfaceAccount<'info, Mint>>,
    pub protocol_fee_accumulator: Box<InterfaceAccount<'info, TokenAccount>>,
    pub lst_token_program: Program<'info, Token>,
    pub lp_token_program: Program<'info, Token>,
    pub pool_state: &'info AccountInfo<'info>,
    pub lst_states_list: &'info AccountInfo<'info>,
    pub pool_reserves: &'info AccountInfo<'info>,
    pub spl_sol_calculator: &'info AccountInfo<'info>,
    pub calculator_state: &'info AccountInfo<'info>,
    pub staked_pool_state: &'info AccountInfo<'info>,
    pub validator_pool_program: &'info AccountInfo<'info>,
    pub validator_pool_program_data: &'info AccountInfo<'info>,
    pub sanctum_flat_fee_pricing: &'info AccountInfo<'info>,
}

pub struct SanctumAddWsolLiqAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub lst_mint: Box<InterfaceAccount<'info, Mint>>,
    pub source_lst_acc: Box<InterfaceAccount<'info, TokenAccount>>,
    pub dst_lp_acc: Box<InterfaceAccount<'info, TokenAccount>>,
    pub lp_token_mint: Box<InterfaceAccount<'info, Mint>>,
    pub protocol_fee_accumulator: Box<InterfaceAccount<'info, TokenAccount>>,
    pub lst_token_program: Program<'info, Token>,
    pub lp_token_program: Program<'info, Token>,
    pub pool_state: &'info AccountInfo<'info>,
    pub lst_states_list: &'info AccountInfo<'info>,
    pub pool_reserves: &'info AccountInfo<'info>,
    pub wsol_calculator: &'info AccountInfo<'info>,
    pub sanctum_flat_fee_pricing: &'info AccountInfo<'info>,
}

pub struct SanctumRemoveLstLiqAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub lst_mint: Box<InterfaceAccount<'info, Mint>>,
    pub dst_lst_acc: Box<InterfaceAccount<'info, TokenAccount>>,
    pub source_lp_acc: Box<InterfaceAccount<'info, TokenAccount>>,
    pub lp_token_mint: Box<InterfaceAccount<'info, Mint>>,
    pub protocol_fee_accumulator: Box<InterfaceAccount<'info, TokenAccount>>,
    pub lst_token_program: Program<'info, Token>,
    pub lp_token_program: Program<'info, Token>,
    pub pool_state: &'info AccountInfo<'info>,
    pub lst_states_list: &'info AccountInfo<'info>,
    pub pool_reserves: &'info AccountInfo<'info>,
    pub spl_sol_calculator: &'info AccountInfo<'info>,
    pub calculator_state: &'info AccountInfo<'info>,
    pub staked_pool_state: &'info AccountInfo<'info>,
    pub validator_pool_program: &'info AccountInfo<'info>,
    pub validator_pool_program_data: &'info AccountInfo<'info>,
    pub sanctum_flat_fee_pricing: &'info AccountInfo<'info>,
    pub sanctum_flat_fee_pricing_account: &'info AccountInfo<'info>,
}

pub struct SanctumRemoveWsolLiqAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub lst_mint: Box<InterfaceAccount<'info, Mint>>,
    pub dst_lst_acc: Box<InterfaceAccount<'info, TokenAccount>>,
    pub source_lp_acc: Box<InterfaceAccount<'info, TokenAccount>>,
    pub lp_token_mint: Box<InterfaceAccount<'info, Mint>>,
    pub protocol_fee_accumulator: Box<InterfaceAccount<'info, TokenAccount>>,
    pub lst_token_program: Program<'info, Token>,
    pub lp_token_program: Program<'info, Token>,
    pub pool_state: &'info AccountInfo<'info>,
    pub lst_states_list: &'info AccountInfo<'info>,
    pub pool_reserves: &'info AccountInfo<'info>,
    pub wsol_calculator: &'info AccountInfo<'info>,
    pub sanctum_flat_fee_pricing: &'info AccountInfo<'info>,
    pub sanctum_flat_fee_pricing_account: &'info AccountInfo<'info>,
}

pub struct SanctumWsolLstSwapAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub source_lst_mint: Box<InterfaceAccount<'info, Mint>>,
    pub dst_lst_mint: Box<InterfaceAccount<'info, Mint>>,
    pub source_lst_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub dst_lst_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub protocol_fee_accumulator: Box<InterfaceAccount<'info, TokenAccount>>,
    pub source_token_program: Program<'info, Token>,
    pub dst_token_program: Program<'info, Token>,
    pub pool_state: &'info AccountInfo<'info>,
    pub lst_states_list: &'info AccountInfo<'info>,
    pub source_pool_reserves: &'info AccountInfo<'info>,
    pub dst_pool_reserves: &'info AccountInfo<'info>,
    pub wsol_calculator: &'info AccountInfo<'info>,
    pub spl_sol_calculator: &'info AccountInfo<'info>,
    pub calculator_state: &'info AccountInfo<'info>,
    pub staked_pool_state: &'info AccountInfo<'info>,
    pub validator_pool_program: &'info AccountInfo<'info>,
    pub validator_pool_program_data: &'info AccountInfo<'info>,
    pub sanctum_flat_fee_pricing: &'info AccountInfo<'info>,
    pub sanctum_src_flat_fee_pricing_account: &'info AccountInfo<'info>,
    pub sanctum_dst_flat_fee_pricing_account: &'info AccountInfo<'info>,
}

pub struct SanctumLstWsolSwapAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub source_lst_mint: Box<InterfaceAccount<'info, Mint>>,
    pub dst_lst_mint: Box<InterfaceAccount<'info, Mint>>,
    pub source_lst_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub dst_lst_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub protocol_fee_accumulator: Box<InterfaceAccount<'info, TokenAccount>>,
    pub source_token_program: Program<'info, Token>,
    pub dst_token_program: Program<'info, Token>,
    pub pool_state: &'info AccountInfo<'info>,
    pub lst_states_list: &'info AccountInfo<'info>,
    pub source_pool_reserves: &'info AccountInfo<'info>,
    pub dst_pool_reserves: &'info AccountInfo<'info>,
    pub spl_sol_calculator: &'info AccountInfo<'info>,
    pub calculator_state: &'info AccountInfo<'info>,
    pub staked_pool_state: &'info AccountInfo<'info>,
    pub validator_pool_program: &'info AccountInfo<'info>,
    pub validator_pool_program_data: &'info AccountInfo<'info>,
    pub wsol_calculator: &'info AccountInfo<'info>,
    pub sanctum_flat_fee_pricing: &'info AccountInfo<'info>,
    pub sanctum_src_flat_fee_pricing_account: &'info AccountInfo<'info>,
    pub sanctum_dst_flat_fee_pricing_account: &'info AccountInfo<'info>,
}

pub struct SanctumLstLstSwapAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub source_lst_mint: Box<InterfaceAccount<'info, Mint>>,
    pub dst_lst_mint: Box<InterfaceAccount<'info, Mint>>,
    pub source_lst_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub dst_lst_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub protocol_fee_accumulator: Box<InterfaceAccount<'info, TokenAccount>>,
    pub source_token_program: Program<'info, Token>,
    pub dst_token_program: Program<'info, Token>,
    pub pool_state: &'info AccountInfo<'info>,
    pub lst_states_list: &'info AccountInfo<'info>,
    pub source_pool_reserves: &'info AccountInfo<'info>,
    pub dst_pool_reserves: &'info AccountInfo<'info>,
    pub src_spl_sol_calculator: &'info AccountInfo<'info>,
    pub src_calculator_state: &'info AccountInfo<'info>,
    pub src_staked_pool_state: &'info AccountInfo<'info>,
    pub src_validator_pool_program: &'info AccountInfo<'info>,
    pub src_validator_pool_program_data: &'info AccountInfo<'info>,
    pub dst_spl_sol_calculator: &'info AccountInfo<'info>,
    pub dst_calculator_state: &'info AccountInfo<'info>,
    pub dst_staked_pool_state: &'info AccountInfo<'info>,
    pub dst_validator_pool_program: &'info AccountInfo<'info>,
    pub dst_validator_pool_program_data: &'info AccountInfo<'info>,
    pub sanctum_flat_fee_pricing: &'info AccountInfo<'info>,
    pub sanctum_src_flat_fee_pricing_account: &'info AccountInfo<'info>,
    pub sanctum_dst_flat_fee_pricing_account: &'info AccountInfo<'info>,
}

impl<'info> SanctumAddLiqAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        //check the mint of the LST account to determine if WSOL
        let is_wsol = accounts[2 + offset].key() == wsol_program::id();
        match is_wsol {
            true => {
                require!(
                    accounts.len() >= offset + ADD_WSOL_LIQ_ACCOUNTS_LEN,
                    ErrorCode::InvalidAccountsLength
                );

                let [
                    dex_program_id,
                    swap_authority_pubkey,
                    lst_mint,
                    source_lst_acc,
                    dst_lp_acc,
                    lp_token_mint,
                    protocol_fee_accumulator,
                    lst_token_program,
                    lp_token_program,
                    pool_state,
                    lst_states_list,
                    pool_reserves,
                    wsol_calculator,
                    sanctum_flat_fee_pricing,
                ]: &[AccountInfo<'info>; ADD_WSOL_LIQ_ACCOUNTS_LEN] =
                    array_ref![accounts, offset, ADD_WSOL_LIQ_ACCOUNTS_LEN];
                Ok(SanctumAddLiqAccounts::WSOL(SanctumAddWsolLiqAccounts {
                    dex_program_id,
                    swap_authority_pubkey,
                    lst_mint: Box::new(InterfaceAccount::try_from(lst_mint)?),
                    source_lst_acc: Box::new(InterfaceAccount::try_from(source_lst_acc)?),
                    lp_token_mint: Box::new(InterfaceAccount::try_from(lp_token_mint)?),
                    dst_lp_acc: Box::new(InterfaceAccount::try_from(dst_lp_acc)?),
                    protocol_fee_accumulator: Box::new(InterfaceAccount::try_from(
                        protocol_fee_accumulator,
                    )?),
                    lst_token_program: Program::try_from(lst_token_program)?,
                    lp_token_program: Program::try_from(lp_token_program)?,
                    pool_state,
                    lst_states_list,
                    pool_reserves,
                    wsol_calculator,
                    sanctum_flat_fee_pricing,
                }))
            }
            false => {
                require!(
                    accounts.len() >= offset + ADD_LST_LIQ_ACCOUNTS_LEN,
                    ErrorCode::InvalidAccountsLength
                );

                let [
                    dex_program_id,
                    swap_authority_pubkey,
                    lst_mint,
                    source_lst_acc,
                    dst_lp_acc,
                    lp_token_mint,
                    protocol_fee_accumulator,
                    lst_token_program,
                    lp_token_program,
                    pool_state,
                    lst_states_list,
                    pool_reserves,
                    spl_sol_calculator,
                    calculator_state,
                    staked_pool_state,
                    validator_pool_program,
                    validator_pool_program_data,
                    sanctum_flat_fee_pricing,
                ]: &[AccountInfo<'info>; ADD_LST_LIQ_ACCOUNTS_LEN] =
                    array_ref![accounts, offset, ADD_LST_LIQ_ACCOUNTS_LEN];
                Ok(SanctumAddLiqAccounts::LST(SanctumAddLstLiqAccounts {
                    dex_program_id,
                    swap_authority_pubkey,
                    lst_mint: Box::new(InterfaceAccount::try_from(lst_mint)?),
                    source_lst_acc: Box::new(InterfaceAccount::try_from(source_lst_acc)?),
                    lp_token_mint: Box::new(InterfaceAccount::try_from(lp_token_mint)?),
                    dst_lp_acc: Box::new(InterfaceAccount::try_from(dst_lp_acc)?),
                    protocol_fee_accumulator: Box::new(InterfaceAccount::try_from(
                        protocol_fee_accumulator,
                    )?),
                    lst_token_program: Program::try_from(lst_token_program)?,
                    lp_token_program: Program::try_from(lp_token_program)?,
                    pool_state,
                    lst_states_list,
                    pool_reserves,
                    spl_sol_calculator,
                    calculator_state,
                    staked_pool_state,
                    validator_pool_program,
                    validator_pool_program_data,
                    sanctum_flat_fee_pricing,
                }))
            }
        }
    }
}

impl<'info> SanctumRemoveLiqAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        //check the mint of the LST account to determine if WSOL
        let is_wsol = accounts[2 + offset].key() == wsol_program::id();
        match is_wsol {
            true => {
                require!(
                    accounts.len() >= offset + REMOVE_WSOL_LIQ_ACCOUNTS_LEN,
                    ErrorCode::InvalidAccountsLength
                );

                let [
                    dex_program_id,
                    swap_authority_pubkey,
                    lst_mint,
                    dst_lst_acc,
                    source_lp_acc,
                    lp_token_mint,
                    protocol_fee_accumulator,
                    lst_token_program,
                    lp_token_program,
                    pool_state,
                    lst_states_list,
                    pool_reserves,
                    wsol_calculator,
                    sanctum_flat_fee_pricing,
                    sanctum_flat_fee_pricing_account,
                ]: &[AccountInfo<'info>; REMOVE_WSOL_LIQ_ACCOUNTS_LEN] =
                    array_ref![accounts, offset, REMOVE_WSOL_LIQ_ACCOUNTS_LEN];
                Ok(SanctumRemoveLiqAccounts::WSOL(SanctumRemoveWsolLiqAccounts {
                    dex_program_id,
                    swap_authority_pubkey,
                    lst_mint: Box::new(InterfaceAccount::try_from(lst_mint)?),
                    dst_lst_acc: Box::new(InterfaceAccount::try_from(dst_lst_acc)?),
                    source_lp_acc: Box::new(InterfaceAccount::try_from(source_lp_acc)?),
                    lp_token_mint: Box::new(InterfaceAccount::try_from(lp_token_mint)?),
                    protocol_fee_accumulator: Box::new(InterfaceAccount::try_from(
                        protocol_fee_accumulator,
                    )?),
                    lst_token_program: Program::try_from(lst_token_program)?,
                    lp_token_program: Program::try_from(lp_token_program)?,
                    pool_state,
                    lst_states_list,
                    pool_reserves,
                    wsol_calculator,
                    sanctum_flat_fee_pricing,
                    sanctum_flat_fee_pricing_account,
                }))
            }
            false => {
                require!(
                    accounts.len() >= offset + REMOVE_LST_LIQ_ACCOUNTS_LEN,
                    ErrorCode::InvalidAccountsLength
                );

                let [
                    dex_program_id,
                    swap_authority_pubkey,
                    lst_mint,
                    dst_lst_acc,
                    source_lp_acc,
                    lp_token_mint,
                    protocol_fee_accumulator,
                    lst_token_program,
                    lp_token_program,
                    pool_state,
                    lst_states_list,
                    pool_reserves,
                    spl_sol_calculator,
                    calculator_state,
                    staked_pool_state,
                    validator_pool_program,
                    validator_pool_program_data,
                    sanctum_flat_fee_pricing,
                    sanctum_flat_fee_pricing_account,
                ]: &[AccountInfo<'info>; REMOVE_LST_LIQ_ACCOUNTS_LEN] =
                    array_ref![accounts, offset, REMOVE_LST_LIQ_ACCOUNTS_LEN];
                Ok(SanctumRemoveLiqAccounts::LST(SanctumRemoveLstLiqAccounts {
                    dex_program_id,
                    swap_authority_pubkey,
                    lst_mint: Box::new(InterfaceAccount::try_from(lst_mint)?),
                    dst_lst_acc: Box::new(InterfaceAccount::try_from(dst_lst_acc)?),
                    source_lp_acc: Box::new(InterfaceAccount::try_from(source_lp_acc)?),
                    lp_token_mint: Box::new(InterfaceAccount::try_from(lp_token_mint)?),
                    protocol_fee_accumulator: Box::new(InterfaceAccount::try_from(
                        protocol_fee_accumulator,
                    )?),
                    lst_token_program: Program::try_from(lst_token_program)?,
                    lp_token_program: Program::try_from(lp_token_program)?,
                    pool_state,
                    lst_states_list,
                    pool_reserves,
                    spl_sol_calculator,
                    calculator_state,
                    staked_pool_state,
                    validator_pool_program,
                    validator_pool_program_data,
                    sanctum_flat_fee_pricing,
                    sanctum_flat_fee_pricing_account,
                }))
            }
        }
    }
}

impl<'info> SanctumSwapAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let is_src_wsol = accounts[2 + offset].key() == wsol_program::id();
        let is_dst_wsol = accounts[3 + offset].key() == wsol_program::id();
        match (is_src_wsol, is_dst_wsol) {
            (false, false) => {
                //LstLst swap accounts
                require!(
                    accounts.len() >= offset + SWAP_LST_LST_ACCOUNTS_LEN,
                    ErrorCode::InvalidAccountsLength
                );

                let [
                    dex_program_id,
                    swap_authority_pubkey,
                    source_lst_mint,
                    dst_lst_mint,
                    source_lst_account,
                    dst_lst_account,
                    protocol_fee_accumulator,
                    source_token_program,
                    dst_token_program,
                    pool_state,
                    lst_states_list,
                    source_pool_reserves,
                    dst_pool_reserves,
                    src_spl_sol_calculator,
                    src_calculator_state,
                    src_staked_pool_state,
                    src_validator_pool_program,
                    src_validator_pool_program_data,
                    dst_spl_sol_calculator,
                    dst_calculator_state,
                    dst_staked_pool_state,
                    dst_validator_pool_program,
                    dst_validator_pool_program_data,
                    sanctum_flat_fee_pricing,
                    sanctum_src_flat_fee_pricing_account,
                    sanctum_dst_flat_fee_pricing_account,
                ]: &[AccountInfo<'info>; SWAP_LST_LST_ACCOUNTS_LEN] =
                    array_ref![accounts, offset, SWAP_LST_LST_ACCOUNTS_LEN];
                Ok(SanctumSwapAccounts::LstLst(SanctumLstLstSwapAccounts {
                    dex_program_id,
                    swap_authority_pubkey,
                    source_lst_mint: Box::new(InterfaceAccount::try_from(source_lst_mint)?),
                    dst_lst_mint: Box::new(InterfaceAccount::try_from(dst_lst_mint)?),
                    source_lst_account: Box::new(InterfaceAccount::try_from(source_lst_account)?),
                    dst_lst_account: Box::new(InterfaceAccount::try_from(dst_lst_account)?),
                    protocol_fee_accumulator: Box::new(InterfaceAccount::try_from(
                        protocol_fee_accumulator,
                    )?),
                    source_token_program: Program::try_from(source_token_program)?,
                    dst_token_program: Program::try_from(dst_token_program)?,
                    pool_state,
                    lst_states_list,
                    source_pool_reserves,
                    dst_pool_reserves,
                    src_spl_sol_calculator,
                    src_calculator_state,
                    src_staked_pool_state,
                    src_validator_pool_program,
                    src_validator_pool_program_data,
                    dst_spl_sol_calculator,
                    dst_calculator_state,
                    dst_staked_pool_state,
                    dst_validator_pool_program,
                    dst_validator_pool_program_data,
                    sanctum_flat_fee_pricing,
                    sanctum_src_flat_fee_pricing_account,
                    sanctum_dst_flat_fee_pricing_account,
                }))
            }
            (true, false) => {
                //WSOL -> LST swap accounts
                require!(
                    accounts.len() >= offset + SWAP_LST_SOL_ACCOUNTS_LEN,
                    ErrorCode::InvalidAccountsLength
                );

                let [
                    dex_program_id,
                    swap_authority_pubkey,
                    source_lst_mint,
                    dst_lst_mint,
                    source_lst_account,
                    dst_lst_account,
                    protocol_fee_accumulator,
                    source_token_program,
                    dst_token_program,
                    pool_state,
                    lst_states_list,
                    source_pool_reserves,
                    dst_pool_reserves,
                    wsol_calculator,
                    spl_sol_calculator,
                    calculator_state,
                    staked_pool_state,
                    validator_pool_program,
                    validator_pool_program_data,
                    sanctum_flat_fee_pricing,
                    sanctum_src_flat_fee_pricing_account,
                    sanctum_dst_flat_fee_pricing_account,
                ]: &[AccountInfo<'info>; SWAP_LST_SOL_ACCOUNTS_LEN] =
                    array_ref![accounts, offset, SWAP_LST_SOL_ACCOUNTS_LEN];
                Ok(SanctumSwapAccounts::WsolLst(SanctumWsolLstSwapAccounts {
                    dex_program_id,
                    swap_authority_pubkey,
                    source_lst_mint: Box::new(InterfaceAccount::try_from(source_lst_mint)?),
                    dst_lst_mint: Box::new(InterfaceAccount::try_from(dst_lst_mint)?),
                    source_lst_account: Box::new(InterfaceAccount::try_from(source_lst_account)?),
                    dst_lst_account: Box::new(InterfaceAccount::try_from(dst_lst_account)?),
                    protocol_fee_accumulator: Box::new(InterfaceAccount::try_from(
                        protocol_fee_accumulator,
                    )?),
                    source_token_program: Program::try_from(source_token_program)?,
                    dst_token_program: Program::try_from(dst_token_program)?,
                    pool_state,
                    lst_states_list,
                    source_pool_reserves,
                    dst_pool_reserves,
                    wsol_calculator,
                    spl_sol_calculator,
                    calculator_state,
                    staked_pool_state,
                    validator_pool_program,
                    validator_pool_program_data,
                    sanctum_flat_fee_pricing,
                    sanctum_src_flat_fee_pricing_account,
                    sanctum_dst_flat_fee_pricing_account,
                }))
            }
            (false, true) => {
                // LST -> WSOL swap accounts
                require!(
                    accounts.len() >= offset + SWAP_LST_SOL_ACCOUNTS_LEN,
                    ErrorCode::InvalidAccountsLength
                );

                let [
                    dex_program_id,
                    swap_authority_pubkey,
                    source_lst_mint,
                    dst_lst_mint,
                    source_lst_account,
                    dst_lst_account,
                    protocol_fee_accumulator,
                    source_token_program,
                    dst_token_program,
                    pool_state,
                    lst_states_list,
                    source_pool_reserves,
                    dst_pool_reserves,
                    spl_sol_calculator,
                    calculator_state,
                    staked_pool_state,
                    validator_pool_program,
                    validator_pool_program_data,
                    wsol_calculator,
                    sanctum_flat_fee_pricing,
                    sanctum_src_flat_fee_pricing_account,
                    sanctum_dst_flat_fee_pricing_account,
                ]: &[AccountInfo<'info>; SWAP_LST_SOL_ACCOUNTS_LEN] =
                    array_ref![accounts, offset, SWAP_LST_SOL_ACCOUNTS_LEN];
                Ok(SanctumSwapAccounts::LstWsol(SanctumLstWsolSwapAccounts {
                    dex_program_id,
                    swap_authority_pubkey,
                    source_lst_mint: Box::new(InterfaceAccount::try_from(source_lst_mint)?),
                    dst_lst_mint: Box::new(InterfaceAccount::try_from(dst_lst_mint)?),
                    source_lst_account: Box::new(InterfaceAccount::try_from(source_lst_account)?),
                    dst_lst_account: Box::new(InterfaceAccount::try_from(dst_lst_account)?),
                    protocol_fee_accumulator: Box::new(InterfaceAccount::try_from(
                        protocol_fee_accumulator,
                    )?),
                    source_token_program: Program::try_from(source_token_program)?,
                    dst_token_program: Program::try_from(dst_token_program)?,
                    pool_state,
                    lst_states_list,
                    source_pool_reserves,
                    dst_pool_reserves,
                    spl_sol_calculator,
                    calculator_state,
                    staked_pool_state,
                    validator_pool_program,
                    validator_pool_program_data,
                    wsol_calculator,
                    sanctum_flat_fee_pricing,
                    sanctum_src_flat_fee_pricing_account,
                    sanctum_dst_flat_fee_pricing_account,
                }))
            }
            (true, true) => {
                return Err(ErrorCode::InvalidSanctumSwapAccounts.into());
            }
        }
    }
}

pub fn add_liquidity_handler<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::SanctumAddLiq amount_in: {}, offset: {}", amount_in, offset);
    //account length check moved inside of parse method
    let swap_accounts = SanctumAddLiqAccounts::parse_accounts(remaining_accounts, *offset)?;

    let amount_out: u64 = match swap_accounts {
        SanctumAddLiqAccounts::WSOL(mut swap_accounts) => {
            // log pool address
            swap_accounts.pool_state.key().log();
            handle_prechecks(&swap_accounts, hop_accounts, hop, proxy_swap, owner_seeds)?;

            let lst_index =
                try_get_lst_index(swap_accounts.lst_mint.key(), &swap_accounts.lst_states_list)?;
            let mut ix_data = Vec::with_capacity(ADD_REMOVE_LIQ_ARGS_LEN);
            ix_data.extend_from_slice(&3u8.to_le_bytes()); //add liquidity discriminant
            ix_data.extend_from_slice(&1u8.to_le_bytes()); //lstValueCalcAccs
            ix_data.extend_from_slice(&lst_index.to_le_bytes()); //lst index
            ix_data.extend_from_slice(&amount_in.to_le_bytes()); //lst amount
            ix_data.extend_from_slice(&1u64.to_le_bytes()); //min lp out

            let accounts = swap_accounts.get_accountmetas();
            let account_infos = swap_accounts.get_account_infos();
            let (source_token_account, dst_token_account) = swap_accounts.get_token_accounts_mut();

            let dex_processor = &SanctumProcessor;
            invoke_process(
                amount_in,
                dex_processor,
                &account_infos,
                source_token_account,
                dst_token_account,
                hop_accounts,
                Instruction { program_id: sanctum_program::id(), accounts, data: ix_data },
                hop,
                offset,
                ADD_WSOL_LIQ_ACCOUNTS_LEN,
                proxy_swap,
                owner_seeds,
            )?
        }
        SanctumAddLiqAccounts::LST(mut swap_accounts) => {
            // log pool address
            swap_accounts.pool_state.key().log();
            handle_prechecks(&swap_accounts, hop_accounts, hop, proxy_swap, owner_seeds)?;

            let lst_index =
                try_get_lst_index(swap_accounts.lst_mint.key(), &swap_accounts.lst_states_list)?;
            let mut ix_data = Vec::with_capacity(ADD_REMOVE_LIQ_ARGS_LEN);
            ix_data.extend_from_slice(&3u8.to_le_bytes()); //add liquidity discriminant
            ix_data.extend_from_slice(&5u8.to_le_bytes()); //lstValueCalcAccs
            ix_data.extend_from_slice(&lst_index.to_le_bytes()); //lst index
            ix_data.extend_from_slice(&amount_in.to_le_bytes()); //lst amount
            ix_data.extend_from_slice(&1u64.to_le_bytes()); //min lp out

            let accounts = swap_accounts.get_accountmetas();
            let account_infos = swap_accounts.get_account_infos();
            let (source_token_account, dst_token_account) = swap_accounts.get_token_accounts_mut();

            let dex_processor = &SanctumProcessor;
            invoke_process(
                amount_in,
                dex_processor,
                &account_infos,
                source_token_account,
                dst_token_account,
                hop_accounts,
                Instruction { program_id: sanctum_program::id(), accounts, data: ix_data },
                hop,
                offset,
                ADD_LST_LIQ_ACCOUNTS_LEN,
                proxy_swap,
                owner_seeds,
            )?
        }
    };

    Ok(amount_out)
}

pub fn remove_liquidity_handler<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::SanctumRemoveLiq amount_in: {}, offset: {}", amount_in, offset);
    //account length check moved inside of parse method
    let swap_accounts = SanctumRemoveLiqAccounts::parse_accounts(remaining_accounts, *offset)?;

    let amount_out: u64 = match swap_accounts {
        SanctumRemoveLiqAccounts::WSOL(mut swap_accounts) => {
            // log pool address
            swap_accounts.pool_state.key().log();
            handle_prechecks(&swap_accounts, hop_accounts, hop, proxy_swap, owner_seeds)?;

            let lst_index: u32 =
                try_get_lst_index(swap_accounts.lst_mint.key(), &swap_accounts.lst_states_list)?;

            let mut ix_data = Vec::with_capacity(ADD_REMOVE_LIQ_ARGS_LEN);
            ix_data.extend_from_slice(&4u8.to_le_bytes()); //add liquidity discriminant
            ix_data.extend_from_slice(&1u8.to_le_bytes()); //lstValueCalcAccs
            ix_data.extend_from_slice(&lst_index.to_le_bytes()); //lst index
            ix_data.extend_from_slice(&amount_in.to_le_bytes()); //lst amount
            ix_data.extend_from_slice(&1u64.to_le_bytes()); //min lp out

            let accounts = swap_accounts.get_accountmetas();
            let account_infos = swap_accounts.get_account_infos();
            let (source_token_account, dst_token_account) = swap_accounts.get_token_accounts_mut();

            let dex_processor = &SanctumProcessor;
            invoke_process(
                amount_in,
                dex_processor,
                &account_infos,
                source_token_account,
                dst_token_account,
                hop_accounts,
                Instruction { program_id: sanctum_program::id(), accounts, data: ix_data },
                hop,
                offset,
                REMOVE_WSOL_LIQ_ACCOUNTS_LEN,
                proxy_swap,
                owner_seeds,
            )?
        }
        SanctumRemoveLiqAccounts::LST(mut swap_accounts) => {
            // log pool address
            swap_accounts.pool_state.key().log();
            handle_prechecks(&swap_accounts, hop_accounts, hop, proxy_swap, owner_seeds)?;

            let lst_index =
                try_get_lst_index(swap_accounts.lst_mint.key(), &swap_accounts.lst_states_list)?;

            let mut ix_data = Vec::with_capacity(ADD_REMOVE_LIQ_ARGS_LEN);
            ix_data.extend_from_slice(&4u8.to_le_bytes()); //add liquidity discriminant
            ix_data.extend_from_slice(&5u8.to_le_bytes()); //lstValueCalcAccs
            ix_data.extend_from_slice(&lst_index.to_le_bytes()); //lst index
            ix_data.extend_from_slice(&amount_in.to_le_bytes()); //lst amount
            ix_data.extend_from_slice(&1u64.to_le_bytes()); //min lp out

            let accounts = swap_accounts.get_accountmetas();
            let account_infos = swap_accounts.get_account_infos();
            let (source_token_account, dst_token_account) = swap_accounts.get_token_accounts_mut();
            let dex_processor = &SanctumProcessor;
            invoke_process(
                amount_in,
                dex_processor,
                &account_infos,
                source_token_account,
                dst_token_account,
                hop_accounts,
                Instruction { program_id: sanctum_program::id(), accounts, data: ix_data },
                hop,
                offset,
                REMOVE_LST_LIQ_ACCOUNTS_LEN,
                proxy_swap,
                owner_seeds,
            )?
        }
    };
    Ok(amount_out)
}

pub fn swap_with_wsol_handler<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::SanctumSwapWithWsol amount_in: {}, offset: {}", amount_in, offset);
    //account length check moved inside of parse method
    let swap_accounts = SanctumSwapAccounts::parse_accounts(remaining_accounts, *offset)?;

    let amount_out: u64 = match swap_accounts {
        SanctumSwapAccounts::LstWsol(mut swap_accounts) => handle_lst_wsol_swap(
            &mut swap_accounts,
            amount_in,
            offset,
            hop_accounts,
            hop,
            proxy_swap,
            owner_seeds,
        ),
        SanctumSwapAccounts::WsolLst(mut swap_accounts) => handle_wsol_lst_swap(
            &mut swap_accounts,
            amount_in,
            offset,
            hop_accounts,
            hop,
            proxy_swap,
            owner_seeds,
        ),
        SanctumSwapAccounts::LstLst(_swap_accounts) => {
            Err(ErrorCode::InvalidSanctumSwapAccounts.into())
        }
    }?;
    Ok(amount_out)
}

pub fn swap_without_wsol_handler<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::SanctumSwapWithoutWsol amount_in: {}, offset: {}", amount_in, offset);

    //account length check moved inside of parse method
    let swap_accounts = SanctumSwapAccounts::parse_accounts(remaining_accounts, *offset)?;
    let amount_out: u64 = match swap_accounts {
        SanctumSwapAccounts::LstLst(mut swap_accounts) => handle_lst_lst_swap(
            &mut swap_accounts,
            amount_in,
            offset,
            hop_accounts,
            hop,
            proxy_swap,
            owner_seeds,
        ),
        _ => Err(ErrorCode::InvalidSanctumSwapAccounts.into()),
    }?;

    Ok(amount_out)
}

#[repr(C)]
#[derive(Clone, Debug, AnchorDeserialize, AnchorSerialize, PartialEq, Pod, Copy, Zeroable)]
pub struct LstState {
    pub is_input_disabled: u8,
    pub pool_reserves_bump: u8,
    pub protocol_fee_accumulator_bump: u8,
    pub padding: [u8; 5],
    pub sol_value: u64,
    pub mint: Pubkey,
    pub sol_value_calculator: Pubkey,
}

/// Tries to reinterpret `list_acc_data` bytes as a slice.
///
/// `list_acc_data` should only contain data of the items, no headers etc
///
/// Returns None if failed. Could be due to:
/// - `list_acc_data` is not divisible by T's len
/// - `list_acc_data` is not aligned to T's align
fn try_list<T: AnyBitPattern>(list_acc_data: &[u8]) -> Option<&[T]> {
    if list_acc_data.len() % std::mem::size_of::<T>() != 0 {
        return None;
    }
    let ptr = list_acc_data.as_ptr();
    if ptr.align_offset(std::mem::align_of::<T>()) != 0 {
        return None;
    }
    let len = list_acc_data.len() / std::mem::size_of::<T>();
    Some(unsafe { std::slice::from_raw_parts(ptr as *const T, len) })
}

/// Tries to reinterpret `lst_state_list_acc_data` bytes as a LstStateList
pub fn try_lst_state_list(lst_state_list_acc_data: &[u8]) -> Result<&[LstState]> {
    try_list(lst_state_list_acc_data).ok_or(ErrorCode::InvalidSanctumLstStateListData.into())
}

fn try_get_lst_index<'info>(lst_mint: Pubkey, lst_states_acc: &AccountInfo<'info>) -> Result<u32> {
    let lst_states_data = lst_states_acc.try_borrow_data().unwrap();
    let lst_states_list = try_lst_state_list(&*lst_states_data).unwrap();
    lst_states_list
        .iter()
        .position(|lst_state| lst_state.mint == lst_mint)
        .map(|index| index as u32)
        .ok_or(ErrorCode::InvalidSanctumLstStateListIndex.into())
}

fn handle_prechecks<'info>(
    swap_accounts: &impl CommonAccountInfo<'info>,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<()> {
    if swap_accounts.dex_program_id().key != &sanctum_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }

    before_check(
        swap_accounts.swap_authority_pubkey(),
        &swap_accounts.source_token_account(),
        swap_accounts.dst_token_account().key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    Ok(())
}

trait CommonAccountInfo<'info> {
    fn dex_program_id(&self) -> &AccountInfo<'info>;
    fn swap_authority_pubkey(&self) -> &AccountInfo<'info>;
    fn source_token_account(&self) -> &Box<InterfaceAccount<'info, TokenAccount>>;
    fn dst_token_account(&self) -> &Box<InterfaceAccount<'info, TokenAccount>>;
    fn get_token_accounts_mut(
        &mut self,
    ) -> (
        &mut Box<InterfaceAccount<'info, TokenAccount>>,
        &mut Box<InterfaceAccount<'info, TokenAccount>>,
    );
    fn get_accountmetas(&self) -> Vec<AccountMeta>;
    fn get_account_infos(&self) -> Vec<AccountInfo<'info>>;
}

impl<'info> CommonAccountInfo<'info> for SanctumAddLstLiqAccounts<'info> {
    fn dex_program_id(&self) -> &AccountInfo<'info> {
        &self.dex_program_id
    }
    fn swap_authority_pubkey(&self) -> &AccountInfo<'info> {
        &self.swap_authority_pubkey
    }
    fn source_token_account(&self) -> &Box<InterfaceAccount<'info, TokenAccount>> {
        &self.source_lst_acc
    }
    fn dst_token_account(&self) -> &Box<InterfaceAccount<'info, TokenAccount>> {
        &self.dst_lp_acc
    }
    fn get_token_accounts_mut(
        &mut self,
    ) -> (
        &mut Box<InterfaceAccount<'info, TokenAccount>>,
        &mut Box<InterfaceAccount<'info, TokenAccount>>,
    ) {
        (&mut self.source_lst_acc, &mut self.dst_lp_acc)
    }
    fn get_accountmetas(&self) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.swap_authority_pubkey.key(), true),
            AccountMeta::new_readonly(self.lst_mint.key(), false),
            AccountMeta::new(self.source_lst_acc.key(), false),
            AccountMeta::new(self.dst_lp_acc.key(), false),
            AccountMeta::new(self.lp_token_mint.key(), false),
            AccountMeta::new(self.protocol_fee_accumulator.key(), false),
            AccountMeta::new_readonly(self.lst_token_program.key(), false),
            AccountMeta::new_readonly(self.lp_token_program.key(), false),
            AccountMeta::new(self.pool_state.key(), false),
            AccountMeta::new(self.lst_states_list.key(), false),
            AccountMeta::new(self.pool_reserves.key(), false),
            AccountMeta::new_readonly(self.spl_sol_calculator.key(), false),
            AccountMeta::new_readonly(self.calculator_state.key(), false),
            AccountMeta::new_readonly(self.staked_pool_state.key(), false),
            AccountMeta::new_readonly(self.validator_pool_program.key(), false),
            AccountMeta::new_readonly(self.validator_pool_program_data.key(), false),
            AccountMeta::new_readonly(self.sanctum_flat_fee_pricing.key(), false),
        ]
    }
    fn get_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.swap_authority_pubkey.to_account_info(),
            self.lst_mint.to_account_info(),
            self.source_lst_acc.to_account_info(),
            self.dst_lp_acc.to_account_info(),
            self.lp_token_mint.to_account_info(),
            self.protocol_fee_accumulator.to_account_info(),
            self.lst_token_program.to_account_info(),
            self.lp_token_program.to_account_info(),
            self.pool_state.to_account_info(),
            self.lst_states_list.to_account_info(),
            self.pool_reserves.to_account_info(),
            self.spl_sol_calculator.to_account_info(),
            self.calculator_state.to_account_info(),
            self.staked_pool_state.to_account_info(),
            self.validator_pool_program.to_account_info(),
            self.validator_pool_program_data.to_account_info(),
            self.sanctum_flat_fee_pricing.to_account_info(),
        ]
    }
}

impl<'info> CommonAccountInfo<'info> for SanctumAddWsolLiqAccounts<'info> {
    fn dex_program_id(&self) -> &AccountInfo<'info> {
        &self.dex_program_id
    }
    fn swap_authority_pubkey(&self) -> &AccountInfo<'info> {
        &self.swap_authority_pubkey
    }
    fn source_token_account(&self) -> &Box<InterfaceAccount<'info, TokenAccount>> {
        &self.source_lst_acc
    }
    fn dst_token_account(&self) -> &Box<InterfaceAccount<'info, TokenAccount>> {
        &self.dst_lp_acc
    }
    fn get_token_accounts_mut(
        &mut self,
    ) -> (
        &mut Box<InterfaceAccount<'info, TokenAccount>>,
        &mut Box<InterfaceAccount<'info, TokenAccount>>,
    ) {
        (&mut self.source_lst_acc, &mut self.dst_lp_acc)
    }
    fn get_accountmetas(&self) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.swap_authority_pubkey.key(), true),
            AccountMeta::new_readonly(self.lst_mint.key(), false),
            AccountMeta::new(self.source_lst_acc.key(), false),
            AccountMeta::new(self.dst_lp_acc.key(), false),
            AccountMeta::new(self.lp_token_mint.key(), false),
            AccountMeta::new(self.protocol_fee_accumulator.key(), false),
            AccountMeta::new_readonly(self.lst_token_program.key(), false),
            AccountMeta::new_readonly(self.lp_token_program.key(), false),
            AccountMeta::new(self.pool_state.key(), false),
            AccountMeta::new(self.lst_states_list.key(), false),
            AccountMeta::new(self.pool_reserves.key(), false),
            AccountMeta::new_readonly(self.wsol_calculator.key(), false),
            AccountMeta::new_readonly(self.sanctum_flat_fee_pricing.key(), false),
        ]
    }
    fn get_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.swap_authority_pubkey.to_account_info(),
            self.lst_mint.to_account_info(),
            self.source_lst_acc.to_account_info(),
            self.dst_lp_acc.to_account_info(),
            self.lp_token_mint.to_account_info(),
            self.protocol_fee_accumulator.to_account_info(),
            self.lst_token_program.to_account_info(),
            self.lp_token_program.to_account_info(),
            self.pool_state.to_account_info(),
            self.lst_states_list.to_account_info(),
            self.pool_reserves.to_account_info(),
            self.wsol_calculator.to_account_info(),
            self.sanctum_flat_fee_pricing.to_account_info(),
        ]
    }
}

impl<'info> CommonAccountInfo<'info> for SanctumRemoveWsolLiqAccounts<'info> {
    fn dex_program_id(&self) -> &AccountInfo<'info> {
        &self.dex_program_id
    }
    fn swap_authority_pubkey(&self) -> &AccountInfo<'info> {
        &self.swap_authority_pubkey
    }
    fn source_token_account(&self) -> &Box<InterfaceAccount<'info, TokenAccount>> {
        &self.source_lp_acc
    }
    fn dst_token_account(&self) -> &Box<InterfaceAccount<'info, TokenAccount>> {
        &self.dst_lst_acc
    }
    fn get_token_accounts_mut(
        &mut self,
    ) -> (
        &mut Box<InterfaceAccount<'info, TokenAccount>>,
        &mut Box<InterfaceAccount<'info, TokenAccount>>,
    ) {
        (&mut self.source_lp_acc, &mut self.dst_lst_acc)
    }
    fn get_accountmetas(&self) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.swap_authority_pubkey.key(), true),
            AccountMeta::new_readonly(self.lst_mint.key(), false),
            AccountMeta::new(self.dst_lst_acc.key(), false),
            AccountMeta::new(self.source_lp_acc.key(), false),
            AccountMeta::new(self.lp_token_mint.key(), false),
            AccountMeta::new(self.protocol_fee_accumulator.key(), false),
            AccountMeta::new_readonly(self.lst_token_program.key(), false),
            AccountMeta::new_readonly(self.lp_token_program.key(), false),
            AccountMeta::new(self.pool_state.key(), false),
            AccountMeta::new(self.lst_states_list.key(), false),
            AccountMeta::new(self.pool_reserves.key(), false),
            AccountMeta::new_readonly(self.wsol_calculator.key(), false),
            AccountMeta::new_readonly(self.sanctum_flat_fee_pricing.key(), false),
            AccountMeta::new_readonly(self.sanctum_flat_fee_pricing_account.key(), false),
        ]
    }
    fn get_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.swap_authority_pubkey.to_account_info(),
            self.lst_mint.to_account_info(),
            self.dst_lst_acc.to_account_info(),
            self.source_lp_acc.to_account_info(),
            self.lp_token_mint.to_account_info(),
            self.protocol_fee_accumulator.to_account_info(),
            self.lst_token_program.to_account_info(),
            self.lp_token_program.to_account_info(),
            self.pool_state.to_account_info(),
            self.lst_states_list.to_account_info(),
            self.pool_reserves.to_account_info(),
            self.wsol_calculator.to_account_info(),
            self.sanctum_flat_fee_pricing.to_account_info(),
            self.sanctum_flat_fee_pricing_account.to_account_info(),
        ]
    }
}

impl<'info> CommonAccountInfo<'info> for SanctumRemoveLstLiqAccounts<'info> {
    fn dex_program_id(&self) -> &AccountInfo<'info> {
        &self.dex_program_id
    }
    fn swap_authority_pubkey(&self) -> &AccountInfo<'info> {
        &self.swap_authority_pubkey
    }
    fn source_token_account(&self) -> &Box<InterfaceAccount<'info, TokenAccount>> {
        &self.source_lp_acc
    }
    fn dst_token_account(&self) -> &Box<InterfaceAccount<'info, TokenAccount>> {
        &self.dst_lst_acc
    }
    fn get_token_accounts_mut(
        &mut self,
    ) -> (
        &mut Box<InterfaceAccount<'info, TokenAccount>>,
        &mut Box<InterfaceAccount<'info, TokenAccount>>,
    ) {
        (&mut self.source_lp_acc, &mut self.dst_lst_acc)
    }
    fn get_accountmetas(&self) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.swap_authority_pubkey.key(), true),
            AccountMeta::new_readonly(self.lst_mint.key(), false),
            AccountMeta::new(self.dst_lst_acc.key(), false),
            AccountMeta::new(self.source_lp_acc.key(), false),
            AccountMeta::new(self.lp_token_mint.key(), false),
            AccountMeta::new(self.protocol_fee_accumulator.key(), false),
            AccountMeta::new_readonly(self.lst_token_program.key(), false),
            AccountMeta::new_readonly(self.lp_token_program.key(), false),
            AccountMeta::new(self.pool_state.key(), false),
            AccountMeta::new(self.lst_states_list.key(), false),
            AccountMeta::new(self.pool_reserves.key(), false),
            AccountMeta::new_readonly(self.spl_sol_calculator.key(), false),
            AccountMeta::new_readonly(self.calculator_state.key(), false),
            AccountMeta::new_readonly(self.staked_pool_state.key(), false),
            AccountMeta::new_readonly(self.validator_pool_program.key(), false),
            AccountMeta::new_readonly(self.validator_pool_program_data.key(), false),
            AccountMeta::new_readonly(self.sanctum_flat_fee_pricing.key(), false),
            AccountMeta::new_readonly(self.sanctum_flat_fee_pricing_account.key(), false),
        ]
    }
    fn get_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.swap_authority_pubkey.to_account_info(),
            self.lst_mint.to_account_info(),
            self.dst_lst_acc.to_account_info(),
            self.source_lp_acc.to_account_info(),
            self.lp_token_mint.to_account_info(),
            self.protocol_fee_accumulator.to_account_info(),
            self.lst_token_program.to_account_info(),
            self.lp_token_program.to_account_info(),
            self.pool_state.to_account_info(),
            self.lst_states_list.to_account_info(),
            self.pool_reserves.to_account_info(),
            self.spl_sol_calculator.to_account_info(),
            self.calculator_state.to_account_info(),
            self.staked_pool_state.to_account_info(),
            self.validator_pool_program.to_account_info(),
            self.validator_pool_program_data.to_account_info(),
            self.sanctum_flat_fee_pricing.to_account_info(),
            self.sanctum_flat_fee_pricing_account.to_account_info(),
        ]
    }
}

impl<'info> CommonAccountInfo<'info> for SanctumWsolLstSwapAccounts<'info> {
    fn dex_program_id(&self) -> &AccountInfo<'info> {
        &self.dex_program_id
    }
    fn swap_authority_pubkey(&self) -> &AccountInfo<'info> {
        &self.swap_authority_pubkey
    }
    fn source_token_account(&self) -> &Box<InterfaceAccount<'info, TokenAccount>> {
        &self.source_lst_account
    }
    fn dst_token_account(&self) -> &Box<InterfaceAccount<'info, TokenAccount>> {
        &self.dst_lst_account
    }
    fn get_token_accounts_mut(
        &mut self,
    ) -> (
        &mut Box<InterfaceAccount<'info, TokenAccount>>,
        &mut Box<InterfaceAccount<'info, TokenAccount>>,
    ) {
        (&mut self.source_lst_account, &mut self.dst_lst_account)
    }
    fn get_accountmetas(&self) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.swap_authority_pubkey.key(), true),
            AccountMeta::new_readonly(self.source_lst_mint.key(), false),
            AccountMeta::new_readonly(self.dst_lst_mint.key(), false),
            AccountMeta::new(self.source_lst_account.key(), false),
            AccountMeta::new(self.dst_lst_account.key(), false),
            AccountMeta::new(self.protocol_fee_accumulator.key(), false),
            AccountMeta::new_readonly(self.source_token_program.key(), false),
            AccountMeta::new_readonly(self.dst_token_program.key(), false),
            AccountMeta::new(self.pool_state.key(), false),
            AccountMeta::new(self.lst_states_list.key(), false),
            AccountMeta::new(self.source_pool_reserves.key(), false),
            AccountMeta::new(self.dst_pool_reserves.key(), false),
            AccountMeta::new_readonly(self.wsol_calculator.key(), false),
            AccountMeta::new_readonly(self.spl_sol_calculator.key(), false),
            AccountMeta::new_readonly(self.calculator_state.key(), false),
            AccountMeta::new_readonly(self.staked_pool_state.key(), false),
            AccountMeta::new_readonly(self.validator_pool_program.key(), false),
            AccountMeta::new_readonly(self.validator_pool_program_data.key(), false),
            AccountMeta::new_readonly(self.sanctum_flat_fee_pricing.key(), false),
            AccountMeta::new_readonly(self.sanctum_src_flat_fee_pricing_account.key(), false),
            AccountMeta::new_readonly(self.sanctum_dst_flat_fee_pricing_account.key(), false),
        ]
    }
    fn get_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.swap_authority_pubkey.to_account_info(),
            self.source_lst_mint.to_account_info(),
            self.dst_lst_mint.to_account_info(),
            self.source_lst_account.to_account_info(),
            self.dst_lst_account.to_account_info(),
            self.protocol_fee_accumulator.to_account_info(),
            self.source_token_program.to_account_info(),
            self.dst_token_program.to_account_info(),
            self.pool_state.to_account_info(),
            self.lst_states_list.to_account_info(),
            self.source_pool_reserves.to_account_info(),
            self.dst_pool_reserves.to_account_info(),
            self.wsol_calculator.to_account_info(),
            self.spl_sol_calculator.to_account_info(),
            self.calculator_state.to_account_info(),
            self.staked_pool_state.to_account_info(),
            self.validator_pool_program.to_account_info(),
            self.validator_pool_program_data.to_account_info(),
            self.sanctum_flat_fee_pricing.to_account_info(),
            self.sanctum_src_flat_fee_pricing_account.to_account_info(),
            self.sanctum_dst_flat_fee_pricing_account.to_account_info(),
        ]
    }
}

impl<'info> CommonAccountInfo<'info> for SanctumLstWsolSwapAccounts<'info> {
    fn dex_program_id(&self) -> &AccountInfo<'info> {
        &self.dex_program_id
    }
    fn swap_authority_pubkey(&self) -> &AccountInfo<'info> {
        &self.swap_authority_pubkey
    }
    fn source_token_account(&self) -> &Box<InterfaceAccount<'info, TokenAccount>> {
        &self.source_lst_account
    }
    fn dst_token_account(&self) -> &Box<InterfaceAccount<'info, TokenAccount>> {
        &self.dst_lst_account
    }
    fn get_token_accounts_mut(
        &mut self,
    ) -> (
        &mut Box<InterfaceAccount<'info, TokenAccount>>,
        &mut Box<InterfaceAccount<'info, TokenAccount>>,
    ) {
        (&mut self.source_lst_account, &mut self.dst_lst_account)
    }
    fn get_accountmetas(&self) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.swap_authority_pubkey.key(), true),
            AccountMeta::new_readonly(self.source_lst_mint.key(), false),
            AccountMeta::new_readonly(self.dst_lst_mint.key(), false),
            AccountMeta::new(self.source_lst_account.key(), false),
            AccountMeta::new(self.dst_lst_account.key(), false),
            AccountMeta::new(self.protocol_fee_accumulator.key(), false),
            AccountMeta::new_readonly(self.source_token_program.key(), false),
            AccountMeta::new_readonly(self.dst_token_program.key(), false),
            AccountMeta::new(self.pool_state.key(), false),
            AccountMeta::new(self.lst_states_list.key(), false),
            AccountMeta::new(self.source_pool_reserves.key(), false),
            AccountMeta::new(self.dst_pool_reserves.key(), false),
            AccountMeta::new_readonly(self.spl_sol_calculator.key(), false),
            AccountMeta::new_readonly(self.calculator_state.key(), false),
            AccountMeta::new_readonly(self.staked_pool_state.key(), false),
            AccountMeta::new_readonly(self.validator_pool_program.key(), false),
            AccountMeta::new_readonly(self.validator_pool_program_data.key(), false),
            AccountMeta::new_readonly(self.wsol_calculator.key(), false),
            AccountMeta::new_readonly(self.sanctum_flat_fee_pricing.key(), false),
            AccountMeta::new_readonly(self.sanctum_src_flat_fee_pricing_account.key(), false),
            AccountMeta::new_readonly(self.sanctum_dst_flat_fee_pricing_account.key(), false),
        ]
    }
    fn get_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.swap_authority_pubkey.to_account_info(),
            self.source_lst_mint.to_account_info(),
            self.dst_lst_mint.to_account_info(),
            self.source_lst_account.to_account_info(),
            self.dst_lst_account.to_account_info(),
            self.protocol_fee_accumulator.to_account_info(),
            self.source_token_program.to_account_info(),
            self.dst_token_program.to_account_info(),
            self.pool_state.to_account_info(),
            self.lst_states_list.to_account_info(),
            self.source_pool_reserves.to_account_info(),
            self.dst_pool_reserves.to_account_info(),
            self.spl_sol_calculator.to_account_info(),
            self.calculator_state.to_account_info(),
            self.staked_pool_state.to_account_info(),
            self.validator_pool_program.to_account_info(),
            self.validator_pool_program_data.to_account_info(),
            self.wsol_calculator.to_account_info(),
            self.sanctum_flat_fee_pricing.to_account_info(),
            self.sanctum_src_flat_fee_pricing_account.to_account_info(),
            self.sanctum_dst_flat_fee_pricing_account.to_account_info(),
        ]
    }
}

impl<'info> CommonAccountInfo<'info> for SanctumLstLstSwapAccounts<'info> {
    fn dex_program_id(&self) -> &AccountInfo<'info> {
        &self.dex_program_id
    }
    fn swap_authority_pubkey(&self) -> &AccountInfo<'info> {
        &self.swap_authority_pubkey
    }
    fn source_token_account(&self) -> &Box<InterfaceAccount<'info, TokenAccount>> {
        &self.source_lst_account
    }
    fn dst_token_account(&self) -> &Box<InterfaceAccount<'info, TokenAccount>> {
        &self.dst_lst_account
    }
    fn get_token_accounts_mut(
        &mut self,
    ) -> (
        &mut Box<InterfaceAccount<'info, TokenAccount>>,
        &mut Box<InterfaceAccount<'info, TokenAccount>>,
    ) {
        (&mut self.source_lst_account, &mut self.dst_lst_account)
    }
    fn get_accountmetas(&self) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.swap_authority_pubkey.key(), true),
            AccountMeta::new_readonly(self.source_lst_mint.key(), false),
            AccountMeta::new_readonly(self.dst_lst_mint.key(), false),
            AccountMeta::new(self.source_lst_account.key(), false),
            AccountMeta::new(self.dst_lst_account.key(), false),
            AccountMeta::new(self.protocol_fee_accumulator.key(), false),
            AccountMeta::new_readonly(self.source_token_program.key(), false),
            AccountMeta::new_readonly(self.dst_token_program.key(), false),
            AccountMeta::new(self.pool_state.key(), false),
            AccountMeta::new(self.lst_states_list.key(), false),
            AccountMeta::new(self.source_pool_reserves.key(), false),
            AccountMeta::new(self.dst_pool_reserves.key(), false),
            AccountMeta::new_readonly(self.src_spl_sol_calculator.key(), false),
            AccountMeta::new_readonly(self.src_calculator_state.key(), false),
            AccountMeta::new_readonly(self.src_staked_pool_state.key(), false),
            AccountMeta::new_readonly(self.src_validator_pool_program.key(), false),
            AccountMeta::new_readonly(self.src_validator_pool_program_data.key(), false),
            AccountMeta::new_readonly(self.dst_spl_sol_calculator.key(), false),
            AccountMeta::new_readonly(self.dst_calculator_state.key(), false),
            AccountMeta::new_readonly(self.dst_staked_pool_state.key(), false),
            AccountMeta::new_readonly(self.dst_validator_pool_program.key(), false),
            AccountMeta::new_readonly(self.dst_validator_pool_program_data.key(), false),
            AccountMeta::new_readonly(self.sanctum_flat_fee_pricing.key(), false),
            AccountMeta::new_readonly(self.sanctum_src_flat_fee_pricing_account.key(), false),
            AccountMeta::new_readonly(self.sanctum_dst_flat_fee_pricing_account.key(), false),
        ]
    }
    fn get_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.swap_authority_pubkey.to_account_info(),
            self.source_lst_mint.to_account_info(),
            self.dst_lst_mint.to_account_info(),
            self.source_lst_account.to_account_info(),
            self.dst_lst_account.to_account_info(),
            self.protocol_fee_accumulator.to_account_info(),
            self.source_token_program.to_account_info(),
            self.dst_token_program.to_account_info(),
            self.pool_state.to_account_info(),
            self.lst_states_list.to_account_info(),
            self.source_pool_reserves.to_account_info(),
            self.dst_pool_reserves.to_account_info(),
            self.src_spl_sol_calculator.to_account_info(),
            self.src_calculator_state.to_account_info(),
            self.src_staked_pool_state.to_account_info(),
            self.src_validator_pool_program.to_account_info(),
            self.src_validator_pool_program_data.to_account_info(),
            self.dst_spl_sol_calculator.to_account_info(),
            self.dst_calculator_state.to_account_info(),
            self.dst_staked_pool_state.to_account_info(),
            self.dst_validator_pool_program.to_account_info(),
            self.dst_validator_pool_program_data.to_account_info(),
            self.sanctum_flat_fee_pricing.to_account_info(),
            self.sanctum_src_flat_fee_pricing_account.to_account_info(),
            self.sanctum_dst_flat_fee_pricing_account.to_account_info(),
        ]
    }
}

// #[inline(never)]
fn handle_lst_lst_swap<'info>(
    swap_accounts: &mut SanctumLstLstSwapAccounts<'info>,
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    // log pool address
    swap_accounts.pool_state.key().log();
    handle_prechecks(swap_accounts, hop_accounts, hop, proxy_swap, owner_seeds)?;

    let src_lst_index =
        try_get_lst_index(swap_accounts.source_lst_mint.key(), &swap_accounts.lst_states_list)?;

    let dst_lst_index =
        try_get_lst_index(swap_accounts.dst_lst_mint.key(), &swap_accounts.lst_states_list)?;

    let mut ix_data = Vec::with_capacity(SWAP_EXACT_IN_ARGS_LEN);
    ix_data.extend_from_slice(&1u8.to_le_bytes()); //discriminant
    ix_data.extend_from_slice(&5u8.to_le_bytes()); //src_lst_value_calc_accs
    ix_data.extend_from_slice(&5u8.to_le_bytes()); //dst_lst_value_calc_accs
    ix_data.extend_from_slice(&src_lst_index.to_le_bytes()); //src_lst_index
    ix_data.extend_from_slice(&dst_lst_index.to_le_bytes()); //dst_lst_index
    ix_data.extend_from_slice(&1u64.to_le_bytes()); //min_amount_out
    ix_data.extend_from_slice(&amount_in.to_le_bytes()); //lst_amount

    let accounts = swap_accounts.get_accountmetas();
    let account_infos = swap_accounts.get_account_infos();
    let (source_token_account, dst_token_account) = swap_accounts.get_token_accounts_mut();

    let dex_processor = &SanctumProcessor;
    invoke_process(
        amount_in,
        dex_processor,
        &account_infos,
        source_token_account,
        dst_token_account,
        hop_accounts,
        Instruction { program_id: sanctum_program::id(), accounts, data: ix_data },
        hop,
        offset,
        SWAP_LST_LST_ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )
}

// #[inline(never)]
fn handle_wsol_lst_swap<'info>(
    swap_accounts: &mut SanctumWsolLstSwapAccounts<'info>,
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    // log pool address
    swap_accounts.pool_state.key().log();
    handle_prechecks(swap_accounts, hop_accounts, hop, proxy_swap, owner_seeds)?;

    let lst_index =
        try_get_lst_index(swap_accounts.dst_lst_mint.key(), &swap_accounts.lst_states_list)?;

    let mut ix_data = Vec::with_capacity(SWAP_EXACT_IN_ARGS_LEN);
    ix_data.extend_from_slice(&1u8.to_le_bytes()); //discriminant
    ix_data.extend_from_slice(&1u8.to_le_bytes()); //src_lst_value_calc_accs
    ix_data.extend_from_slice(&5u8.to_le_bytes()); //dst_lst_value_calc_accs
    ix_data.extend_from_slice(&1u32.to_le_bytes()); //src_lst_index
    ix_data.extend_from_slice(&lst_index.to_le_bytes()); //dst_lst_index
    ix_data.extend_from_slice(&1u64.to_le_bytes()); //min_amount_out
    ix_data.extend_from_slice(&amount_in.to_le_bytes()); //lst_amount

    let accounts = swap_accounts.get_accountmetas();
    let account_infos = swap_accounts.get_account_infos();
    let (source_token_account, dst_token_account) = swap_accounts.get_token_accounts_mut();

    let dex_processor = &SanctumProcessor;
    invoke_process(
        amount_in,
        dex_processor,
        &account_infos,
        source_token_account,
        dst_token_account,
        hop_accounts,
        Instruction { program_id: sanctum_program::id(), accounts, data: ix_data },
        hop,
        offset,
        SWAP_LST_SOL_ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )
}

// #[inline(never)]
fn handle_lst_wsol_swap<'info>(
    swap_accounts: &mut SanctumLstWsolSwapAccounts<'info>,
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    // log pool address
    swap_accounts.pool_state.key().log();
    handle_prechecks(swap_accounts, hop_accounts, hop, proxy_swap, owner_seeds)?;

    let lst_index =
        try_get_lst_index(swap_accounts.source_lst_mint.key(), &swap_accounts.lst_states_list)?;

    let mut ix_data = Vec::with_capacity(SWAP_EXACT_IN_ARGS_LEN);
    ix_data.extend_from_slice(&1u8.to_le_bytes()); //discriminant
    ix_data.extend_from_slice(&5u8.to_le_bytes()); //src_lst_value_calc_accs
    ix_data.extend_from_slice(&1u8.to_le_bytes()); //dst_lst_value_calc_accs
    ix_data.extend_from_slice(&lst_index.to_le_bytes()); //src_lst_index
    ix_data.extend_from_slice(&1u32.to_le_bytes()); //dst_lst_index
    ix_data.extend_from_slice(&1u64.to_le_bytes()); //min_amount_out
    ix_data.extend_from_slice(&amount_in.to_le_bytes()); //lst_amount

    let accounts = swap_accounts.get_accountmetas();
    let account_infos = swap_accounts.get_account_infos();
    let (source_token_account, dst_token_account) = swap_accounts.get_token_accounts_mut();

    let dex_processor = &SanctumProcessor;
    invoke_process(
        amount_in,
        dex_processor,
        &account_infos,
        source_token_account,
        dst_token_account,
        hop_accounts,
        Instruction { program_id: sanctum_program::id(), accounts, data: ix_data },
        hop,
        offset,
        SWAP_LST_SOL_ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )
}
