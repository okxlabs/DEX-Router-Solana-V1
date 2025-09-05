use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token_interface::{TokenAccount, TokenInterface};
use arrayref::array_ref;
use borsh::{BorshDeserialize, BorshSerialize};

use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{humidifi_program, HopAccounts, HUMIDIFI_IX_DATA_KEY, HUMIDIFI_SWAP_SELECTOR};

use super::common::DexProcessor;

const ARGS_LEN: usize = 25;

pub struct HumidifiProcessor;

impl DexProcessor for HumidifiProcessor {}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct SwapParams {
    pub swap_id: u64,
    pub amount_in: u64,
    pub is_base_to_quote: u8,
    pub padding: [u8; 7],
}

pub struct HumidifiAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub humidifi_param: &'info AccountInfo<'info>,

    pub pool: &'info AccountInfo<'info>,
    pub pool_base_token_account: InterfaceAccount<'info, TokenAccount>,
    pub pool_quote_token_account: InterfaceAccount<'info, TokenAccount>,
    pub clok: &'info AccountInfo<'info>,
    pub token_program: Interface<'info, TokenInterface>,
    pub sysvar_instructions: &'info AccountInfo<'info>,
}
const ACCOUNTS_LEN: usize = 11;

impl<'info> HumidifiAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            humidifi_param,
            pool,
            pool_base_token_account,
            pool_quote_token_account,
            clok,
            token_program,
            sysvar_instructions,
        ]: &[AccountInfo<'info>; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            humidifi_param,
            pool,
            pool_base_token_account: InterfaceAccount::try_from(pool_base_token_account)?,
            pool_quote_token_account: InterfaceAccount::try_from(pool_quote_token_account)?,
            clok,
            token_program: Interface::try_from(token_program)?,
            sysvar_instructions,
        })
    }

    pub fn obfuscate_instruction_data(data: &mut [u8]) {
        let mut qwords = data.chunks_exact_mut(8);
        let mut pos_mask = 0_u64;
        while let Some(qword) = qwords
            .next()
            .map(|q| unsafe { &mut *q.as_mut_ptr().cast::<u64>() })
        {
            *qword ^= HUMIDIFI_IX_DATA_KEY;
            *qword ^= pos_mask;
            pos_mask = pos_mask.wrapping_add(0x0001_0001_0001_0001);
        }
        let remainder = qwords.into_remainder();
        let mut rem = 0_u64;
        unsafe {
            core::ptr::copy_nonoverlapping(
                remainder.as_ptr(),
                &mut rem as *mut u64 as *mut u8,
                remainder.len(),
            );
        }
        rem ^= HUMIDIFI_IX_DATA_KEY;
        rem ^= pos_mask;
        unsafe {
            core::ptr::copy_nonoverlapping(
                &rem as *const u64 as *const u8,
                remainder.as_mut_ptr(),
                remainder.len(),
            )
        }
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
    msg!("Dex::Humidifi amount_in: {}, offset: {}", amount_in, offset);

    require!(
        remaining_accounts.len() >= *offset + ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );

    let mut swap_accounts = HumidifiAccounts::parse_accounts(remaining_accounts, *offset)?;

    if swap_accounts.dex_program_id.key != &humidifi_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }

    swap_accounts.pool.key().log();

    before_check(
        &swap_accounts.swap_authority_pubkey,
        &swap_accounts.swap_source_token,
        swap_accounts.swap_destination_token.key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let is_base_in = if swap_accounts.swap_source_token.mint
        == swap_accounts.pool_base_token_account.mint
    {
        true
    } else if swap_accounts.swap_source_token.mint == swap_accounts.pool_quote_token_account.mint {
        false
    } else {
        return Err(ErrorCode::InvalidTokenMint.into());
    };

    let expected_destination_mint = if is_base_in {
        swap_accounts.pool_quote_token_account.mint
    } else {
        swap_accounts.pool_base_token_account.mint
    };

    if swap_accounts.swap_destination_token.mint != expected_destination_mint {
        return Err(ErrorCode::InvalidTokenMint.into());
    }

    let (base_account, quote_account) = if is_base_in {
        (
            swap_accounts.swap_source_token.clone(),
            swap_accounts.swap_destination_token.clone(),
        )
    } else {
        (
            swap_accounts.swap_destination_token.clone(),
            swap_accounts.swap_source_token.clone(),
        )
    };

    // Extract swap_id from humidifi_param account
    let humidifi_param_data = swap_accounts.humidifi_param.key().as_array().clone();
    let swap_id = u64::from_le_bytes(
        humidifi_param_data[0..8]
            .try_into()
            .map_err(|_| ErrorCode::InvalidTokenMint)?,
    );
    let unused = &humidifi_param_data[8..32];
    require!(unused == &[0u8; 24], ErrorCode::InvalidTokenMint);

    let swap_params: SwapParams = SwapParams {
        swap_id,
        amount_in,
        is_base_to_quote: !is_base_in as u8,
        padding: [0; 7],
    };

    let mut data: Vec<u8> = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(&swap_params.try_to_vec()?);
    data.extend_from_slice(&[HUMIDIFI_SWAP_SELECTOR]);
    HumidifiAccounts::obfuscate_instruction_data(&mut data);

    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new(swap_accounts.pool.key(), false),
        AccountMeta::new(swap_accounts.pool_base_token_account.key(), false),
        AccountMeta::new(swap_accounts.pool_quote_token_account.key(), false),
        AccountMeta::new(base_account.key(), false),
        AccountMeta::new(quote_account.key(), false),
        AccountMeta::new_readonly(swap_accounts.clok.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.sysvar_instructions.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.pool.to_account_info(),
        swap_accounts.pool_base_token_account.to_account_info(),
        swap_accounts.pool_quote_token_account.to_account_info(),
        base_account.to_account_info(),
        quote_account.to_account_info(),
        swap_accounts.clok.to_account_info(),
        swap_accounts.token_program.to_account_info(),
        swap_accounts.sysvar_instructions.to_account_info(),
    ];

    let instruction = Instruction {
        program_id: swap_accounts.dex_program_id.key(),
        accounts,
        data,
    };

    let dex_processor = &HumidifiProcessor;
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
