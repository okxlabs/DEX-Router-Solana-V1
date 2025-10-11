use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{
    CPSWAP_SELECTOR, HopAccounts, SWAP_SELECTOR, SWAP_V2_SELECTOR, ZERO_ADDRESS,
    raydium_clmm_program, raydium_cpmm_program, raydium_stable_program, raydium_swap_program,
};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token::Token;
use anchor_spl::token_interface::{Mint, Token2022, TokenAccount, TokenInterface};
use arrayref::array_ref;

use super::common::DexProcessor;

const ARGS_LEN: usize = 17;
const ARGS_CLMM_LEN: usize = 41;
const ARGS_CPMM_LEN: usize = 24;

pub struct RaydiumSwapProcessor;
impl DexProcessor for RaydiumSwapProcessor {}

pub struct RaydiumSwapAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub amm_id: &'info AccountInfo<'info>,
    pub amm_authority: &'info AccountInfo<'info>,
    pub amm_open_orders: &'info AccountInfo<'info>,
    pub amm_target_orders: &'info AccountInfo<'info>,
    pub pool_coin_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub pool_pc_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub serum_program_id: &'info AccountInfo<'info>,
    pub serum_market: &'info AccountInfo<'info>,
    pub serum_bids: &'info AccountInfo<'info>,
    pub serum_asks: &'info AccountInfo<'info>,
    pub serum_event_queue: &'info AccountInfo<'info>,
    pub serum_coin_vault_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub serum_pc_vault_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub serum_vault_signer: &'info AccountInfo<'info>,
}
const ACCOUNTS_LEN: usize = 19;

pub struct RaydiumSwapV2Accounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub amm_id: &'info AccountInfo<'info>,
    pub amm_authority: &'info AccountInfo<'info>,
    pub pool_coin_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub pool_pc_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
}
const ACCOUNTS_V2_LEN: usize = 9;

pub struct RaydiumStableAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub amm_id: &'info AccountInfo<'info>,
    pub amm_authority: &'info AccountInfo<'info>,
    pub amm_open_orders: &'info AccountInfo<'info>,
    pub pool_coin_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub pool_pc_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub model_data_account: &'info AccountInfo<'info>,
    pub serum_program_id: &'info AccountInfo<'info>,
    pub serum_market: &'info AccountInfo<'info>,
    pub serum_bids: &'info AccountInfo<'info>,
    pub serum_asks: &'info AccountInfo<'info>,
    pub serum_event_queue: &'info AccountInfo<'info>,
    pub serum_coin_vault_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub serum_pc_vault_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub serum_vault_signer: &'info AccountInfo<'info>,
}
const STABLE_ACCOUNTS_LEN: usize = 19;

pub struct RaydiumClmmAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub amm_config_id: &'info AccountInfo<'info>,
    pub pool_id: &'info AccountInfo<'info>,
    pub input_vault: &'info AccountInfo<'info>,
    pub output_vault: &'info AccountInfo<'info>,
    pub observation_id: &'info AccountInfo<'info>,
    pub tick_array0: &'info AccountInfo<'info>,
    pub ex_bitmap: &'info AccountInfo<'info>,
    pub tick_array1: &'info AccountInfo<'info>,
    pub tick_array2: &'info AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}
const CLMM_ACCOUNTS_LEN: usize = 14;

pub struct RaydiumClmmV2Accounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub amm_config_id: &'info AccountInfo<'info>,
    pub pool_id: &'info AccountInfo<'info>,
    pub input_vault: InterfaceAccount<'info, TokenAccount>,
    pub output_vault: InterfaceAccount<'info, TokenAccount>,
    pub observation_id: &'info AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
    pub token_program_2022: Program<'info, Token2022>,
    pub memo_program: &'info AccountInfo<'info>,
    pub input_vault_mint: InterfaceAccount<'info, Mint>,
    pub output_vault_mint: InterfaceAccount<'info, Mint>,
    pub ex_bitmap: &'info AccountInfo<'info>,
    pub tick_array0: &'info AccountInfo<'info>,
    pub tick_array1: &'info AccountInfo<'info>,
    pub tick_array2: &'info AccountInfo<'info>,
}
const CLMM_V2_ACCOUNTS_LEN: usize = 18;

