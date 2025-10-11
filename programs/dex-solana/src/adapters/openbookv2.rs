use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{HopAccounts, PLACE_TAKE_ORDER_SELECTOR, ZERO_ADDRESS, openbookv2_program};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token::Token;
use anchor_spl::token_interface::TokenAccount;
use arrayref::array_ref;
use num_enum::{IntoPrimitive, TryFromPrimitive};

use super::common::DexProcessor;

const ARGS_LEN: usize = 35;

#[derive(
    AnchorDeserialize, AnchorSerialize, Clone, Debug, PartialEq, TryFromPrimitive, IntoPrimitive,
)]
#[repr(u8)]
pub enum Side {
    Bid = 0,
    Ask = 1,
}

pub struct OpenbookV2Processor;
impl DexProcessor for OpenbookV2Processor {}

pub struct PlaceTakeOrderAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub market: &'info AccountInfo<'info>,
    pub market_authority: &'info AccountInfo<'info>,
    pub bids: &'info AccountInfo<'info>,
    pub asks: &'info AccountInfo<'info>,
    pub market_base_vault: InterfaceAccount<'info, TokenAccount>,
    pub market_quote_vault: InterfaceAccount<'info, TokenAccount>,
    pub event_heap: &'info AccountInfo<'info>,
    pub oracle_a: &'info AccountInfo<'info>,
    pub oracle_b: &'info AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub open_orders_admin: &'info AccountInfo<'info>,
    pub open_orders_account0: &'info AccountInfo<'info>,
    pub open_orders_account1: &'info AccountInfo<'info>,
    pub open_orders_account2: &'info AccountInfo<'info>,
}
const ACCOUNTS_LEN: usize = 19;

impl<'info> PlaceTakeOrderAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            market,
            market_authority,
            bids,
            asks,
            market_base_vault,
            market_quote_vault,
            event_heap,
            oracle_a,
            oracle_b,
            token_program,
            system_program,
            open_orders_admin,
            open_orders_account0,
            open_orders_account1,
            open_orders_account2,
        ]: &[AccountInfo<'info>; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            market,
            market_authority,
            bids,
            asks,
            market_base_vault: InterfaceAccount::try_from(market_base_vault)?,
            market_quote_vault: InterfaceAccount::try_from(market_quote_vault)?,
            event_heap,
            oracle_a,
            oracle_b,
            token_program: Program::try_from(token_program)?,
            system_program: Program::try_from(system_program)?,
            open_orders_admin,
            open_orders_account0,
            open_orders_account1,
            open_orders_account2,
        })
    }

    fn get_lot_size(&self) -> Result<(i64, i64)> {
        let data = &self.market.try_borrow_data()?;
        let quote_lots_size = i64::from_le_bytes(*array_ref![data, 448, 8]);
        let base_lots_size = i64::from_le_bytes(*array_ref![data, 456, 8]);
        Ok((base_lots_size, quote_lots_size))
    }
}

