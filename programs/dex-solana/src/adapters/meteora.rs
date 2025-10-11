use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{
    DEPOSIT_SELECTOR, HopAccounts, SWAP_SELECTOR, SWAP2_SELECTOR, WITHDRAW_SELECTOR, ZERO_ADDRESS,
    meteora_damm_v2_program, meteora_dlmm_program, meteora_dynamicpool_program,
    meteora_vault_program,
};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token::Token;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use arrayref::array_ref;

use super::common::DexProcessor;

const ARGS_LEN: usize = 24;
const DLMM_SWAP2_ARGS_LEN: usize = 28;

pub struct MeteoraDynamicPoolProcessor;
impl DexProcessor for MeteoraDynamicPoolProcessor {}

pub struct MeteoraDynamicPoolAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub pool: &'info AccountInfo<'info>,
    pub a_vault: &'info AccountInfo<'info>,
    pub b_vault: &'info AccountInfo<'info>,
    pub a_token_vault: &'info AccountInfo<'info>,
    pub b_token_vault: &'info AccountInfo<'info>,
    pub a_vault_lp_mint: InterfaceAccount<'info, Mint>,
    pub b_vault_lp_mint: InterfaceAccount<'info, Mint>,
    pub a_vault_lp: InterfaceAccount<'info, TokenAccount>,
    pub b_vault_lp: InterfaceAccount<'info, TokenAccount>,
    pub admin_token_fee: &'info AccountInfo<'info>,
    pub vault_program: &'info AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}
const ACCOUNTS_LEN: usize = 16;

pub struct MeteoraLSTPoolAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub pool: &'info AccountInfo<'info>,
    pub a_vault: &'info AccountInfo<'info>,
    pub b_vault: &'info AccountInfo<'info>,
    pub a_token_vault: &'info AccountInfo<'info>,
    pub b_token_vault: &'info AccountInfo<'info>,
    pub a_vault_lp_mint: InterfaceAccount<'info, Mint>,
    pub b_vault_lp_mint: InterfaceAccount<'info, Mint>,
    pub a_vault_lp: InterfaceAccount<'info, TokenAccount>,
    pub b_vault_lp: InterfaceAccount<'info, TokenAccount>,
    pub admin_token_fee: &'info AccountInfo<'info>,
    pub vault_program: &'info AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
    pub lst: &'info AccountInfo<'info>,
}
const LST_ACCOUNTS_LEN: usize = 17;

pub struct MeteoraDlmmAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub lb_pair: &'info AccountInfo<'info>,
    pub bin_array_bitmap_extension: &'info AccountInfo<'info>,
    pub reserve_x: InterfaceAccount<'info, TokenAccount>,
    pub reserve_y: InterfaceAccount<'info, TokenAccount>,
    pub token_x_mint: InterfaceAccount<'info, Mint>,
    pub token_y_mint: InterfaceAccount<'info, Mint>,
    pub oracle: &'info AccountInfo<'info>,
    pub host_fee_in: &'info AccountInfo<'info>,
    pub token_x_program: &'info AccountInfo<'info>,
    pub token_y_program: &'info AccountInfo<'info>,
    pub event_authority: &'info AccountInfo<'info>,
    pub bin_array0: &'info AccountInfo<'info>,
    pub bin_array1: &'info AccountInfo<'info>,
    pub bin_array2: &'info AccountInfo<'info>,
}
const DLMM_ACCOUNTS_LEN: usize = 18;

pub struct MeteoraDlmmSwap2Accounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub lb_pair: &'info AccountInfo<'info>,
    pub bin_array_bitmap_extension: &'info AccountInfo<'info>,
    pub reserve_x: InterfaceAccount<'info, TokenAccount>,
    pub reserve_y: InterfaceAccount<'info, TokenAccount>,
    pub token_x_mint: InterfaceAccount<'info, Mint>,
    pub token_y_mint: InterfaceAccount<'info, Mint>,
    pub oracle: &'info AccountInfo<'info>,
    pub host_fee_in: &'info AccountInfo<'info>,
    pub token_x_program: &'info AccountInfo<'info>,
    pub token_y_program: &'info AccountInfo<'info>,
    pub memo_program: &'info AccountInfo<'info>,
    pub event_authority: &'info AccountInfo<'info>,
    pub bin_array0: &'info AccountInfo<'info>,
    pub bin_array1: &'info AccountInfo<'info>,
    pub bin_array2: &'info AccountInfo<'info>,
}
const DLMM_SWAP2_ACCOUNTS_LEN: usize = 19;

pub struct MeteoraDynamicVaultAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub vault: &'info AccountInfo<'info>,
    pub token_vault: InterfaceAccount<'info, TokenAccount>,
    pub lp_mint: InterfaceAccount<'info, Mint>,
    pub token_program: Program<'info, Token>,
}
const VAULT_ACCOUNTS_LEN: usize = 8;

// https://github.com/MeteoraAg/cp-amm/blob/main/programs/cp-amm/src/instructions/ix_swap.rs#L22-L70
pub struct MeteoraDAMMV2SwapAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub pool_authority: &'info AccountInfo<'info>,
    pub pool: &'info AccountInfo<'info>,
    pub input_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub output_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub token_a_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    pub token_b_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    pub token_a_mint: InterfaceAccount<'info, Mint>,
    pub token_b_mint: InterfaceAccount<'info, Mint>,
    pub token_a_program: Interface<'info, TokenInterface>,
    pub token_b_program: Interface<'info, TokenInterface>,
    pub referral_token_account: &'info AccountInfo<'info>,
    pub event_authority: &'info AccountInfo<'info>,
}

const DAMMV2_ACCOUNTS_LEN: usize = 16;

pub struct MeteoraDAMMV2Swap2Accounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority: &'info AccountInfo<'info>,
    pub swap_source_account: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_account: InterfaceAccount<'info, TokenAccount>,

    pub pool_authority: &'info AccountInfo<'info>,
    pub pool: &'info AccountInfo<'info>,
    pub token_a_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    pub token_b_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    pub token_a_mint: InterfaceAccount<'info, Mint>,
    pub token_b_mint: InterfaceAccount<'info, Mint>,
    pub token_a_program: Interface<'info, TokenInterface>,
    pub token_b_program: Interface<'info, TokenInterface>,
    pub referral_token_account: &'info AccountInfo<'info>,
    pub event_authority: &'info AccountInfo<'info>,
    pub instruction_sysvar: &'info AccountInfo<'info>,
}

const DAMMV2_SWAP2_ACCOUNTS_LEN: usize = 15;

impl<'info> MeteoraDynamicPoolAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            pool,
            a_vault,
            b_vault,
            a_token_vault,
            b_token_vault,
            a_vault_lp_mint,
            b_vault_lp_mint,
            a_vault_lp,
            b_vault_lp,
            admin_token_fee,
            vault_program,
            token_program,
        ]: &[AccountInfo<'info>; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];
        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            pool,
            a_vault,
            b_vault,
            a_token_vault,
            b_token_vault,
            a_vault_lp_mint: InterfaceAccount::try_from(a_vault_lp_mint)?,
            b_vault_lp_mint: InterfaceAccount::try_from(b_vault_lp_mint)?,
            a_vault_lp: InterfaceAccount::try_from(a_vault_lp)?,
            b_vault_lp: InterfaceAccount::try_from(b_vault_lp)?,
            admin_token_fee,
            vault_program,
            token_program: Program::try_from(token_program)?,
        })
    }
}

impl<'info> MeteoraLSTPoolAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            pool,
            a_vault,
            b_vault,
            a_token_vault,
            b_token_vault,
            a_vault_lp_mint,
            b_vault_lp_mint,
            a_vault_lp,
            b_vault_lp,
            admin_token_fee,
            vault_program,
            token_program,
            lst,
        ]: &[AccountInfo<'info>; LST_ACCOUNTS_LEN] = array_ref![accounts, offset, LST_ACCOUNTS_LEN];
        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            pool,
            a_vault,
            b_vault,
            a_token_vault,
            b_token_vault,
            a_vault_lp_mint: InterfaceAccount::try_from(a_vault_lp_mint)?,
            b_vault_lp_mint: InterfaceAccount::try_from(b_vault_lp_mint)?,
            a_vault_lp: InterfaceAccount::try_from(a_vault_lp)?,
            b_vault_lp: InterfaceAccount::try_from(b_vault_lp)?,
            admin_token_fee,
            vault_program,
            token_program: Program::try_from(token_program)?,
            lst,
        })
    }
}