pub struct RaydiumCpmmAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

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
const CPMM_ACCOUNTS_LEN: usize = 14;

impl<'info> RaydiumSwapAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            token_program,
            amm_id,
            amm_authority,
            amm_open_orders,
            amm_target_orders,
            pool_coin_token_account,
            pool_pc_token_account,
            serum_program_id,
            serum_market,
            serum_bids,
            serum_asks,
            serum_event_queue,
            serum_coin_vault_account,
            serum_pc_vault_account,
            serum_vault_signer,
        ]: &[AccountInfo<'info>; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            token_program: Program::try_from(token_program)?,
            amm_id,
            amm_authority,
            amm_open_orders,
            amm_target_orders,
            pool_coin_token_account: Box::new(InterfaceAccount::try_from(pool_coin_token_account)?),
            pool_pc_token_account: Box::new(InterfaceAccount::try_from(pool_pc_token_account)?),
            serum_program_id,
            serum_market,
            serum_bids,
            serum_asks,
            serum_event_queue,
            serum_coin_vault_account: Box::new(InterfaceAccount::try_from(
                serum_coin_vault_account,
            )?),
            serum_pc_vault_account: Box::new(InterfaceAccount::try_from(serum_pc_vault_account)?),
            serum_vault_signer,
        })
    }
}

impl<'info> RaydiumSwapV2Accounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            token_program,
            amm_id,
            amm_authority,
            pool_coin_token_account,
            pool_pc_token_account,
        ]: &[AccountInfo<'info>; ACCOUNTS_V2_LEN] = array_ref![accounts, offset, ACCOUNTS_V2_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            token_program: Program::try_from(token_program)?,
            amm_id,
            amm_authority,
            pool_coin_token_account: Box::new(InterfaceAccount::try_from(pool_coin_token_account)?),
            pool_pc_token_account: Box::new(InterfaceAccount::try_from(pool_pc_token_account)?),
        })
    }
}

impl<'info> RaydiumStableAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            token_program,
            amm_id,
            amm_authority,
            amm_open_orders,
            pool_coin_token_account,
            pool_pc_token_account,
            model_data_account,
            serum_program_id,
            serum_market,
            serum_bids,
            serum_asks,
            serum_event_queue,
            serum_coin_vault_account,
            serum_pc_vault_account,
            serum_vault_signer,
        ]: &[AccountInfo<'info>; STABLE_ACCOUNTS_LEN] =
            array_ref![accounts, offset, STABLE_ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            token_program: Program::try_from(token_program)?,
            amm_id,
            amm_authority,
            amm_open_orders,
            pool_coin_token_account: Box::new(InterfaceAccount::try_from(pool_coin_token_account)?),
            pool_pc_token_account: Box::new(InterfaceAccount::try_from(pool_pc_token_account)?),
            model_data_account,
            serum_program_id,
            serum_market,
            serum_bids,
            serum_asks,
            serum_event_queue,
            serum_coin_vault_account: Box::new(InterfaceAccount::try_from(
                serum_coin_vault_account,
            )?),
            serum_pc_vault_account: Box::new(InterfaceAccount::try_from(serum_pc_vault_account)?),
            serum_vault_signer,
        })
    }
}

impl<'info> RaydiumClmmAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            amm_config_id,
            pool_id,
            input_vault,
            output_vault,
            observation_id,
            tick_array0,
            ex_bitmap,
            tick_array1,
            tick_array2,
            token_program,
        ]: &[AccountInfo<'info>; CLMM_ACCOUNTS_LEN] =
            array_ref![accounts, offset, CLMM_ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            amm_config_id,
            pool_id,
            input_vault,
            output_vault,
            observation_id,
            tick_array0,
            ex_bitmap,
            tick_array1,
            tick_array2,
            token_program: Program::try_from(token_program)?,
        })
    }
}

