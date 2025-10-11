use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token_interface::{TokenAccount, TokenInterface};
use arrayref::array_ref;
use borsh::{BorshDeserialize, BorshSerialize};

use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{GOONFI_SWAP_SELECTOR, HopAccounts, goonfi_program};

use super::common::DexProcessor;

const ARGS_LEN: usize = 19;

pub struct GoonfiProcessor;
impl DexProcessor for GoonfiProcessor {}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct SwapParams {
    pub is_user_bid: bool,
    pub bump: u8,
    pub amount_in: u64,
    pub minimum_amount_out: u64,
}

pub struct GoonfiAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority: &'info AccountInfo<'info>,
    pub swap_source_account: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_account: InterfaceAccount<'info, TokenAccount>,

    pub goonfi_param: &'info AccountInfo<'info>,

    pub market: &'info AccountInfo<'info>,
    pub base_vault: &'info AccountInfo<'info>,
    pub quote_vault: &'info AccountInfo<'info>,
    pub blacklist: &'info AccountInfo<'info>,
    pub sysvar_instructions: &'info AccountInfo<'info>,
    pub token_program: Interface<'info, TokenInterface>,
}
const ACCOUNTS_LEN: usize = 11;

impl<'info> GoonfiAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority,
            swap_source_account,
            swap_destination_account,
            goonfi_param,
            market,
            base_vault,
            quote_vault,
            blacklist,
            sysvar_instructions,
            token_program,
        ]: &[AccountInfo<'info>; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority,
            swap_source_account: InterfaceAccount::try_from(swap_source_account)?,
            swap_destination_account: InterfaceAccount::try_from(swap_destination_account)?,
            goonfi_param,
            market,
            base_vault,
            quote_vault,
            blacklist,
            sysvar_instructions,
            token_program: Interface::try_from(token_program)?,
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
    msg!("Dex::GoonFi amount_in: {}, offset: {}", amount_in, offset);

    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);

    let mut swap_accounts = GoonfiAccounts::parse_accounts(remaining_accounts, *offset)?;

    if swap_accounts.dex_program_id.key != &goonfi_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }

    swap_accounts.market.key().log();

    before_check(
        &swap_accounts.swap_authority,
        &swap_accounts.swap_source_account,
        swap_accounts.swap_destination_account.key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let quote_mint: Pubkey =
        swap_accounts.quote_vault.try_borrow_data()?[0..32].try_into().unwrap();

    let is_bid = swap_accounts.swap_source_account.mint == quote_mint;

    let (base_account, quote_account) = if is_bid {
        (&swap_accounts.swap_destination_account, &swap_accounts.swap_source_account)
    } else {
        (&swap_accounts.swap_source_account, &swap_accounts.swap_destination_account)
    };

    // Extract blacklist_bump from goonfi_param account
    let goonfi_param_data = swap_accounts.goonfi_param.key().as_array().clone();
    let blacklist_bump = u8::from_le_bytes(
        goonfi_param_data[0..1].try_into().map_err(|_| ErrorCode::InvalidGoonfiParameters)?,
    );
    let unused = &goonfi_param_data[1..32];
    require!(unused == &[0u8; 31], ErrorCode::InvalidGoonfiParameters);

    let swap_params: SwapParams =
        SwapParams { is_user_bid: is_bid, bump: blacklist_bump, amount_in, minimum_amount_out: 1 };

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(GOONFI_SWAP_SELECTOR);
    data.extend_from_slice(&swap_params.try_to_vec()?);

    let mut accounts = Vec::with_capacity(ACCOUNTS_LEN - 2);
    accounts.push(AccountMeta::new(swap_accounts.swap_authority.key(), true));
    accounts.push(AccountMeta::new(swap_accounts.market.key(), false));
    accounts.push(AccountMeta::new(base_account.key(), false));
    accounts.push(AccountMeta::new(quote_account.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.base_vault.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.quote_vault.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.blacklist.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.sysvar_instructions.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.token_program.key(), false));

    let mut account_infos = Vec::with_capacity(ACCOUNTS_LEN - 2);
    account_infos.push(swap_accounts.swap_authority.to_account_info());
    account_infos.push(swap_accounts.market.to_account_info());
    account_infos.push(base_account.to_account_info());
    account_infos.push(quote_account.to_account_info());
    account_infos.push(swap_accounts.base_vault.to_account_info());
    account_infos.push(swap_accounts.quote_vault.to_account_info());
    account_infos.push(swap_accounts.blacklist.to_account_info());
    account_infos.push(swap_accounts.sysvar_instructions.to_account_info());
    account_infos.push(swap_accounts.token_program.to_account_info());

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &GoonfiProcessor;
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