impl<'info> MeteoraDlmmAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            lb_pair,
            bin_array_bitmap_extension,
            reserve_x,
            reserve_y,
            token_x_mint,
            token_y_mint,
            oracle,
            host_fee_in,
            token_x_program,
            token_y_program,
            event_authority,
            bin_array0,
            bin_array1,
            bin_array2,
        ]: &[AccountInfo<'info>; DLMM_ACCOUNTS_LEN] =
            array_ref![accounts, offset, DLMM_ACCOUNTS_LEN];
        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            lb_pair,
            bin_array_bitmap_extension,
            reserve_x: InterfaceAccount::try_from(reserve_x)?,
            reserve_y: InterfaceAccount::try_from(reserve_y)?,
            token_x_mint: InterfaceAccount::try_from(token_x_mint)?,
            token_y_mint: InterfaceAccount::try_from(token_y_mint)?,
            oracle,
            host_fee_in,
            token_x_program,
            token_y_program,
            event_authority,
            bin_array0,
            bin_array1,
            bin_array2,
        })
    }
}

impl<'info> MeteoraDlmmSwap2Accounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            lb_pair,
            bin_array_bitmap_extension,
            reserve_x,
            reserve_y,
            token_x_mint,
            token_y_mint,
            oracle,
            host_fee_in,
            token_x_program,
            token_y_program,
            memo_program,
            event_authority,
            bin_array0,
            bin_array1,
            bin_array2,
        ]: &[AccountInfo<'info>; DLMM_SWAP2_ACCOUNTS_LEN] =
            array_ref![accounts, offset, DLMM_SWAP2_ACCOUNTS_LEN];
        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            lb_pair,
            bin_array_bitmap_extension,
            reserve_x: InterfaceAccount::try_from(reserve_x)?,
            reserve_y: InterfaceAccount::try_from(reserve_y)?,
            token_x_mint: InterfaceAccount::try_from(token_x_mint)?,
            token_y_mint: InterfaceAccount::try_from(token_y_mint)?,
            oracle,
            host_fee_in,
            token_x_program,
            token_y_program,
            memo_program,
            event_authority,
            bin_array0,
            bin_array1,
            bin_array2,
        })
    }
}

impl<'info> MeteoraDynamicVaultAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            vault,
            token_vault,
            lp_mint,
            token_program,
        ]: &[AccountInfo<'info>; VAULT_ACCOUNTS_LEN] =
            array_ref![accounts, offset, VAULT_ACCOUNTS_LEN];
        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            vault,
            token_vault: InterfaceAccount::try_from(token_vault)?,
            lp_mint: InterfaceAccount::try_from(lp_mint)?,
            token_program: Program::try_from(token_program)?,
        })
    }
}

impl<'info> MeteoraDAMMV2SwapAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            pool_authority,
            pool,
            input_token_account,
            output_token_account,
            token_a_vault,
            token_b_vault,
            token_a_mint,
            token_b_mint,
            token_a_program,
            token_b_program,
            referral_token_account,
            event_authority,
        ]: &[AccountInfo<'info>; DAMMV2_ACCOUNTS_LEN] =
            array_ref![accounts, offset, DAMMV2_ACCOUNTS_LEN];
        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            pool_authority,
            pool,
            input_token_account: Box::new(InterfaceAccount::try_from(input_token_account)?),
            output_token_account: Box::new(InterfaceAccount::try_from(output_token_account)?),
            token_a_vault: Box::new(InterfaceAccount::try_from(token_a_vault)?),
            token_b_vault: Box::new(InterfaceAccount::try_from(token_b_vault)?),
            token_a_mint: InterfaceAccount::try_from(token_a_mint)?,
            token_b_mint: InterfaceAccount::try_from(token_b_mint)?,
            token_a_program: Interface::try_from(token_a_program)?,
            token_b_program: Interface::try_from(token_b_program)?,
            referral_token_account,
            event_authority,
        })
    }
}