impl<'info> RaydiumClmmV2Accounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            amm_config_id,
            pool_id,
            input_vault,
            output_vault,
            observation_id,
            token_program,
            token_program_2022,
            memo_program,
            input_vault_mint,
            output_vault_mint,
            ex_bitmap,
            tick_array0,
            tick_array1,
            tick_array2,
        ]: &[AccountInfo<'info>; CLMM_V2_ACCOUNTS_LEN] =
            array_ref![accounts, offset, CLMM_V2_ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            amm_config_id,
            pool_id,
            input_vault: InterfaceAccount::try_from(input_vault)?,
            output_vault: InterfaceAccount::try_from(output_vault)?,
            observation_id,
            token_program: Program::try_from(token_program)?,
            token_program_2022: Program::try_from(token_program_2022)?,
            memo_program,
            input_vault_mint: InterfaceAccount::try_from(input_vault_mint)?,
            output_vault_mint: InterfaceAccount::try_from(output_vault_mint)?,
            ex_bitmap,
            tick_array0,
            tick_array1,
            tick_array2,
        })
    }
}

impl<'info> RaydiumCpmmAccounts<'info> {
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
        ]: &[AccountInfo<'info>; CPMM_ACCOUNTS_LEN] =
            array_ref![accounts, offset, CPMM_ACCOUNTS_LEN];

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
    msg!("Dex::RaydiumSwap amount_in: {}, offset: {}", amount_in, offset);
    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);

    let mut swap_accounts = RaydiumSwapAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &raydium_swap_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.amm_id.key().log();

    // check hop accounts & swap authority
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

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.push(9);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&1u64.to_le_bytes());

    let accounts = vec![
        // spl token
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
        // amm
        AccountMeta::new(swap_accounts.amm_id.key(), false),
        AccountMeta::new_readonly(swap_accounts.amm_authority.key(), false),
        AccountMeta::new(swap_accounts.amm_open_orders.key(), false),
        AccountMeta::new(swap_accounts.amm_target_orders.key(), false),
        AccountMeta::new(swap_accounts.pool_coin_token_account.key(), false),
        AccountMeta::new(swap_accounts.pool_pc_token_account.key(), false),
        // serum
        AccountMeta::new_readonly(swap_accounts.serum_program_id.key(), false),
        AccountMeta::new(swap_accounts.serum_market.key(), false),
        AccountMeta::new(swap_accounts.serum_bids.key(), false),
        AccountMeta::new(swap_accounts.serum_asks.key(), false),
        AccountMeta::new(swap_accounts.serum_event_queue.key(), false),
        AccountMeta::new(swap_accounts.serum_coin_vault_account.key(), false),
        AccountMeta::new(swap_accounts.serum_pc_vault_account.key(), false),
        AccountMeta::new_readonly(swap_accounts.serum_vault_signer.key(), false),
        // user
        AccountMeta::new(swap_source_token, false),
        AccountMeta::new(swap_destination_token, false),
        AccountMeta::new_readonly(swap_accounts.swap_authority_pubkey.key(), true),
    ];

    let account_infos = vec![
        swap_accounts.token_program.to_account_info(),
        swap_accounts.amm_id.to_account_info(),
        swap_accounts.amm_authority.to_account_info(),
        swap_accounts.amm_open_orders.to_account_info(),
        swap_accounts.amm_target_orders.to_account_info(),
        swap_accounts.pool_coin_token_account.to_account_info(),
        swap_accounts.pool_pc_token_account.to_account_info(),
        swap_accounts.serum_program_id.to_account_info(),
        swap_accounts.serum_market.to_account_info(),
        swap_accounts.serum_bids.to_account_info(),
        swap_accounts.serum_asks.to_account_info(),
        swap_accounts.serum_event_queue.to_account_info(),
        swap_accounts.serum_coin_vault_account.to_account_info(),
        swap_accounts.serum_pc_vault_account.to_account_info(),
        swap_accounts.serum_vault_signer.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &RaydiumSwapProcessor;
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

pub fn swap_v2<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::RaydiumSwapV2 amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + ACCOUNTS_V2_LEN,
        ErrorCode::InvalidAccountsLength
    );

    let mut swap_accounts = RaydiumSwapV2Accounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &raydium_swap_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.amm_id.key().log();

    // check hop accounts & swap authority
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

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.push(16);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&1u64.to_le_bytes());

    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
        AccountMeta::new(swap_accounts.amm_id.key(), false),
        AccountMeta::new_readonly(swap_accounts.amm_authority.key(), false),
        AccountMeta::new(swap_accounts.pool_coin_token_account.key(), false),
        AccountMeta::new(swap_accounts.pool_pc_token_account.key(), false),
        AccountMeta::new(swap_source_token, false),
        AccountMeta::new(swap_destination_token, false),
        AccountMeta::new_readonly(swap_accounts.swap_authority_pubkey.key(), true),
    ];

    let account_infos = vec![
        swap_accounts.token_program.to_account_info(),
        swap_accounts.amm_id.to_account_info(),
        swap_accounts.amm_authority.to_account_info(),
        swap_accounts.pool_coin_token_account.to_account_info(),
        swap_accounts.pool_pc_token_account.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &RaydiumSwapProcessor;
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
        ACCOUNTS_V2_LEN,
        proxy_swap,
        owner_seeds,
    )?;
    Ok(amount_out)
}

