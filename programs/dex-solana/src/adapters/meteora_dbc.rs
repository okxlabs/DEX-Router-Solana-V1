use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{HopAccounts, SWAP2_SELECTOR, ZERO_ADDRESS, meteora_dbc_program};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use arrayref::array_ref;

use super::common::DexProcessor;

pub struct MeteoraDynamicBondingCurveProcessor;
impl DexProcessor for MeteoraDynamicBondingCurveProcessor {}

pub struct MeteoraDynamicBondingCurve<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub pool_authority: &'info AccountInfo<'info>,
    pub config: &'info AccountInfo<'info>,
    pub pool: &'info AccountInfo<'info>,
    pub base_vault: InterfaceAccount<'info, TokenAccount>,
    pub quote_vault: InterfaceAccount<'info, TokenAccount>,
    pub base_mint: InterfaceAccount<'info, Mint>,
    pub quote_mint: InterfaceAccount<'info, Mint>,
    pub token_base_program: Interface<'info, TokenInterface>,
    pub token_quote_program: Interface<'info, TokenInterface>,
    pub referral_token_account: Option<&'info AccountInfo<'info>>,
    pub event_authority: &'info AccountInfo<'info>,
}
const ACCOUNTS_LEN: usize = 15;

impl<'info> MeteoraDynamicBondingCurve<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            pool_authority,
            config,
            pool,
            base_vault,
            quote_vault,
            base_mint,
            quote_mint,
            token_base_program,
            token_quote_program,
            referral_token_account,
            event_authority,
        ]: &[AccountInfo<'info>; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];
        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            pool_authority,
            config,
            pool,
            base_vault: InterfaceAccount::try_from(base_vault)?,
            quote_vault: InterfaceAccount::try_from(quote_vault)?,
            base_mint: InterfaceAccount::try_from(base_mint)?,
            quote_mint: InterfaceAccount::try_from(quote_mint)?,
            token_base_program: Interface::try_from(token_base_program)?,
            token_quote_program: Interface::try_from(token_quote_program)?,
            referral_token_account: if referral_token_account.key.eq(&ZERO_ADDRESS) {
                None
            } else {
                Some(referral_token_account)
            },
            event_authority,
        })
    }
}

pub struct MeteoraDynamicBondingCurve2<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub pool_authority: &'info AccountInfo<'info>,
    pub config: &'info AccountInfo<'info>,
    pub pool: &'info AccountInfo<'info>,
    pub base_vault: InterfaceAccount<'info, TokenAccount>,
    pub quote_vault: InterfaceAccount<'info, TokenAccount>,
    pub base_mint: InterfaceAccount<'info, Mint>,
    pub quote_mint: InterfaceAccount<'info, Mint>,
    pub token_base_program: Interface<'info, TokenInterface>,
    pub token_quote_program: Interface<'info, TokenInterface>,
    pub referral_token_account: Option<&'info AccountInfo<'info>>,
    pub event_authority: &'info AccountInfo<'info>,
    pub sysvar_instructions: &'info AccountInfo<'info>,
}
const ACCOUNTS2_LEN: usize = 16;