impl<'info> MeteoraDAMMV2Swap2Accounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority,
            swap_source_account,
            swap_destination_account,
            pool_authority,
            pool,
            token_a_vault,
            token_b_vault,
            token_a_mint,
            token_b_mint,
            token_a_program,
            token_b_program,
            referral_token_account,
            event_authority,
            instruction_sysvar,
        ]: &[AccountInfo<'info>; DAMMV2_SWAP2_ACCOUNTS_LEN] =
            array_ref![accounts, offset, DAMMV2_SWAP2_ACCOUNTS_LEN];
        Ok(Self {
            dex_program_id,
            swap_authority,
            swap_source_account: InterfaceAccount::try_from(swap_source_account)?,
            swap_destination_account: InterfaceAccount::try_from(swap_destination_account)?,
            pool_authority,
            pool,
            token_a_vault: Box::new(InterfaceAccount::try_from(token_a_vault)?),
            token_b_vault: Box::new(InterfaceAccount::try_from(token_b_vault)?),
            token_a_mint: InterfaceAccount::try_from(token_a_mint)?,
            token_b_mint: InterfaceAccount::try_from(token_b_mint)?,
            token_a_program: Interface::try_from(token_a_program)?,
            token_b_program: Interface::try_from(token_b_program)?,
            referral_token_account,
            event_authority,
            instruction_sysvar,
        })
    }
}

pub fn deposit<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::MeteoraVaultDeposit amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + VAULT_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );
    let mut swap_accounts =
        MeteoraDynamicVaultAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &meteora_vault_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.vault.key().log();

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
    data.extend_from_slice(DEPOSIT_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&1u64.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(swap_accounts.vault.key(), false),
        AccountMeta::new(swap_accounts.token_vault.key(), false),
        AccountMeta::new(swap_accounts.lp_mint.key(), false),
        AccountMeta::new(swap_source_token, false),
        AccountMeta::new(swap_destination_token, false),
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.vault.to_account_info(),
        swap_accounts.token_vault.to_account_info(),
        swap_accounts.lp_mint.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.token_program.to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &MeteoraDynamicPoolProcessor;
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
        VAULT_ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )?;
    Ok(amount_out)
}