pub fn swap_stable<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::RaydiumStable amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + STABLE_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );

    let mut swap_accounts = RaydiumStableAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &raydium_stable_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.amm_id.key().log();

    // check hop accounts & swap authority
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

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.push(9);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&1u64.to_le_bytes());

    let accounts = vec![
        // spl token
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
        // amm
        AccountMeta::new(swap_accounts.amm_id.key(), false),
        AccountMeta::new_readonly(swap_accounts.amm_authority.key(), false),
        AccountMeta::new(swap_accounts.amm_open_orders.key(), false),
        AccountMeta::new(swap_accounts.pool_coin_token_account.key(), false),
        AccountMeta::new(swap_accounts.pool_pc_token_account.key(), false),
        AccountMeta::new(swap_accounts.model_data_account.key(), false),
        // serum
        AccountMeta::new_readonly(swap_accounts.serum_program_id.key(), false),
        AccountMeta::new(swap_accounts.serum_market.key(), false),
        AccountMeta::new(swap_accounts.serum_bids.key(), false),
        AccountMeta::new(swap_accounts.serum_asks.key(), false),
        AccountMeta::new(swap_accounts.serum_event_queue.key(), false),
        AccountMeta::new(swap_accounts.serum_coin_vault_account.key(), false),
        AccountMeta::new(swap_accounts.serum_pc_vault_account.key(), false),
        AccountMeta::new_readonly(swap_accounts.serum_vault_signer.key(), false),
        // user
        AccountMeta::new(swap_source_token, false),
        AccountMeta::new(swap_destination_token, false),
        AccountMeta::new_readonly(swap_accounts.swap_authority_pubkey.key(), true),
    ];

    let account_infos = vec![
        swap_accounts.token_program.to_account_info(),
        swap_accounts.amm_id.to_account_info(),
        swap_accounts.amm_authority.to_account_info(),
        swap_accounts.amm_open_orders.to_account_info(),
        swap_accounts.pool_coin_token_account.to_account_info(),
        swap_accounts.pool_pc_token_account.to_account_info(),
        swap_accounts.model_data_account.to_account_info(),
        swap_accounts.serum_program_id.to_account_info(),
        swap_accounts.serum_market.to_account_info(),
        swap_accounts.serum_bids.to_account_info(),
        swap_accounts.serum_asks.to_account_info(),
        swap_accounts.serum_event_queue.to_account_info(),
        swap_accounts.serum_coin_vault_account.to_account_info(),
        swap_accounts.serum_pc_vault_account.to_account_info(),
        swap_accounts.serum_vault_signer.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &RaydiumSwapProcessor;
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
        STABLE_ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )?;
    Ok(amount_out)
}

