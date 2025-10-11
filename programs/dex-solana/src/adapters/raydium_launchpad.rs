use super::common::DexProcessor;
use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{
    BUY_EXACT_IN_SELECTOR, HopAccounts, SELL_EXACT_IN_SELECTOR, raydium_launchpad_program,
};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use arrayref::array_ref;

pub struct LaunchpadProcessor;
impl DexProcessor for LaunchpadProcessor {}

const LAUNCHPAD_ACCOUNTS_LEN: usize = 18;
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
    pub system_program: Program<'info, System>,
    pub platform_claim_fee_vault: &'info AccountInfo<'info>,
    pub creator_claim_fee_vault: &'info AccountInfo<'info>,
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
            system_program,
            platform_claim_fee_vault,
            creator_claim_fee_vault,
            event_authority,
        ]: &[AccountInfo<'info>; LAUNCHPAD_ACCOUNTS_LEN] =
            array_ref![accounts, offset, LAUNCHPAD_ACCOUNTS_LEN];

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
            system_program: Program::try_from(system_program)?,
            platform_claim_fee_vault,
            creator_claim_fee_vault,
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
    platform_name: &str,
) -> Result<u64> {
    require!(
        remaining_accounts.len() >= *offset + LAUNCHPAD_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );
    let mut swap_accounts = LaunchpadAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &raydium_launchpad_program::id() {
        swap_accounts.dex_program_id.key().log();
        return Err(ErrorCode::InvalidProgramId.into());
    }

    msg!("Dex::{} amount_in: {}, offset: {}", platform_name, amount_in, offset);
    swap_accounts.pool_state.key().log();

    let is_buy = swap_accounts.swap_source_token.mint.eq(&swap_accounts.quote_mint.key());

    before_check(
        &swap_accounts.swap_authority_pubkey,
        &swap_accounts.swap_source_token,
        swap_accounts.swap_destination_token.key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let (swap_base_token, swap_quote_token) = if is_buy {
        (&swap_accounts.swap_destination_token, &swap_accounts.swap_source_token)
    } else {
        (&swap_accounts.swap_source_token, &swap_accounts.swap_destination_token)
    };

    let mut data = Vec::with_capacity(32);
    if is_buy {
        data.extend_from_slice(BUY_EXACT_IN_SELECTOR);
    } else {
        data.extend_from_slice(SELL_EXACT_IN_SELECTOR);
    }
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&1u64.to_le_bytes()); //minimum_amount_out
    data.extend_from_slice(&0u64.to_le_bytes()); //share_fee_rate

    let mut accounts = Vec::with_capacity(LAUNCHPAD_ACCOUNTS_LEN);
    accounts.push(AccountMeta::new_readonly(swap_accounts.swap_authority_pubkey.key(), true));
    accounts.push(AccountMeta::new_readonly(swap_accounts.launchpad_authority.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.global_config.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.platform_config.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.pool_state.key(), false));
    accounts.push(AccountMeta::new(swap_base_token.key(), false));
    accounts.push(AccountMeta::new(swap_quote_token.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.base_vault.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.quote_vault.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.base_mint.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.quote_mint.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.base_token_program.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.quote_token_program.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.event_authority.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.dex_program_id.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.system_program.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.platform_claim_fee_vault.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.creator_claim_fee_vault.key(), false));

    let mut account_infos = Vec::with_capacity(LAUNCHPAD_ACCOUNTS_LEN);
    account_infos.push(swap_accounts.swap_authority_pubkey.to_account_info());
    account_infos.push(swap_accounts.launchpad_authority.to_account_info());
    account_infos.push(swap_accounts.global_config.to_account_info());
    account_infos.push(swap_accounts.platform_config.to_account_info());
    account_infos.push(swap_accounts.pool_state.to_account_info());
    account_infos.push(swap_base_token.to_account_info());
    account_infos.push(swap_quote_token.to_account_info());
    account_infos.push(swap_accounts.base_vault.to_account_info());
    account_infos.push(swap_accounts.quote_vault.to_account_info());
    account_infos.push(swap_accounts.base_mint.to_account_info());
    account_infos.push(swap_accounts.quote_mint.to_account_info());
    account_infos.push(swap_accounts.base_token_program.to_account_info());
    account_infos.push(swap_accounts.quote_token_program.to_account_info());
    account_infos.push(swap_accounts.event_authority.to_account_info());
    account_infos.push(swap_accounts.dex_program_id.to_account_info());
    account_infos.push(swap_accounts.system_program.to_account_info());
    account_infos.push(swap_accounts.platform_claim_fee_vault.to_account_info());
    account_infos.push(swap_accounts.creator_claim_fee_vault.to_account_info());

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &LaunchpadProcessor;
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
        LAUNCHPAD_ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )?;
    Ok(amount_out)
}
