use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token_interface::TokenAccount;
use arrayref::array_ref;

use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{HopAccounts, SWAP_SELECTOR, whalestreet_program};

use super::common::DexProcessor;

const ARGS_LEN: usize = 25;

pub struct SwaapProcessor;
impl DexProcessor for SwaapProcessor {}

pub struct SwaapAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority: &'info AccountInfo<'info>,
    pub swap_source_account: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_account: InterfaceAccount<'info, TokenAccount>,

    pub safeguard_pool: &'info AccountInfo<'info>,
    pub base_vault: &'info AccountInfo<'info>,
    pub quote_vault: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
}
const ACCOUNTS_LEN: usize = 8;

impl<'info> SwaapAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority,
            swap_source_account,
            swap_destination_account,
            safeguard_pool,
            base_vault,
            quote_vault,
            token_program,
        ]: &[AccountInfo<'info>; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority,
            swap_source_account: InterfaceAccount::try_from(swap_source_account)?,
            swap_destination_account: InterfaceAccount::try_from(swap_destination_account)?,
            safeguard_pool,
            base_vault,
            quote_vault,
            token_program,
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
    msg!("Dex::Whalestreet amount_in: {}, offset: {}", amount_in, offset);

    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);

    let mut swap_accounts = SwaapAccounts::parse_accounts(remaining_accounts, *offset)?;

    if swap_accounts.dex_program_id.key != &whalestreet_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }

    swap_accounts.safeguard_pool.key().log();

    before_check(
        &swap_accounts.swap_authority,
        &swap_accounts.swap_source_account,
        swap_accounts.swap_destination_account.key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let (order_type, owner_base_token, owner_quote_token) = {
        let borrow_data1 = &swap_accounts.safeguard_pool.try_borrow_data()?[22..54];
        let borrow_data2 = &swap_accounts.safeguard_pool.try_borrow_data()?[137..169];
        let base_mint: &Pubkey = bytemuck::from_bytes(borrow_data1);
        let quote_mint: &Pubkey = bytemuck::from_bytes(borrow_data2);

        if base_mint == &swap_accounts.swap_source_account.mint
            && quote_mint == &swap_accounts.swap_destination_account.mint
        {
            // BuyBase
            (1u8, &swap_accounts.swap_source_account, &swap_accounts.swap_destination_account)
        } else if base_mint == &swap_accounts.swap_destination_account.mint
            && quote_mint == &swap_accounts.swap_source_account.mint
        {
            // BuyQuote
            (3, &swap_accounts.swap_destination_account, &swap_accounts.swap_source_account)
        } else {
            return Err(ProgramError::InvalidArgument.into());
        }
    };

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(SWAP_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&1u64.to_le_bytes());
    data.extend_from_slice(&order_type.to_le_bytes());

    let mut accounts = Vec::with_capacity(ACCOUNTS_LEN - 1);
    accounts.push(AccountMeta::new(swap_accounts.safeguard_pool.key(), false));
    accounts.push(AccountMeta::new(owner_base_token.key(), false));
    accounts.push(AccountMeta::new(owner_quote_token.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.base_vault.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.quote_vault.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.swap_authority.key(), true));
    accounts.push(AccountMeta::new_readonly(swap_accounts.token_program.key(), false));

    let mut account_infos = Vec::with_capacity(ACCOUNTS_LEN - 1);
    account_infos.push(swap_accounts.safeguard_pool.to_account_info());
    account_infos.push(owner_base_token.to_account_info());
    account_infos.push(owner_quote_token.to_account_info());
    account_infos.push(swap_accounts.base_vault.to_account_info());
    account_infos.push(swap_accounts.quote_vault.to_account_info());
    account_infos.push(swap_accounts.swap_authority.to_account_info());
    account_infos.push(swap_accounts.token_program.to_account_info());

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &SwaapProcessor;
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
        ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )?;

    Ok(amount_out)
}
