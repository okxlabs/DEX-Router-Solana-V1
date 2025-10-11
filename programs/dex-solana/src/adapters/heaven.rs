use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{HEAVEN_BUY_SELECTOR, HEAVEN_SELL_SELECTOR, HopAccounts, heaven_program};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token_interface::TokenAccount;
use arrayref::array_ref;

use super::common::DexProcessor;

const ARGS_LEN: usize = 28;

struct HeavenProcessor;
impl DexProcessor for HeavenProcessor {}

pub struct HeavenSwapAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub token_a_program: &'info AccountInfo<'info>,
    pub token_b_program: &'info AccountInfo<'info>,
    pub associated_token_program: &'info AccountInfo<'info>,
    pub system_program: &'info AccountInfo<'info>,
    pub liquidity_pool_state: &'info AccountInfo<'info>,
    pub user: &'info AccountInfo<'info>,
    pub token_a_mint: &'info AccountInfo<'info>,
    pub token_b_mint: &'info AccountInfo<'info>,
    pub user_token_a_vault: &'info AccountInfo<'info>,
    pub user_token_b_vault: &'info AccountInfo<'info>,
    pub token_a_vault: &'info AccountInfo<'info>,
    pub token_b_vault: &'info AccountInfo<'info>,
    pub protocol_config: &'info AccountInfo<'info>,
    pub instruction_sysvar_account_info: &'info AccountInfo<'info>,
    pub chainlink_program: &'info AccountInfo<'info>,
    pub chainlink_sol_usd_feed: &'info AccountInfo<'info>,
}

const ACCOUNTS_LEN: usize = 20;

impl<'info> HeavenSwapAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            token_a_program,
            token_b_program,
            associated_token_program,
            system_program,
            liquidity_pool_state,
            user,
            token_a_mint,
            token_b_mint,
            user_token_a_vault,
            user_token_b_vault,
            token_a_vault,
            token_b_vault,
            protocol_config,
            instruction_sysvar_account_info,
            chainlink_program,
            chainlink_sol_usd_feed,
        ] = array_ref![accounts, offset, ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            token_a_program,
            token_b_program,
            associated_token_program,
            system_program,
            liquidity_pool_state,
            user,
            token_a_mint,
            token_b_mint,
            user_token_a_vault,
            user_token_b_vault,
            token_a_vault,
            token_b_vault,
            protocol_config,
            instruction_sysvar_account_info,
            chainlink_program,
            chainlink_sol_usd_feed,
        })
    }
}

pub fn swap_handler<'a>(
    is_buy: bool,
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::Heaven amount_in: {}, offset: {}", amount_in, offset);
    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);
    let mut swap_accounts = HeavenSwapAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &heaven_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.liquidity_pool_state.key().log();

    before_check(
        &swap_accounts.swap_authority_pubkey,
        &swap_accounts.swap_source_token,
        swap_accounts.swap_destination_token.key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(if is_buy { HEAVEN_BUY_SELECTOR } else { HEAVEN_SELL_SELECTOR });
    data.extend_from_slice(&amount_in.to_le_bytes()); // amount_in
    data.extend_from_slice(&1u64.to_le_bytes()); // minimum_amount_out
    data.extend_from_slice(&0u32.to_le_bytes()); // encoded_user_defined_event_data

    let account_metas = vec![
        AccountMeta::new_readonly(swap_accounts.token_a_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_b_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.associated_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.system_program.key(), false),
        AccountMeta::new(swap_accounts.liquidity_pool_state.key(), false),
        AccountMeta::new(swap_accounts.user.key(), true),
        AccountMeta::new_readonly(swap_accounts.token_a_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_b_mint.key(), false),
        AccountMeta::new(swap_accounts.user_token_a_vault.key(), false),
        AccountMeta::new(swap_accounts.user_token_b_vault.key(), false),
        AccountMeta::new(swap_accounts.token_a_vault.key(), false),
        AccountMeta::new(swap_accounts.token_b_vault.key(), false),
        AccountMeta::new(swap_accounts.protocol_config.key(), false),
        AccountMeta::new_readonly(swap_accounts.instruction_sysvar_account_info.key(), false),
        AccountMeta::new_readonly(swap_accounts.chainlink_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.chainlink_sol_usd_feed.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.token_a_program.to_account_info(),
        swap_accounts.token_b_program.to_account_info(),
        swap_accounts.associated_token_program.to_account_info(),
        swap_accounts.system_program.to_account_info(),
        swap_accounts.liquidity_pool_state.to_account_info(),
        swap_accounts.user.to_account_info(),
        swap_accounts.token_a_mint.to_account_info(),
        swap_accounts.token_b_mint.to_account_info(),
        swap_accounts.user_token_a_vault.to_account_info(),
        swap_accounts.user_token_b_vault.to_account_info(),
        swap_accounts.token_a_vault.to_account_info(),
        swap_accounts.token_b_vault.to_account_info(),
        swap_accounts.protocol_config.to_account_info(),
        swap_accounts.instruction_sysvar_account_info.to_account_info(),
        swap_accounts.chainlink_program.to_account_info(),
        swap_accounts.chainlink_sol_usd_feed.to_account_info(),
    ];

    let instruction = Instruction {
        program_id: swap_accounts.dex_program_id.key(),
        accounts: account_metas,
        data,
    };

    let dex_processor = &HeavenProcessor;
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

pub fn buy<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    swap_handler(
        true,
        remaining_accounts,
        amount_in,
        offset,
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )
}

pub fn sell<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    swap_handler(
        false,
        remaining_accounts,
        amount_in,
        offset,
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )
}