pub fn swap_clmm<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::RaydiumClmmSwap amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + CLMM_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );

    let mut swap_accounts = RaydiumClmmAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &raydium_clmm_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.pool_id.key().log();

    // check hop accounts & swap authority
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

    let is_base_input = true;
    let sqrt_price_limit_x64 = 0u128;
    let other_amount_threshold = 1u64;

    let mut data = Vec::with_capacity(ARGS_CLMM_LEN);
    data.extend_from_slice(SWAP_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&other_amount_threshold.to_le_bytes());
    data.extend_from_slice(&sqrt_price_limit_x64.to_le_bytes());
    data.extend_from_slice(&(is_base_input as u8).to_le_bytes());

    let mut accounts = vec![
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true), // payer
        AccountMeta::new_readonly(swap_accounts.amm_config_id.key(), false),
        AccountMeta::new(swap_accounts.pool_id.key(), false),
        AccountMeta::new(swap_source_token, false),
        AccountMeta::new(swap_destination_token, false),
        AccountMeta::new(swap_accounts.input_vault.key(), false),
        AccountMeta::new(swap_accounts.output_vault.key(), false),
        AccountMeta::new(swap_accounts.observation_id.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false), // spl token
        AccountMeta::new(swap_accounts.tick_array0.key(), false),
        AccountMeta::new(swap_accounts.ex_bitmap.key(), false),
    ];

    let mut account_infos = vec![
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.amm_config_id.to_account_info(),
        swap_accounts.pool_id.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.input_vault.to_account_info(),
        swap_accounts.output_vault.to_account_info(),
        swap_accounts.observation_id.to_account_info(),
        swap_accounts.token_program.to_account_info(),
        swap_accounts.tick_array0.to_account_info(),
        swap_accounts.ex_bitmap.to_account_info(),
    ];

    let tick_array1 = swap_accounts.tick_array1.key();
    let tick_array2 = swap_accounts.tick_array2.key();
    if tick_array1 != ZERO_ADDRESS {
        accounts.push(AccountMeta::new(tick_array1, false));
        account_infos.push(swap_accounts.tick_array1.to_account_info());
    }
    if tick_array2 != ZERO_ADDRESS {
        accounts.push(AccountMeta::new(tick_array2, false));
        account_infos.push(swap_accounts.tick_array2.to_account_info());
    }

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &RaydiumSwapProcessor;
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
        CLMM_ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )?;
    Ok(amount_out)
}

pub fn swap_clmm_v2<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::RaydiumClmmSwapV2 amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + CLMM_V2_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );

    let mut swap_accounts = RaydiumClmmV2Accounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &raydium_clmm_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.pool_id.key().log();

    // check hop accounts & swap authority
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

    let is_base_input = true;
    let sqrt_price_limit_x64 = 0u128;
    let other_amount_threshold = 1u64;

    let mut data = Vec::with_capacity(ARGS_CLMM_LEN);
    data.extend_from_slice(SWAP_V2_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&other_amount_threshold.to_le_bytes());
    data.extend_from_slice(&sqrt_price_limit_x64.to_le_bytes());
    data.extend_from_slice(&(is_base_input as u8).to_le_bytes());

    let mut accounts = vec![
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true), // payer
        AccountMeta::new_readonly(swap_accounts.amm_config_id.key(), false),
        AccountMeta::new(swap_accounts.pool_id.key(), false),
        AccountMeta::new(swap_source_token, false),
        AccountMeta::new(swap_destination_token, false),
        AccountMeta::new(swap_accounts.input_vault.key(), false),
        AccountMeta::new(swap_accounts.output_vault.key(), false),
        AccountMeta::new(swap_accounts.observation_id.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false), // spl token
        AccountMeta::new_readonly(swap_accounts.token_program_2022.key(), false), // token 2022
        AccountMeta::new_readonly(swap_accounts.memo_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.input_vault_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.output_vault_mint.key(), false),
        AccountMeta::new(swap_accounts.ex_bitmap.key(), false),
        AccountMeta::new(swap_accounts.tick_array0.key(), false),
    ];

    let mut account_infos = vec![
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.amm_config_id.to_account_info(),
        swap_accounts.pool_id.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.input_vault.to_account_info(),
        swap_accounts.output_vault.to_account_info(),
        swap_accounts.observation_id.to_account_info(),
        swap_accounts.token_program.to_account_info(),
        swap_accounts.token_program_2022.to_account_info(),
        swap_accounts.memo_program.to_account_info(),
        swap_accounts.input_vault_mint.to_account_info(),
        swap_accounts.output_vault_mint.to_account_info(),
        swap_accounts.ex_bitmap.to_account_info(),
        swap_accounts.tick_array0.to_account_info(),
    ];

    let tick_array1 = swap_accounts.tick_array1.key();
    let tick_array2 = swap_accounts.tick_array2.key();
    if tick_array1 != ZERO_ADDRESS {
        accounts.push(AccountMeta::new(tick_array1, false));
        account_infos.push(swap_accounts.tick_array1.to_account_info());
    }
    if tick_array2 != ZERO_ADDRESS {
        accounts.push(AccountMeta::new(tick_array2, false));
        account_infos.push(swap_accounts.tick_array2.to_account_info());
    }

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &RaydiumSwapProcessor;
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
        CLMM_V2_ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )?;
    Ok(amount_out)
}