pub fn withdraw<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::MeteoraVaultWithdraw amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + VAULT_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );
    let mut swap_accounts =
        MeteoraDynamicVaultAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &meteora_vault_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.vault.key().log();

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
    data.extend_from_slice(WITHDRAW_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&1u64.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(swap_accounts.vault.key(), false),
        AccountMeta::new(swap_accounts.token_vault.key(), false),
        AccountMeta::new(swap_accounts.lp_mint.key(), false),
        AccountMeta::new(swap_destination_token, false),
        AccountMeta::new(swap_source_token, false),
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.vault.to_account_info(),
        swap_accounts.token_vault.to_account_info(),
        swap_accounts.lp_mint.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.token_program.to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &MeteoraDynamicPoolProcessor;
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
        VAULT_ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )?;
    Ok(amount_out)
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
    msg!("Dex::MeteoraSwap amount_in: {}, offset: {}", amount_in, offset);
    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);
    let mut swap_accounts =
        MeteoraDynamicPoolAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &meteora_dynamicpool_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.pool.key().log();

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
    data.extend_from_slice(SWAP_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&1u64.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(swap_accounts.pool.key(), false),
        AccountMeta::new(swap_source_token, false),
        AccountMeta::new(swap_destination_token, false),
        AccountMeta::new(swap_accounts.a_vault.key(), false),
        AccountMeta::new(swap_accounts.b_vault.key(), false),
        AccountMeta::new(swap_accounts.a_token_vault.key(), false),
        AccountMeta::new(swap_accounts.b_token_vault.key(), false),
        AccountMeta::new(swap_accounts.a_vault_lp_mint.key(), false),
        AccountMeta::new(swap_accounts.b_vault_lp_mint.key(), false),
        AccountMeta::new(swap_accounts.a_vault_lp.key(), false),
        AccountMeta::new(swap_accounts.b_vault_lp.key(), false),
        AccountMeta::new(swap_accounts.admin_token_fee.key(), false),
        AccountMeta::new_readonly(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.vault_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.pool.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.a_vault.to_account_info(),
        swap_accounts.b_vault.to_account_info(),
        swap_accounts.a_token_vault.to_account_info(),
        swap_accounts.b_token_vault.to_account_info(),
        swap_accounts.a_vault_lp_mint.to_account_info(),
        swap_accounts.b_vault_lp_mint.to_account_info(),
        swap_accounts.a_vault_lp.to_account_info(),
        swap_accounts.b_vault_lp.to_account_info(),
        swap_accounts.admin_token_fee.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.vault_program.to_account_info(),
        swap_accounts.token_program.to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &MeteoraDynamicPoolProcessor;
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

pub fn swap_lst<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::MeteoraSwapLst amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + LST_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );
    let mut swap_accounts = MeteoraLSTPoolAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &meteora_dynamicpool_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.pool.key().log();

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
    data.extend_from_slice(SWAP_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&1u64.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(swap_accounts.pool.key(), false),
        AccountMeta::new(swap_source_token, false),
        AccountMeta::new(swap_destination_token, false),
        AccountMeta::new(swap_accounts.a_vault.key(), false),
        AccountMeta::new(swap_accounts.b_vault.key(), false),
        AccountMeta::new(swap_accounts.a_token_vault.key(), false),
        AccountMeta::new(swap_accounts.b_token_vault.key(), false),
        AccountMeta::new(swap_accounts.a_vault_lp_mint.key(), false),
        AccountMeta::new(swap_accounts.b_vault_lp_mint.key(), false),
        AccountMeta::new(swap_accounts.a_vault_lp.key(), false),
        AccountMeta::new(swap_accounts.b_vault_lp.key(), false),
        AccountMeta::new(swap_accounts.admin_token_fee.key(), false),
        AccountMeta::new_readonly(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.vault_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.lst.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.pool.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.a_vault.to_account_info(),
        swap_accounts.b_vault.to_account_info(),
        swap_accounts.a_token_vault.to_account_info(),
        swap_accounts.b_token_vault.to_account_info(),
        swap_accounts.a_vault_lp_mint.to_account_info(),
        swap_accounts.b_vault_lp_mint.to_account_info(),
        swap_accounts.a_vault_lp.to_account_info(),
        swap_accounts.b_vault_lp.to_account_info(),
        swap_accounts.admin_token_fee.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.vault_program.to_account_info(),
        swap_accounts.token_program.to_account_info(),
        swap_accounts.lst.to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &MeteoraDynamicPoolProcessor;
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
        LST_ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )?;
    Ok(amount_out)
}

pub fn dlmm_swap<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::MeteoraDlmm amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + DLMM_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );
    let mut swap_accounts = MeteoraDlmmAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &meteora_dlmm_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.lb_pair.key().log();

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
    data.extend_from_slice(SWAP_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&1u64.to_le_bytes());

    let mut accounts = vec![
        AccountMeta::new(swap_accounts.lb_pair.key(), false),
        AccountMeta::new_readonly(swap_accounts.bin_array_bitmap_extension.key(), false),
        AccountMeta::new(swap_accounts.reserve_x.key(), false),
        AccountMeta::new(swap_accounts.reserve_y.key(), false),
        AccountMeta::new(swap_source_token, false),
        AccountMeta::new(swap_destination_token, false),
        AccountMeta::new_readonly(swap_accounts.token_x_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_y_mint.key(), false),
        AccountMeta::new(swap_accounts.oracle.key(), false),
        AccountMeta::new(swap_accounts.host_fee_in.key(), false),
        AccountMeta::new_readonly(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.token_x_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_y_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.event_authority.key(), false),
        AccountMeta::new_readonly(swap_accounts.dex_program_id.key(), false),
        AccountMeta::new(swap_accounts.bin_array0.key(), false),
    ];

    let mut account_infos = vec![
        swap_accounts.lb_pair.to_account_info(),
        swap_accounts.bin_array_bitmap_extension.to_account_info(),
        swap_accounts.reserve_x.to_account_info(),
        swap_accounts.reserve_y.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.token_x_mint.to_account_info(),
        swap_accounts.token_y_mint.to_account_info(),
        swap_accounts.oracle.to_account_info(),
        swap_accounts.host_fee_in.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.token_x_program.to_account_info(),
        swap_accounts.token_y_program.to_account_info(),
        swap_accounts.event_authority.to_account_info(),
        swap_accounts.dex_program_id.to_account_info(),
        swap_accounts.bin_array0.to_account_info(),
    ];

    let bin_array1 = swap_accounts.bin_array1.key();
    let bin_array2 = swap_accounts.bin_array2.key();
    if bin_array1 != ZERO_ADDRESS {
        accounts.push(AccountMeta::new(bin_array1, false));
        account_infos.push(swap_accounts.bin_array1.to_account_info());
    }
    if bin_array2 != ZERO_ADDRESS {
        accounts.push(AccountMeta::new(bin_array2, false));
        account_infos.push(swap_accounts.bin_array2.to_account_info());
    }

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &MeteoraDynamicPoolProcessor;
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
        DLMM_ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )?;
    Ok(amount_out)
}

