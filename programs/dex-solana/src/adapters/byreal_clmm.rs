use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{HopAccounts, SWAP_V2_SELECTOR, byreal_clmm_program};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token_interface::TokenAccount;
use arrayref::array_ref;

use super::common::DexProcessor;

const ARGS_LEN: usize = 41;
pub struct ByrealClmmSwapV2Processor;
impl DexProcessor for ByrealClmmSwapV2Processor {}
pub struct ByrealClmmSwapV2Accounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub payer: &'info AccountInfo<'info>,
    pub amm_config: &'info AccountInfo<'info>,
    pub pool_state: &'info AccountInfo<'info>,
    pub input_token_account: &'info AccountInfo<'info>,
    pub output_token_account: &'info AccountInfo<'info>,
    pub input_vault: &'info AccountInfo<'info>,
    pub output_vault: &'info AccountInfo<'info>,
    pub observation_state: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub token_program_2022: &'info AccountInfo<'info>,
    pub memo_program: &'info AccountInfo<'info>,
    pub input_vault_mint: &'info AccountInfo<'info>,
    pub output_vault_mint: &'info AccountInfo<'info>,
    pub tickarray_bitmap_extension: &'info AccountInfo<'info>,
    pub tick_array0: &'info AccountInfo<'info>,
    pub tick_array1: &'info AccountInfo<'info>,
    pub tick_array2: &'info AccountInfo<'info>,
    pub tick_array3: &'info AccountInfo<'info>,
    pub tick_array4: &'info AccountInfo<'info>,
    pub tick_array5: &'info AccountInfo<'info>,
}

const ACCOUNTS_LEN: usize = 24;

impl<'info> ByrealClmmSwapV2Accounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            payer,
            amm_config,
            pool_state,
            input_token_account,
            output_token_account,
            input_vault,
            output_vault,
            observation_state,
            token_program,
            token_program_2022,
            memo_program,
            input_vault_mint,
            output_vault_mint,
            tickarray_bitmap_extension,
            tick_array0,
            tick_array1,
            tick_array2,
            tick_array3,
            tick_array4,
            tick_array5,
        ]: &[AccountInfo; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            payer,
            amm_config,
            pool_state,
            input_token_account,
            output_token_account,
            input_vault,
            output_vault,
            observation_state,
            token_program,
            token_program_2022,
            memo_program,
            input_vault_mint,
            output_vault_mint,
            tickarray_bitmap_extension,
            tick_array0,
            tick_array1,
            tick_array2,
            tick_array3,
            tick_array4,
            tick_array5,
        })
    }
}

pub fn swap_v2<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::ByrealClmm amount_in: {}, offset: {}", amount_in, offset);
    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);
    let mut swap_accounts = ByrealClmmSwapV2Accounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &byreal_clmm_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.pool_state.key().log();

    // check hop accounts & swap authority
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

    let mut accounts = vec![
        AccountMeta::new_readonly(swap_accounts.payer.key(), true),
        AccountMeta::new_readonly(swap_accounts.amm_config.key(), false),
        AccountMeta::new(swap_accounts.pool_state.key(), false),
        AccountMeta::new(swap_accounts.input_token_account.key(), false),
        AccountMeta::new(swap_accounts.output_token_account.key(), false),
        AccountMeta::new(swap_accounts.input_vault.key(), false),
        AccountMeta::new(swap_accounts.output_vault.key(), false),
        AccountMeta::new(swap_accounts.observation_state.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program_2022.key(), false),
        AccountMeta::new_readonly(swap_accounts.memo_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.input_vault_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.output_vault_mint.key(), false),
        AccountMeta::new(swap_accounts.tickarray_bitmap_extension.key(), false),
    ];

    let mut account_infos = vec![
        swap_accounts.payer.to_account_info(),
        swap_accounts.amm_config.to_account_info(),
        swap_accounts.pool_state.to_account_info(),
        swap_accounts.input_token_account.to_account_info(),
        swap_accounts.output_token_account.to_account_info(),
        swap_accounts.input_vault.to_account_info(),
        swap_accounts.output_vault.to_account_info(),
        swap_accounts.observation_state.to_account_info(),
        swap_accounts.token_program.to_account_info(),
        swap_accounts.token_program_2022.to_account_info(),
        swap_accounts.memo_program.to_account_info(),
        swap_accounts.input_vault_mint.to_account_info(),
        swap_accounts.output_vault_mint.to_account_info(),
        swap_accounts.tickarray_bitmap_extension.to_account_info(),
    ];

    if swap_accounts.tick_array0.key != &byreal_clmm_program::id() {
        accounts.push(AccountMeta::new(swap_accounts.tick_array0.key(), false));
        account_infos.push(swap_accounts.tick_array0.to_account_info());
    }

    if swap_accounts.tick_array1.key != &byreal_clmm_program::id() {
        accounts.push(AccountMeta::new(swap_accounts.tick_array1.key(), false));
        account_infos.push(swap_accounts.tick_array1.to_account_info());
    }

    if swap_accounts.tick_array2.key != &byreal_clmm_program::id() {
        accounts.push(AccountMeta::new(swap_accounts.tick_array2.key(), false));
        account_infos.push(swap_accounts.tick_array2.to_account_info());
    }

    if swap_accounts.tick_array3.key != &byreal_clmm_program::id() {
        accounts.push(AccountMeta::new(swap_accounts.tick_array3.key(), false));
        account_infos.push(swap_accounts.tick_array3.to_account_info());
    }

    if swap_accounts.tick_array4.key != &byreal_clmm_program::id() {
        accounts.push(AccountMeta::new(swap_accounts.tick_array4.key(), false));
        account_infos.push(swap_accounts.tick_array4.to_account_info());
    }

    if swap_accounts.tick_array5.key != &byreal_clmm_program::id() {
        accounts.push(AccountMeta::new(swap_accounts.tick_array5.key(), false));
        account_infos.push(swap_accounts.tick_array5.to_account_info());
    }

    let mut data = vec![0u8; ARGS_LEN];
    data[0..8].copy_from_slice(&SWAP_V2_SELECTOR[..]);
    data[8..16].copy_from_slice(&amount_in.to_le_bytes()); // amount
    data[16..24].copy_from_slice(&1u64.to_le_bytes()); // other_amount_threshold
    data[24..40].copy_from_slice(&0u128.to_le_bytes()); // sqrt_price_limit_x64
    data[40..41].copy_from_slice(&1u8.to_le_bytes()); // is_base_input

    let instruction = Instruction { program_id: *swap_accounts.dex_program_id.key, accounts, data };

    let dex_processor = ByrealClmmSwapV2Processor {};
    let amount_out = invoke_process(
        amount_in,
        &dex_processor,
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
