use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{
    HopAccounts, VIRTUALS_BUY_SELECTOR, VIRTUALS_SELL_SELECTOR, virtual_token_mint,
    virtuals_program,
};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token::Token;
use anchor_spl::token_interface::{Mint, TokenAccount};
use arrayref::array_ref;

use super::common::DexProcessor;

const ARGS_LEN: usize = 24;

pub struct VirtualsProcessor;
impl DexProcessor for VirtualsProcessor {}

//this dex only supoort spl token not support token_2022
pub struct VirtualsAccount<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub vpool: &'info AccountInfo<'info>,
    pub token_mint: InterfaceAccount<'info, Mint>,
    pub vpool_token_ata: InterfaceAccount<'info, TokenAccount>,
    pub platform_prototype: &'info AccountInfo<'info>,
    pub platform_prototype_virtuals_ata: InterfaceAccount<'info, TokenAccount>,
    pub vpool_virtuals_ata: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}
const ACCOUNTS_LEN: usize = 11;

impl<'info> VirtualsAccount<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            vpool,
            token_mint,
            vpool_token_ata,
            platform_prototype,
            platform_prototype_virtuals_ata,
            vpool_virtuals_ata,
            token_program,
        ]: &[AccountInfo<'info>; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];
        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            vpool,
            token_mint: InterfaceAccount::try_from(token_mint)?,
            vpool_token_ata: InterfaceAccount::try_from(vpool_token_ata)?,
            platform_prototype,
            platform_prototype_virtuals_ata: InterfaceAccount::try_from(
                platform_prototype_virtuals_ata,
            )?,
            vpool_virtuals_ata: InterfaceAccount::try_from(vpool_virtuals_ata)?,
            token_program: Program::try_from(token_program)?,
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
    msg!("Dex::virtuals amount_in: {}, offset: {}", amount_in, offset);
    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);
    let mut swap_accounts = VirtualsAccount::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &virtuals_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.vpool.key().log();

    // check hop accounts & swap authority
    let swap_source_token = swap_accounts.swap_source_token.clone();
    let swap_destination_token = swap_accounts.swap_destination_token.key();
    before_check(
        &swap_accounts.swap_authority_pubkey,
        &swap_source_token,
        swap_destination_token,
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let mut data = Vec::with_capacity(ARGS_LEN);
    let (user_token_account_info, user_virtuals_account_info);
    let vpool_virtual_y = parse_vpool_virtual_y(swap_accounts.vpool)?;
    //msg!("vpool_account: {}", vpool_virtual_y);

    if swap_accounts.swap_source_token.mint == virtual_token_mint::id() {
        // Calculate the number of tokens to be purchased (reverse calculation)
        let buy_amount = calculate_buy_amount(
            swap_accounts.vpool_token_ata.amount,
            swap_accounts.vpool_virtuals_ata.amount + vpool_virtual_y,
            amount_in,
        )?;

        data.extend_from_slice(VIRTUALS_BUY_SELECTOR);
        data.extend_from_slice(&buy_amount.to_le_bytes()); // buy meme amount
        data.extend_from_slice(&amount_in.to_le_bytes()); // max_virtual_amount_out

        user_virtuals_account_info = swap_accounts.swap_source_token.to_account_info();
        user_token_account_info = swap_accounts.swap_destination_token.to_account_info();
    } else if swap_accounts.swap_destination_token.mint == virtual_token_mint::id() {
        data.extend_from_slice(VIRTUALS_SELL_SELECTOR);
        data.extend_from_slice(&amount_in.to_le_bytes()); // sell meme amount
        data.extend_from_slice(&0u64.to_le_bytes()); // min_virtual_amount_return

        user_token_account_info = swap_accounts.swap_source_token.to_account_info();
        user_virtuals_account_info = swap_accounts.swap_destination_token.to_account_info();
    } else {
        return Err(ErrorCode::InvalidTokenMint.into());
    }

    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new(swap_accounts.vpool.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_mint.key(), false),
        AccountMeta::new(user_virtuals_account_info.key(), false),
        AccountMeta::new(user_token_account_info.key(), false),
        AccountMeta::new(swap_accounts.vpool_token_ata.key(), false),
        AccountMeta::new(swap_accounts.platform_prototype.key(), false),
        AccountMeta::new(swap_accounts.platform_prototype_virtuals_ata.key(), false),
        AccountMeta::new(swap_accounts.vpool_virtuals_ata.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.vpool.to_account_info(),
        swap_accounts.token_mint.to_account_info(),
        user_virtuals_account_info,
        user_token_account_info,
        swap_accounts.vpool_token_ata.to_account_info(),
        swap_accounts.platform_prototype.to_account_info(),
        swap_accounts.platform_prototype_virtuals_ata.to_account_info(),
        swap_accounts.vpool_virtuals_ata.to_account_info(),
        swap_accounts.token_program.to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &VirtualsProcessor;
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

fn calculate_buy_amount(
    token_balance: u64,
    virtuals_balance: u64,
    virtuals_amount: u64,
) -> Result<u64> {
    // 1%（100/10000）
    const FEE_BP: u16 = 100;
    let fee_factor = 10000u128 + FEE_BP as u128;

    // 1. Calculate the actual number of virtual tokens available for exchange (after deducting fees)
    let virtuals_amount_for_swap = (virtuals_amount as u128 * 10000u128)
        .checked_div(fee_factor)
        .ok_or(ErrorCode::CalculationError)?;

    // 2. Apply the inverse operation of xy=k formula
    let numerator = (virtuals_amount_for_swap)
        .checked_mul(token_balance as u128)
        .ok_or(ErrorCode::CalculationError)?;

    let denominator = virtuals_amount_for_swap
        .checked_add(virtuals_balance as u128)
        .ok_or(ErrorCode::CalculationError)?;

    let buy_amount = numerator
        .checked_div(denominator)
        .ok_or(ErrorCode::CalculationError)?
        .try_into()
        .map_err(|_| ErrorCode::CalculationError)?;

    require!(buy_amount > 0, ErrorCode::ResultMustBeGreaterThanZero);

    Ok(buy_amount)
}

fn parse_vpool_virtual_y(account_info: &AccountInfo) -> Result<u64> {
    // Ensure that account data is at least 80 bytes long
    if account_info.data_len() < 80 {
        return Err(ErrorCode::InvalidAccountData.into());
    }

    // Obtain account data
    let data = account_info.try_borrow_data()?;

    // The virtuali_y field is located at the 72-80 byte position (after the 8-byte discriminator and 2 32 byte pubkeys)
    let virtual_y_bytes = array_ref![data, 72, 8];

    // Convert bytes to u64 (small endian format)
    let virtual_y = u64::from_le_bytes(*virtual_y_bytes);

    Ok(virtual_y)
}
