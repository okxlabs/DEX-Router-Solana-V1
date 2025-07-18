use super::common::DexProcessor;
use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{
    raydium_launchpad_program, HopAccounts, RAYDIUM_LAUNCHPAD_BUY_SELECTOR,
    RAYDIUM_LAUNCHPAD_SELL_SELECTOR, letsbonk_platform_config,
};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use arrayref::array_ref;

pub struct LaunchpadProcessor;
impl DexProcessor for LaunchpadProcessor {}

const LAUNCHPAD_ACCOUNTS_LEN: usize = 15;
pub struct LaunchpadAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub launchpad_authority: &'info AccountInfo<'info>,
    pub global_config: &'info AccountInfo<'info>,
    pub platform_config: &'info AccountInfo<'info>,
    pub pool_state: &'info AccountInfo<'info>,
    pub base_vault: InterfaceAccount<'info, TokenAccount>,
    pub quote_vault: InterfaceAccount<'info, TokenAccount>,
    pub base_mint: InterfaceAccount<'info, Mint>,
    pub quote_mint: InterfaceAccount<'info, Mint>,
    pub base_token_program: Interface<'info, TokenInterface>,
    pub quote_token_program: Interface<'info, TokenInterface>,
    pub event_authority: &'info AccountInfo<'info>,
}

impl<'info> LaunchpadAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            launchpad_authority,
            global_config,
            platform_config,
            pool_state,
            base_vault,
            quote_vault,
            base_mint,
            quote_mint,
            base_token_program,
            quote_token_program,
            event_authority,
        ]: &[AccountInfo<'info>; LAUNCHPAD_ACCOUNTS_LEN] = array_ref![accounts, offset, LAUNCHPAD_ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            launchpad_authority,
            global_config,
            platform_config,
            pool_state,
            base_vault: InterfaceAccount::try_from(base_vault)?,
            quote_vault: InterfaceAccount::try_from(quote_vault)?,
            base_mint: InterfaceAccount::try_from(base_mint)?,
            quote_mint: InterfaceAccount::try_from(quote_mint)?,
            base_token_program: Interface::try_from(base_token_program)?,
            quote_token_program: Interface::try_from(quote_token_program)?,
            event_authority,
        })
    }
}

pub fn launchpad_handler<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    require!(
        remaining_accounts.len() >= *offset + LAUNCHPAD_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );
    let mut swap_accounts = LaunchpadAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &raydium_launchpad_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    let platform_name = if swap_accounts.platform_config.key != &letsbonk_platform_config::id() {
        "RaydiumLaunchpad"
    } else {
        "LetsBonkFun"
    };
    msg!("Dex::{} amount_in: {}, offset: {}", platform_name, amount_in, offset);

    swap_accounts.pool_state.key().log();

    let swap_base_token;
    let swap_quote_token;
    let mut data = Vec::with_capacity(32);
    if swap_accounts.swap_source_token.mint.eq(&swap_accounts.quote_mint.key()) {
        swap_base_token = &swap_accounts.swap_destination_token;
        swap_quote_token = &swap_accounts.swap_source_token;
        data.extend_from_slice(RAYDIUM_LAUNCHPAD_BUY_SELECTOR);
    } else {
        swap_base_token = &swap_accounts.swap_source_token;
        swap_quote_token = &swap_accounts.swap_destination_token;
        data.extend_from_slice(RAYDIUM_LAUNCHPAD_SELL_SELECTOR);
    }

    before_check(
        &swap_accounts.swap_authority_pubkey,
        &swap_accounts.swap_source_token,
        swap_accounts.swap_destination_token.key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    swap_accounts.dex_program_id.key().log();

    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&1u64.to_le_bytes()); //minimum_amount_out
    data.extend_from_slice(&0u64.to_le_bytes()); //share_fee_rate

    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.launchpad_authority.key(), false),
        AccountMeta::new_readonly(swap_accounts.global_config.key(), false),
        AccountMeta::new_readonly(swap_accounts.platform_config.key(), false),
        AccountMeta::new(swap_accounts.pool_state.key(), false),
        AccountMeta::new(swap_base_token.key(), false),
        AccountMeta::new(swap_quote_token.key(), false),
        AccountMeta::new(swap_accounts.base_vault.key(), false),
        AccountMeta::new(swap_accounts.quote_vault.key(), false),
        AccountMeta::new_readonly(swap_accounts.base_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.quote_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.base_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.quote_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.event_authority.key(), false),
        AccountMeta::new_readonly(swap_accounts.dex_program_id.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.launchpad_authority.to_account_info(),
        swap_accounts.global_config.to_account_info(),
        swap_accounts.platform_config.to_account_info(),
        swap_accounts.pool_state.to_account_info(),
        swap_base_token.to_account_info(),
        swap_quote_token.to_account_info(),
        swap_accounts.base_vault.to_account_info(),
        swap_accounts.quote_vault.to_account_info(),
        swap_accounts.base_mint.to_account_info(),
        swap_accounts.quote_mint.to_account_info(),
        swap_accounts.base_token_program.to_account_info(),
        swap_accounts.quote_token_program.to_account_info(),
        swap_accounts.event_authority.to_account_info(),
        swap_accounts.dex_program_id.to_account_info(),
    ];

    let instruction = Instruction {
        program_id: swap_accounts.dex_program_id.key(),
        accounts,
        data,
    };
    let dex_processor = &LaunchpadProcessor;
    let amount_out = invoke_process(
        dex_processor,
        &account_infos,
        swap_accounts.swap_source_token.key(),
        &mut swap_accounts.swap_destination_token,
        hop_accounts,
        instruction,
        hop,
        offset,
        LAUNCHPAD_ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )?;
    Ok(amount_out)
}