pub fn dlmm_swap2<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::MeteoraDlmmSwap2 amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + DLMM_SWAP2_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );
    let mut swap_accounts = MeteoraDlmmSwap2Accounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &meteora_dlmm_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.lb_pair.key().log();

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

    let mut data = Vec::with_capacity(DLMM_SWAP2_ARGS_LEN);
    data.extend_from_slice(SWAP2_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&1u64.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());

    let mut accounts = vec![
        AccountMeta::new(swap_accounts.lb_pair.key(), false),
        AccountMeta::new_readonly(swap_accounts.bin_array_bitmap_extension.key(), false),
        AccountMeta::new(swap_accounts.reserve_x.key(), false),
        AccountMeta::new(swap_accounts.reserve_y.key(), false),
        AccountMeta::new(swap_source_token, false),
        AccountMeta::new(swap_destination_token, false),
        AccountMeta::new_readonly(swap_accounts.token_x_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_y_mint.key(), false),
        AccountMeta::new(swap_accounts.oracle.key(), false),
        AccountMeta::new(swap_accounts.host_fee_in.key(), false),
        AccountMeta::new_readonly(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.token_x_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_y_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.memo_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.event_authority.key(), false),
        AccountMeta::new_readonly(swap_accounts.dex_program_id.key(), false),
        AccountMeta::new(swap_accounts.bin_array0.key(), false),
    ];

    let mut account_infos = vec![
        swap_accounts.lb_pair.to_account_info(),
        swap_accounts.bin_array_bitmap_extension.to_account_info(),
        swap_accounts.reserve_x.to_account_info(),
        swap_accounts.reserve_y.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.token_x_mint.to_account_info(),
        swap_accounts.token_y_mint.to_account_info(),
        swap_accounts.oracle.to_account_info(),
        swap_accounts.host_fee_in.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.token_x_program.to_account_info(),
        swap_accounts.token_y_program.to_account_info(),
        swap_accounts.memo_program.to_account_info(),
        swap_accounts.event_authority.to_account_info(),
        swap_accounts.dex_program_id.to_account_info(),
        swap_accounts.bin_array0.to_account_info(),
    ];

    let bin_array1 = swap_accounts.bin_array1.key();
    let bin_array2 = swap_accounts.bin_array2.key();
    if bin_array1 != ZERO_ADDRESS {
        accounts.push(AccountMeta::new(bin_array1, false));
        account_infos.push(swap_accounts.bin_array1.to_account_info());
    }
    if bin_array2 != ZERO_ADDRESS {
        accounts.push(AccountMeta::new(bin_array2, false));
        account_infos.push(swap_accounts.bin_array2.to_account_info());
    }

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &MeteoraDynamicPoolProcessor;
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
        DLMM_SWAP2_ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )?;
    Ok(amount_out)
}

pub fn damm_v2_swap<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::MeteoraDAMMV2 amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + DAMMV2_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );

    let mut swap_accounts = MeteoraDAMMV2SwapAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &meteora_damm_v2_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    swap_accounts.pool.key().log();

    let swap_destination_token = swap_accounts.output_token_account.key();

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
    data.extend_from_slice(SWAP_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&1u64.to_le_bytes());

    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.pool_authority.key(), false),
        AccountMeta::new(swap_accounts.pool.key(), false),
        AccountMeta::new(swap_accounts.input_token_account.key(), false),
        AccountMeta::new(swap_accounts.output_token_account.key(), false),
        AccountMeta::new(swap_accounts.token_a_vault.key(), false),
        AccountMeta::new(swap_accounts.token_b_vault.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_a_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_b_mint.key(), false),
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.token_a_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_b_program.key(), false),
        if swap_accounts.referral_token_account.key() != swap_accounts.dex_program_id.key() {
            AccountMeta::new(swap_accounts.referral_token_account.key(), false)
        } else {
            AccountMeta::new_readonly(swap_accounts.referral_token_account.key(), false)
        },
        AccountMeta::new_readonly(swap_accounts.event_authority.key(), false),
        AccountMeta::new_readonly(swap_accounts.dex_program_id.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.pool_authority.to_account_info(),
        swap_accounts.pool.to_account_info(),
        swap_accounts.input_token_account.to_account_info(),
        swap_accounts.output_token_account.to_account_info(),
        swap_accounts.token_a_vault.to_account_info(),
        swap_accounts.token_b_vault.to_account_info(),
        swap_accounts.token_a_mint.to_account_info(),
        swap_accounts.token_b_mint.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.token_a_program.to_account_info(),
        swap_accounts.token_b_program.to_account_info(),
        swap_accounts.referral_token_account.to_account_info(),
        swap_accounts.event_authority.to_account_info(),
        swap_accounts.dex_program_id.to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &MeteoraDynamicPoolProcessor;
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
        DAMMV2_ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )?;

    Ok(amount_out)
}

