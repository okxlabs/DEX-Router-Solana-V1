use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{HopAccounts, phoenix_program};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token::Token;
use anchor_spl::token_interface::TokenAccount;
use arrayref::array_ref;
use std::u64;

use super::common::DexProcessor;

const ARGS_LEN: usize = 55;

pub struct PhoenixProcessor;
impl DexProcessor for PhoenixProcessor {}

pub struct SwapAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub log_authority: &'info AccountInfo<'info>,
    pub market: &'info AccountInfo<'info>,
    pub base_vault: InterfaceAccount<'info, TokenAccount>,
    pub quote_vault: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}
const ACCOUNTS_LEN: usize = 9;

impl<'info> SwapAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            log_authority,
            market,
            base_vault,
            quote_vault,
            token_program,
        ]: &[AccountInfo<'info>; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            log_authority,
            market,
            base_vault: InterfaceAccount::try_from(base_vault)?,
            quote_vault: InterfaceAccount::try_from(quote_vault)?,
            token_program: Program::try_from(token_program)?,
        })
    }

    fn get_lot_size(&self) -> Result<(u64, u64)> {
        let data = &self.market.try_borrow_data()?;
        let base_lots_size = u64::from_le_bytes(*array_ref![data, 112, 8]);
        let quote_lots_size = u64::from_le_bytes(*array_ref![data, 192, 8]);
        Ok((base_lots_size, quote_lots_size))
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
    msg!("Dex::Phoenix amount_in: {}, offset: {}", amount_in, offset);
    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);

    let mut swap_accounts = SwapAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &phoenix_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.market.key().log();

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
    let (base_lot_size, quote_lot_size) = swap_accounts.get_lot_size()?;
    let (side, num_base_lots, num_quote_lots) =
        if swap_accounts.swap_source_token.mint == swap_accounts.base_vault.mint {
            (1u8, amount_in.checked_div(base_lot_size).ok_or(ErrorCode::CalculationError)?, 0u64)
        // 'ask' side
        } else {
            (0u8, 0u64, amount_in.checked_div(quote_lot_size).ok_or(ErrorCode::CalculationError)?)
            // 'bid' side
        };

    let order_type = 2u8; // 'immediateOrCancel'
    let self_trade_behavior = 1u8; // 'cancelProvide'

    data.push(0); //discriminator
    data.push(order_type);
    data.push(side);
    data.push(0); // Indicates absence of price_in_ticks (market order)
    data.extend_from_slice(&num_base_lots.to_le_bytes());
    data.extend_from_slice(&num_quote_lots.to_le_bytes());
    data.extend_from_slice(&0u64.to_le_bytes()); // min_base_lots_to_fill
    data.extend_from_slice(&0u64.to_le_bytes()); // min_quote_lots_to_fill
    data.push(self_trade_behavior);
    data.push(0); // Indicates absence of match_limit
    data.extend_from_slice(&0u128.to_le_bytes()); // client_order_id
    data.push(0u8); // use_only_deposited_funds as false

    let (user_base_account, user_quote_account) = if swap_accounts.swap_source_token.mint
        == swap_accounts.base_vault.mint
        && swap_accounts.swap_destination_token.mint == swap_accounts.quote_vault.mint
    {
        (swap_source_token, swap_destination_token)
    } else if swap_accounts.swap_source_token.mint == swap_accounts.quote_vault.mint
        && swap_accounts.swap_destination_token.mint == swap_accounts.base_vault.mint
    {
        (swap_destination_token, swap_source_token)
    } else {
        return Err(ErrorCode::InvalidTokenMint.into());
    };

    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.dex_program_id.key(), false),
        AccountMeta::new(swap_accounts.log_authority.key(), false),
        AccountMeta::new(swap_accounts.market.key(), false),
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new(user_base_account.key(), false),
        AccountMeta::new(user_quote_account.key(), false),
        AccountMeta::new(swap_accounts.base_vault.key(), false),
        AccountMeta::new(swap_accounts.quote_vault.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.dex_program_id.to_account_info(),
        swap_accounts.log_authority.to_account_info(),
        swap_accounts.market.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.base_vault.to_account_info(),
        swap_accounts.quote_vault.to_account_info(),
        swap_accounts.token_program.to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &PhoenixProcessor;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_pack_instruction() {
        let side = 1u8; // 'ask' side
        let num_base_lots = 68378u64;
        let num_quote_lots = 0u64;
        let order_type = 2u8; // 'immediateOrCancel'
        let self_trade_behavior = 1u8; // 'cancelProvide'

        let mut data = Vec::with_capacity(ARGS_LEN);
        data.push(0); // discriminator
        data.push(order_type);
        data.push(side);
        data.push(0); // Indicates absence of price_in_ticks (market order)
        data.extend_from_slice(&num_base_lots.to_le_bytes());
        data.extend_from_slice(&num_quote_lots.to_le_bytes());
        data.extend_from_slice(&0u64.to_le_bytes()); // min_base_lots_to_fill
        data.extend_from_slice(&0u64.to_le_bytes()); // min_quote_lots_to_fill
        data.push(self_trade_behavior);
        data.push(0); // Indicates absence of match_limit
        data.extend_from_slice(&0u128.to_le_bytes()); // client_order_id
        data.push(0u8); // use_only_deposited_funds as false

        msg!("data.len: {}", data.len());
        assert!(data.len() == ARGS_LEN);
        println!("constructed_data {:?}", data);
    }
}