pub fn place_take_order<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::OpenBookV2 amount_in: {}, offset: {}", amount_in, offset);
    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);

    let mut swap_accounts = PlaceTakeOrderAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &openbookv2_program::id() {
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

    let (base_lot_size, quote_lot_size) = swap_accounts.get_lot_size()?;
    let side;
    let price_lots: i64;
    let max_base_lots;
    let max_quote_lots_including_fees;
    if swap_accounts.swap_source_token.mint == swap_accounts.market_base_vault.mint {
        side = Side::Ask;
        price_lots = 1;
        max_base_lots = i64::try_from(amount_in)
            .unwrap()
            .checked_div(base_lot_size)
            .ok_or(ErrorCode::CalculationError)?;
        max_quote_lots_including_fees =
            i64::MAX.checked_div(quote_lot_size).ok_or(ErrorCode::CalculationError)?;
    } else {
        side = Side::Bid;
        price_lots = i64::MAX;
        max_base_lots = i64::MAX.checked_div(base_lot_size).ok_or(ErrorCode::CalculationError)?;
        max_quote_lots_including_fees = i64::try_from(amount_in)
            .unwrap()
            .checked_div(quote_lot_size)
            .ok_or(ErrorCode::CalculationError)?;
    }
    let order_type = 3u8;
    let limit = 50u8;

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(PLACE_TAKE_ORDER_SELECTOR);
    data.push(Side::into(side));
    data.extend_from_slice(&price_lots.to_le_bytes());
    data.extend_from_slice(&max_base_lots.to_le_bytes());
    data.extend_from_slice(&max_quote_lots_including_fees.to_le_bytes());
    data.extend_from_slice(&order_type.to_le_bytes());
    data.extend_from_slice(&limit.to_le_bytes());

    let (user_base_account, user_quote_account) = if swap_accounts.swap_source_token.mint
        == swap_accounts.market_base_vault.mint
        && swap_accounts.swap_destination_token.mint == swap_accounts.market_quote_vault.mint
    {
        (swap_source_token, swap_destination_token)
    } else if swap_accounts.swap_source_token.mint == swap_accounts.market_quote_vault.mint
        && swap_accounts.swap_destination_token.mint == swap_accounts.market_base_vault.mint
    {
        (swap_destination_token, swap_source_token)
    } else {
        return Err(ErrorCode::InvalidTokenMint.into());
    };

    let mut accounts = vec![
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new(swap_accounts.market.key(), false),
        AccountMeta::new_readonly(swap_accounts.market_authority.key(), false),
        AccountMeta::new(swap_accounts.bids.key(), false),
        AccountMeta::new(swap_accounts.asks.key(), false),
        AccountMeta::new(swap_accounts.market_base_vault.key(), false),
        AccountMeta::new(swap_accounts.market_quote_vault.key(), false),
        AccountMeta::new(swap_accounts.event_heap.key(), false),
        AccountMeta::new(user_base_account.key(), false),
        AccountMeta::new(user_quote_account.key(), false),
        AccountMeta::new_readonly(swap_accounts.oracle_a.key(), false),
        AccountMeta::new_readonly(swap_accounts.oracle_b.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.system_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.open_orders_admin.key(), false),
    ];

    let mut account_infos = vec![
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.market.to_account_info(),
        swap_accounts.market_authority.to_account_info(),
        swap_accounts.bids.to_account_info(),
        swap_accounts.asks.to_account_info(),
        swap_accounts.market_base_vault.to_account_info(),
        swap_accounts.market_quote_vault.to_account_info(),
        swap_accounts.event_heap.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.oracle_a.to_account_info(),
        swap_accounts.oracle_b.to_account_info(),
        swap_accounts.token_program.to_account_info(),
        swap_accounts.system_program.to_account_info(),
        swap_accounts.open_orders_admin.to_account_info(),
    ];

    let open_orders_account0 = swap_accounts.open_orders_account0.key();
    let open_orders_account1 = swap_accounts.open_orders_account1.key();
    let open_orders_account2 = swap_accounts.open_orders_account2.key();
    if open_orders_account0 != ZERO_ADDRESS {
        accounts.push(AccountMeta::new(open_orders_account0, false));
        account_infos.push(swap_accounts.open_orders_account0.to_account_info());
    }
    if open_orders_account1 != ZERO_ADDRESS {
        accounts.push(AccountMeta::new(open_orders_account1, false));
        account_infos.push(swap_accounts.open_orders_account1.to_account_info());
    }
    if open_orders_account2 != ZERO_ADDRESS {
        accounts.push(AccountMeta::new(open_orders_account2, false));
        account_infos.push(swap_accounts.open_orders_account2.to_account_info());
    }

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &OpenbookV2Processor;
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
        let amount_in = 39624782u64;

        let side = Side::Bid;
        let price_lots = i64::MAX;
        // let quote_lot_size = 1i64;
        let base_lot_size = 100000i64;
        let max_base_lots = i64::MAX / base_lot_size;
        let max_quote_lots_including_fees = i64::try_from(amount_in).unwrap();
        let order_type = 3u8;
        let limit = 50u8;

        let mut data = Vec::with_capacity(ARGS_LEN);
        data.extend_from_slice(PLACE_TAKE_ORDER_SELECTOR);
        data.push(Side::into(side));
        data.extend_from_slice(&price_lots.to_le_bytes());
        data.extend_from_slice(&max_base_lots.to_le_bytes());
        data.extend_from_slice(&max_quote_lots_including_fees.to_le_bytes());
        data.extend_from_slice(&order_type.to_le_bytes());
        data.extend_from_slice(&limit.to_le_bytes());
        msg!("data.len: {}", data.len());
        assert!(data.len() == ARGS_LEN);
        println!("constructed_data {:?}", data);
        //032c47031ac7cb5500ffffffffffffff7fa38d23d6e25300004ea05c02000000000332
    }

    #[test]
    pub fn test_div() {
        let amount_in: u64 = 113934176;
        let base_lot_size = i64::try_from(1000000).unwrap();
        let res = i64::try_from(amount_in).unwrap() / base_lot_size;
        msg!("res: {}", res);
    }
}