pub fn damm_v2_swap2<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::MeteoraDAMMV2Swap2 amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + DAMMV2_SWAP2_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );

    let mut swap_accounts =
        MeteoraDAMMV2Swap2Accounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &meteora_damm_v2_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    swap_accounts.pool.key().log();

    let swap_destination_token = swap_accounts.swap_destination_account.key();

    before_check(
        &swap_accounts.swap_authority,
        &swap_accounts.swap_source_account,
        swap_destination_token,
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(SWAP2_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes()); // amount_0: When it's exact in, this will be amount_in.
    data.extend_from_slice(&1u64.to_le_bytes()); // amount_1: When it's exact in, this will be minimum_amount_out.
    data.extend_from_slice(&0u8.to_le_bytes()); // swap_mode: 0 - ExactIn, 1 - PartialFill, 2 - ExactOut,

    let mut accounts = Vec::with_capacity(DAMMV2_SWAP2_ACCOUNTS_LEN);
    accounts.push(AccountMeta::new_readonly(swap_accounts.pool_authority.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.pool.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.swap_source_account.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.swap_destination_account.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.token_a_vault.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.token_b_vault.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.token_a_mint.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.token_b_mint.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.swap_authority.key(), true));
    accounts.push(AccountMeta::new_readonly(swap_accounts.token_a_program.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.token_b_program.key(), false));
    accounts.push(
        if swap_accounts.referral_token_account.key() != swap_accounts.dex_program_id.key() {
            AccountMeta::new(swap_accounts.referral_token_account.key(), false)
        } else {
            AccountMeta::new_readonly(swap_accounts.referral_token_account.key(), false)
        },
    );
    accounts.push(AccountMeta::new_readonly(swap_accounts.event_authority.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.dex_program_id.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.instruction_sysvar.key(), false));

    let mut account_infos = Vec::with_capacity(DAMMV2_SWAP2_ACCOUNTS_LEN);
    account_infos.push(swap_accounts.pool_authority.to_account_info());
    account_infos.push(swap_accounts.pool.to_account_info());
    account_infos.push(swap_accounts.swap_source_account.to_account_info());
    account_infos.push(swap_accounts.swap_destination_account.to_account_info());
    account_infos.push(swap_accounts.token_a_vault.to_account_info());
    account_infos.push(swap_accounts.token_b_vault.to_account_info());
    account_infos.push(swap_accounts.token_a_mint.to_account_info());
    account_infos.push(swap_accounts.token_b_mint.to_account_info());
    account_infos.push(swap_accounts.swap_authority.to_account_info());
    account_infos.push(swap_accounts.token_a_program.to_account_info());
    account_infos.push(swap_accounts.token_b_program.to_account_info());
    account_infos.push(swap_accounts.referral_token_account.to_account_info());
    account_infos.push(swap_accounts.event_authority.to_account_info());
    account_infos.push(swap_accounts.dex_program_id.to_account_info());
    account_infos.push(swap_accounts.instruction_sysvar.to_account_info());

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &MeteoraDynamicPoolProcessor;
    let amount_out = invoke_process(
        amount_in,
        dex_processor,
        &account_infos,
        &mut swap_accounts.swap_source_account,
        &mut swap_accounts.swap_destination_account,
        hop_accounts,
        instruction,
        hop,
        offset,
        DAMMV2_SWAP2_ACCOUNTS_LEN,
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
        data.extend_from_slice(SWAP_SELECTOR);
        data.extend_from_slice(&amount_in.to_le_bytes());
        data.extend_from_slice(&1u64.to_le_bytes());

        msg!("data.len: {}", data.len());
        assert!(data.len() == ARGS_LEN);
    }
}
