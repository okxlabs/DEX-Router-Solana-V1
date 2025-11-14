use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{HopAccounts, alphaq_program, constants::ALPHAQ_SWAP_SELECTOR};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token::ID;
use anchor_spl::token_2022::spl_token_2022;
use anchor_spl::token_interface::TokenAccount;
use arrayref::array_ref;

use super::common::DexProcessor;

const ARGS_LEN: usize = 18;

pub struct AlphaQProcessor;
impl DexProcessor for AlphaQProcessor {}

pub struct AlphaQAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority: &'info AccountInfo<'info>,
    pub swap_source_account: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_account: InterfaceAccount<'info, TokenAccount>,

    pub market: &'info AccountInfo<'info>,
    pub market_stats: &'info AccountInfo<'info>,
    pub token_a_vault: &'info AccountInfo<'info>,
    pub token_b_vault: &'info AccountInfo<'info>,
    pub token_a_authority: &'info AccountInfo<'info>,
    pub token_b_authority: &'info AccountInfo<'info>,
    pub vendor_key: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub sysvar_instructions: &'info AccountInfo<'info>,
    pub remaining_account1: &'info AccountInfo<'info>,
    pub remaining_account2: &'info AccountInfo<'info>,
}

const ACCOUNTS_LEN: usize = 15;

impl<'info> AlphaQAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority,
            swap_source_account,
            swap_destination_account,
            market,
            market_stats,
            token_a_vault,
            token_b_vault,
            token_a_authority,
            token_b_authority,
            vendor_key,
            token_program,
            sysvar_instructions,
            remaining_account1,
            remaining_account2,
        ]: &[AccountInfo; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority,
            swap_source_account: InterfaceAccount::try_from(swap_source_account)?,
            swap_destination_account: InterfaceAccount::try_from(swap_destination_account)?,
            market,
            market_stats,
            token_a_vault,
            token_b_vault,
            token_a_authority,
            token_b_authority,
            vendor_key,
            token_program,
            sysvar_instructions,
            remaining_account1,
            remaining_account2,
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
    msg!("Dex::AlphaQ amount_in: {}, offset: {}", amount_in, offset);
    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);
    let mut swap_accounts = AlphaQAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &alphaq_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.market.key().log();

    // check hop accounts & swap authority
    before_check(
        &swap_accounts.swap_authority,
        &swap_accounts.swap_source_account,
        swap_accounts.swap_destination_account.key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let token_a_mint =
        *bytemuck::from_bytes::<Pubkey>(&swap_accounts.market.data.borrow()[240..272]);
    let token_b_mint =
        *bytemuck::from_bytes::<Pubkey>(&swap_accounts.market.data.borrow()[272..304]);

    let token_a_2022 = swap_accounts.market.try_borrow_data()?[65] == 1u8;
    let token_b_2022 = swap_accounts.market.try_borrow_data()?[66] == 1u8;

    let (user_token_account_a, user_token_account_b, a_to_b) =
        if swap_accounts.swap_source_account.mint == token_a_mint {
            (&swap_accounts.swap_source_account, &swap_accounts.swap_destination_account, true)
        } else if swap_accounts.swap_source_account.mint == token_b_mint {
            (&swap_accounts.swap_destination_account, &swap_accounts.swap_source_account, false)
        } else {
            return Err(ErrorCode::InvalidTokenMint.into());
        };

    let token_program = if token_a_2022 && token_b_2022 { spl_token_2022::id() } else { ID };
    require!(token_program == swap_accounts.token_program.key(), ErrorCode::InvalidProgramId);

    let mut accounts = Vec::with_capacity(ACCOUNTS_LEN);
    accounts.push(AccountMeta::new(swap_accounts.swap_authority.key(), true));
    accounts.push(AccountMeta::new_readonly(swap_accounts.market.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.market_stats.key(), false));
    accounts.push(AccountMeta::new(user_token_account_a.key(), false));
    accounts.push(AccountMeta::new(user_token_account_b.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.token_a_vault.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.token_b_vault.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.token_a_authority.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.token_b_authority.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.vendor_key.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.token_program.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.sysvar_instructions.key(), false));

    let mut account_infos = Vec::with_capacity(ACCOUNTS_LEN);
    account_infos.push(swap_accounts.swap_authority.to_account_info());
    account_infos.push(swap_accounts.market.to_account_info());
    account_infos.push(swap_accounts.market_stats.to_account_info());
    account_infos.push(user_token_account_a.to_account_info());
    account_infos.push(user_token_account_b.to_account_info());
    account_infos.push(swap_accounts.token_a_vault.to_account_info());
    account_infos.push(swap_accounts.token_b_vault.to_account_info());
    account_infos.push(swap_accounts.token_a_authority.to_account_info());
    account_infos.push(swap_accounts.token_b_authority.to_account_info());
    account_infos.push(swap_accounts.vendor_key.to_account_info());
    account_infos.push(swap_accounts.token_program.to_account_info());
    account_infos.push(swap_accounts.sysvar_instructions.to_account_info());

    if token_a_2022 {
        accounts.push(AccountMeta::new_readonly(token_a_mint, false));
        require!(
            token_a_mint == swap_accounts.remaining_account1.key(),
            ErrorCode::InvalidTokenMint
        );
        account_infos.push(swap_accounts.remaining_account1.to_account_info());
    }

    if token_b_2022 {
        accounts.push(AccountMeta::new_readonly(token_b_mint, false));
        require!(
            token_b_mint == swap_accounts.remaining_account2.key(),
            ErrorCode::InvalidTokenMint
        );
        account_infos.push(swap_accounts.remaining_account2.to_account_info());
    }

    if token_a_2022 ^ token_b_2022 {
        accounts.push(AccountMeta::new_readonly(spl_token_2022::id(), false));
        account_infos.push(swap_accounts.token_program.to_account_info());
    }

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(ALPHAQ_SWAP_SELECTOR); // discriminator
    data.extend_from_slice(&(a_to_b as u8).to_le_bytes()); // a_to_b
    data.extend_from_slice(&amount_in.to_le_bytes()); // amount
    data.extend_from_slice(&0u64.to_le_bytes()); // min_out_amount

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &AlphaQProcessor;
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