pub fn swap_cpmm<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::RaydiumCpmmSwap amount_in: {}, offset: {}", amount_in, offset);

    require!(
        remaining_accounts.len() >= *offset + CPMM_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );

    let mut swap_accounts = RaydiumCpmmAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &raydium_cpmm_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.pool_state.key().log();

    // check hop accounts & swap authority
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

    let minimum_amount_out = 0u64;
    let mut data = Vec::with_capacity(ARGS_CPMM_LEN);
    data.extend_from_slice(CPSWAP_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.authority.key(), false),
        AccountMeta::new_readonly(swap_accounts.amm_config.key(), false),
        AccountMeta::new(swap_accounts.pool_state.key(), false),
        AccountMeta::new(swap_source_token, false),
        AccountMeta::new(swap_destination_token, false),
        AccountMeta::new(swap_accounts.input_vault.key(), false),
        AccountMeta::new(swap_accounts.output_vault.key(), false),
        AccountMeta::new_readonly(swap_accounts.input_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.output_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.input_token_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.output_token_mint.key(), false),
        AccountMeta::new(swap_accounts.observation_state.key(), false),
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

    let dex_processor = &RaydiumSwapProcessor;
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
        CPMM_ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )?;
    Ok(amount_out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_pack_swap_instruction() {
        let amount_in = 100u64;
        let mut data = Vec::with_capacity(ARGS_LEN);
        data.push(9);
        data.extend_from_slice(&amount_in.to_le_bytes());
        data.extend_from_slice(&1u64.to_le_bytes());

        msg!("data.len: {}", data.len());
        assert!(data.len() == ARGS_LEN);
    }

    #[test]
    pub fn test_pack_clmm_instruction() {
        let amount_in = 100u64;
        let is_base_input = true;
        let sqrt_price_limit_x64 = 0u128;
        let other_amount_threshold = 1u64;

        let mut data = Vec::with_capacity(ARGS_CLMM_LEN);
        data.extend_from_slice(SWAP_SELECTOR);
        data.extend_from_slice(&amount_in.to_le_bytes());
        data.extend_from_slice(&other_amount_threshold.to_le_bytes());
        data.extend_from_slice(&sqrt_price_limit_x64.to_le_bytes());
        data.extend_from_slice(&(is_base_input as u8).to_le_bytes());

        msg!("data.len: {}", data.len());
        assert!(data.len() == ARGS_CLMM_LEN);
    }

    #[test]
    pub fn test_pack_cpmm_instruction() {
        let amount_in = 100u64;
        let minimum_amount_out = 0u64;

        let mut data = Vec::with_capacity(ARGS_CPMM_LEN);
        data.extend_from_slice(CPSWAP_SELECTOR);
        data.extend_from_slice(&amount_in.to_le_bytes());
        data.extend_from_slice(&minimum_amount_out.to_le_bytes());

        msg!("data.len: {}", data.len());
        assert!(data.len() == ARGS_CPMM_LEN);
    }
}
