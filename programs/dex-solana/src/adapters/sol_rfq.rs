use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{sol_rfq_program, HopAccounts, SOL_RFQ_FILL_ORDER_SELECTOR};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token_interface::TokenAccount;
use arrayref::array_ref;

use super::common::DexProcessor;

const ARGS_LEN: usize = 58;

pub struct SolRfqProcessor;
impl DexProcessor for SolRfqProcessor {}

pub struct SolRfqAccount<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub sol_rfq_param1: &'info AccountInfo<'info>,
    pub sol_rfq_param2: &'info AccountInfo<'info>,

    pub maker: &'info AccountInfo<'info>,
    pub taker: &'info AccountInfo<'info>,
    pub maker_mint: &'info AccountInfo<'info>,
    pub taker_mint: &'info AccountInfo<'info>,
    pub maker_send_token_account: &'info AccountInfo<'info>,
    pub taker_receive_token_account: &'info AccountInfo<'info>,
    pub taker_send_token_account: &'info AccountInfo<'info>,
    pub maker_receive_token_account: &'info AccountInfo<'info>,
    pub maker_token_program: &'info AccountInfo<'info>,
    pub taker_token_program: &'info AccountInfo<'info>,
    pub system_program: &'info AccountInfo<'info>,
}

const ACCOUNTS_LEN: usize = 17;

impl<'info> SolRfqAccount<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            sol_rfq_param1,
            sol_rfq_param2,
            maker,
            taker,
            maker_mint,
            taker_mint,
            maker_send_token_account,
            taker_receive_token_account,
            taker_send_token_account,
            maker_receive_token_account,
            maker_token_program,
            taker_token_program,
            system_program,
        ] = array_ref![accounts, offset, ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            sol_rfq_param1,
            sol_rfq_param2,
            maker,
            taker,
            maker_mint,
            taker_mint,
            maker_send_token_account,
            taker_receive_token_account,
            taker_send_token_account,
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
) -> Result<u64> {
    msg!("Dex::SolRfq amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );

    let mut swap_accounts = SolRfqAccount::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &sol_rfq_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log dex_program_id as pool address
    swap_accounts.dex_program_id.key().log();

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

    let sol_rfq_param1 = swap_accounts.sol_rfq_param1.key().as_array().clone();
    let rfq_id = u64::from_le_bytes(sol_rfq_param1[0..8].try_into().map_err(|_| ErrorCode::InvalidRfqParameters)?);
    let expected_maker_amount = u64::from_le_bytes(sol_rfq_param1[8..16].try_into().map_err(|_| ErrorCode::InvalidRfqParameters)?);
    let expected_taker_amount = u64::from_le_bytes(sol_rfq_param1[16..24].try_into().map_err(|_| ErrorCode::InvalidRfqParameters)?);
    let unused = &sol_rfq_param1[24..32];

    require!(unused == &[0u8; 8], ErrorCode::InvalidRfqParameters);
    
    let sol_rfq_param2 = swap_accounts.sol_rfq_param2.key().as_array().clone();
    let maker_send_amount = u64::from_le_bytes(sol_rfq_param2[0..8].try_into().map_err(|_| ErrorCode::InvalidRfqParameters)?);
    let taker_send_amount = u64::from_le_bytes(sol_rfq_param2[8..16].try_into().map_err(|_| ErrorCode::InvalidRfqParameters)?);
    let expiry = u64::from_le_bytes(sol_rfq_param2[16..24].try_into().map_err(|_| ErrorCode::InvalidRfqParameters)?);
    let unused = &sol_rfq_param2[24..32];

    require!(unused == &[0u8; 8], ErrorCode::InvalidRfqParameters);

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(SOL_RFQ_FILL_ORDER_SELECTOR);
    data.extend_from_slice(&rfq_id.to_le_bytes());
    data.extend_from_slice(&expected_maker_amount.to_le_bytes());
    data.extend_from_slice(&expected_taker_amount.to_le_bytes());
    data.extend_from_slice(&maker_send_amount.to_le_bytes());
    data.extend_from_slice(&taker_send_amount.to_le_bytes());
    data.extend_from_slice(&expiry.to_le_bytes());
    // We only support spl to spl swap in dex router.
    data.extend_from_slice(&[0u8]);
    data.extend_from_slice(&[0u8]);

    let account_metas = vec![
        AccountMeta::new(swap_accounts.maker.key(), true),
        AccountMeta::new(swap_accounts.taker.key(), true),
        AccountMeta::new_readonly(swap_accounts.maker_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.taker_mint.key(), false),
        AccountMeta::new(swap_accounts.maker_send_token_account.key(), false),
        AccountMeta::new(swap_accounts.taker_receive_token_account.key(), false),
        AccountMeta::new(swap_accounts.taker_send_token_account.key(), false),
        AccountMeta::new(swap_accounts.maker_receive_token_account.key(), false),
        AccountMeta::new_readonly(swap_accounts.maker_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.taker_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.system_program.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.maker.to_account_info(),
        swap_accounts.taker.to_account_info(),
        swap_accounts.maker_mint.to_account_info(),
        swap_accounts.taker_mint.to_account_info(),
        swap_accounts.maker_send_token_account.to_account_info(),
        swap_accounts.taker_receive_token_account.to_account_info(),
        swap_accounts.taker_send_token_account.to_account_info(),
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
        dex_processor,
        &account_infos,
        swap_source_token,
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