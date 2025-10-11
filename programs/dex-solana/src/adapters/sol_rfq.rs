use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{HopAccounts, SOL_RFQ_FILL_ORDER_SELECTOR, sol_rfq_program};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token_interface::TokenAccount;
use arrayref::array_ref;

use super::common::DexProcessor;

const ARGS_LEN: usize = 58;

pub struct SolRfqProcessor;
impl DexProcessor for SolRfqProcessor {}

pub struct SolRfqAccount<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub maker: &'info AccountInfo<'info>,
    pub maker_mint: &'info AccountInfo<'info>,
    pub taker_mint: &'info AccountInfo<'info>,
    pub maker_send_token_account: &'info AccountInfo<'info>,
    pub maker_receive_token_account: &'info AccountInfo<'info>,
    pub maker_token_program: &'info AccountInfo<'info>,
    pub taker_token_program: &'info AccountInfo<'info>,
    pub system_program: &'info AccountInfo<'info>,
}

const ACCOUNTS_LEN: usize = 12;

impl<'info> SolRfqAccount<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority,
            swap_source_token,
            swap_destination_token,
            maker,
            maker_mint,
            taker_mint,
            maker_send_token_account,
            maker_receive_token_account,
            maker_token_program,
            taker_token_program,
            system_program,
        ] = array_ref![accounts, offset, ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            maker,
            maker_mint,
            taker_mint,
            maker_send_token_account,
            maker_receive_token_account,
            maker_token_program,
            taker_token_program,
            system_program,
        })
    }
}

pub fn fill_order<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
    rfq_id: u64,
    expected_maker_amount: u64,
    expected_taker_amount: u64,
    maker_send_amount: u64,
    taker_send_amount: u64,
    expiry: u64,
    maker_use_native_sol: bool,
    taker_use_native_sol: bool,
) -> Result<u64> {
    msg!("Dex::SolRfq amount_in: {}, offset: {}", amount_in, offset);
    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);

    // We only support spl to spl swap in dex router currently.
    require!(!maker_use_native_sol && !taker_use_native_sol, ErrorCode::InvalidRfqParameters);

    require!(taker_send_amount == amount_in, ErrorCode::InvalidRfqParameters);

    let mut swap_accounts = SolRfqAccount::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &sol_rfq_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log dex_program_id as pool address
    swap_accounts.dex_program_id.key().log();

    // check hop accounts & swap authority
    let swap_destination_token = swap_accounts.swap_destination_token.key();
    before_check(
        &swap_accounts.swap_authority,
        &swap_accounts.swap_source_token,
        swap_destination_token,
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(SOL_RFQ_FILL_ORDER_SELECTOR);
    data.extend_from_slice(&rfq_id.to_le_bytes());
    data.extend_from_slice(&expected_maker_amount.to_le_bytes());
    data.extend_from_slice(&expected_taker_amount.to_le_bytes());
    data.extend_from_slice(&maker_send_amount.to_le_bytes());
    data.extend_from_slice(&taker_send_amount.to_le_bytes());
    data.extend_from_slice(&expiry.to_le_bytes());
    data.extend_from_slice(&[maker_use_native_sol as u8]);
    data.extend_from_slice(&[taker_use_native_sol as u8]);

    let account_metas = vec![
        AccountMeta::new(swap_accounts.maker.key(), true),
        AccountMeta::new(swap_accounts.swap_authority.key(), true),
        AccountMeta::new_readonly(swap_accounts.maker_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.taker_mint.key(), false),
        AccountMeta::new(swap_accounts.maker_send_token_account.key(), false),
        AccountMeta::new(swap_accounts.swap_destination_token.key(), false),
        AccountMeta::new(swap_accounts.swap_source_token.key(), false),
        AccountMeta::new(swap_accounts.maker_receive_token_account.key(), false),
        AccountMeta::new_readonly(swap_accounts.maker_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.taker_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.system_program.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.maker.to_account_info(),
        swap_accounts.swap_authority.to_account_info(),
        swap_accounts.maker_mint.to_account_info(),
        swap_accounts.taker_mint.to_account_info(),
        swap_accounts.maker_send_token_account.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.maker_receive_token_account.to_account_info(),
        swap_accounts.maker_token_program.to_account_info(),
        swap_accounts.taker_token_program.to_account_info(),
        swap_accounts.system_program.to_account_info(),
    ];

    let instruction = Instruction {
        program_id: swap_accounts.dex_program_id.key(),
        accounts: account_metas,
        data,
    };

    let dex_processor = &SolRfqProcessor;
    let _ = invoke_process(
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

    Ok(maker_send_amount) // maker_send_amount is the amount_out
}