impl<'info> MeteoraDynamicBondingCurve2<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            pool_authority,
            config,
            pool,
            base_vault,
            quote_vault,
            base_mint,
            quote_mint,
            token_base_program,
            token_quote_program,
            referral_token_account,
            event_authority,
            sysvar_instructions,
        ]: &[AccountInfo<'info>; ACCOUNTS2_LEN] = array_ref![accounts, offset, ACCOUNTS2_LEN];
        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            pool_authority,
            config,
            pool,
            base_vault: InterfaceAccount::try_from(base_vault)?,
            quote_vault: InterfaceAccount::try_from(quote_vault)?,
            base_mint: InterfaceAccount::try_from(base_mint)?,
            quote_mint: InterfaceAccount::try_from(quote_mint)?,
            token_base_program: Interface::try_from(token_base_program)?,
            token_quote_program: Interface::try_from(token_quote_program)?,
            referral_token_account: if referral_token_account.key.eq(&ZERO_ADDRESS) {
                None
            } else {
                Some(referral_token_account)
            },
            event_authority,
            sysvar_instructions,
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
    msg!("Dex::MeteoraDbc amount_in: {}, offset: {}", amount_in, offset);
    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);

    let mut swap_accounts =
        MeteoraDynamicBondingCurve::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &meteora_dbc_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.pool.key().log();

    before_check(
        swap_accounts.swap_authority_pubkey,
        &swap_accounts.swap_source_token,
        swap_accounts.swap_destination_token.key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let mut data = Vec::with_capacity(25);
    data.extend_from_slice(SWAP2_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes()); // amount0(amount_in)
    data.extend_from_slice(&1u64.to_le_bytes()); // amount1(minimum_amount_out)
    data.push(0u8); // swap_mode(exact_in)

    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.pool_authority.key(), false),
        AccountMeta::new_readonly(swap_accounts.config.key(), false),
        AccountMeta::new(swap_accounts.pool.key(), false),
        AccountMeta::new(swap_accounts.swap_source_token.key(), false),
        AccountMeta::new(swap_accounts.swap_destination_token.key(), false),
        AccountMeta::new(swap_accounts.base_vault.key(), false),
        AccountMeta::new(swap_accounts.quote_vault.key(), false),
        AccountMeta::new_readonly(swap_accounts.base_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.quote_mint.key(), false),
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.token_base_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_quote_program.key(), false),
        {
            if let Some(referral_token_account) = swap_accounts.referral_token_account {
                AccountMeta::new(referral_token_account.key(), false)
            } else {
                AccountMeta::new_readonly(swap_accounts.dex_program_id.key(), false)
            }
        },
        AccountMeta::new_readonly(swap_accounts.event_authority.key(), false),
        AccountMeta::new_readonly(swap_accounts.dex_program_id.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.pool_authority.to_account_info(),
        swap_accounts.config.to_account_info(),
        swap_accounts.pool.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.base_vault.to_account_info(),
        swap_accounts.quote_vault.to_account_info(),
        swap_accounts.base_mint.to_account_info(),
        swap_accounts.quote_mint.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.token_base_program.to_account_info(),
        swap_accounts.token_quote_program.to_account_info(),
        {
            if let Some(referral_token_account) = swap_accounts.referral_token_account {
                referral_token_account.to_account_info()
            } else {
                swap_accounts.dex_program_id.to_account_info()
            }
        },
        swap_accounts.event_authority.to_account_info(),
        swap_accounts.dex_program_id.to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let amount_out = invoke_process(
        amount_in,
        &MeteoraDynamicBondingCurveProcessor,
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

pub fn swap2<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::MeteoraDbc2 amount_in: {}, offset: {}", amount_in, offset);
    require!(remaining_accounts.len() >= *offset + ACCOUNTS2_LEN, ErrorCode::InvalidAccountsLength);

    let mut swap_accounts =
        MeteoraDynamicBondingCurve2::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &meteora_dbc_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.pool.key().log();

    before_check(
        swap_accounts.swap_authority_pubkey,
        &swap_accounts.swap_source_token,
        swap_accounts.swap_destination_token.key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let mut data = Vec::with_capacity(25);
    data.extend_from_slice(SWAP2_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes()); // amount0(amount_in)
    data.extend_from_slice(&1u64.to_le_bytes()); // amount1(minimum_amount_out)
    data.push(0u8); // swap_mode(exact_in)

    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.pool_authority.key(), false),
        AccountMeta::new_readonly(swap_accounts.config.key(), false),
        AccountMeta::new(swap_accounts.pool.key(), false),
        AccountMeta::new(swap_accounts.swap_source_token.key(), false),
        AccountMeta::new(swap_accounts.swap_destination_token.key(), false),
        AccountMeta::new(swap_accounts.base_vault.key(), false),
        AccountMeta::new(swap_accounts.quote_vault.key(), false),
        AccountMeta::new_readonly(swap_accounts.base_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.quote_mint.key(), false),
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.token_base_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_quote_program.key(), false),
        {
            if let Some(referral_token_account) = swap_accounts.referral_token_account {
                AccountMeta::new(referral_token_account.key(), false)
            } else {
                AccountMeta::new_readonly(swap_accounts.dex_program_id.key(), false)
            }
        },
        AccountMeta::new_readonly(swap_accounts.event_authority.key(), false),
        AccountMeta::new_readonly(swap_accounts.dex_program_id.key(), false),
        AccountMeta::new_readonly(swap_accounts.sysvar_instructions.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.pool_authority.to_account_info(),
        swap_accounts.config.to_account_info(),
        swap_accounts.pool.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.base_vault.to_account_info(),
        swap_accounts.quote_vault.to_account_info(),
        swap_accounts.base_mint.to_account_info(),
        swap_accounts.quote_mint.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.token_base_program.to_account_info(),
        swap_accounts.token_quote_program.to_account_info(),
        {
            if let Some(referral_token_account) = swap_accounts.referral_token_account {
                referral_token_account.to_account_info()
            } else {
                swap_accounts.dex_program_id.to_account_info()
            }
        },
        swap_accounts.event_authority.to_account_info(),
        swap_accounts.dex_program_id.to_account_info(),
        swap_accounts.sysvar_instructions.to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let amount_out = invoke_process(
        amount_in,
        &MeteoraDynamicBondingCurveProcessor,
        &account_infos,
        &mut swap_accounts.swap_source_token,
        &mut swap_accounts.swap_destination_token,
        hop_accounts,
        instruction,
        hop,
        offset,
        ACCOUNTS2_LEN,
        proxy_swap,
        owner_seeds,
    )?;
    Ok(amount_out)
}
