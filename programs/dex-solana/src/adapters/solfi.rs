use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{solfi_program, HopAccounts};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token::Token;
use anchor_spl::token_interface::TokenAccount;
use arrayref::array_ref;

use super::common::DexProcessor;

const ARGS_LEN: usize = 18;

pub struct SolfiProcessor;
impl DexProcessor for SolfiProcessor {}

pub struct SolfiAccount<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub pair: &'info AccountInfo<'info>,
    pub pool_token_account_a: InterfaceAccount<'info, TokenAccount>,
    pub pool_token_account_b: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub sysvar_instructions: &'info AccountInfo<'info>,
}
const ACCOUNTS_LEN: usize = 9;

impl<'info> SolfiAccount<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            pair,
            pool_token_account_a,
            pool_token_account_b,
            token_program,
            sysvar_instructions
        ]: & [AccountInfo<'info>; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];
        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            pair,
            pool_token_account_a: InterfaceAccount::try_from(pool_token_account_a)?,
            pool_token_account_b: InterfaceAccount::try_from(pool_token_account_b)?,
            token_program: Program::try_from(token_program)?,
            sysvar_instructions,
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
    msg!("Dex::Solfi amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );

    let mut swap_accounts = SolfiAccount::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &solfi_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.pair.key().log();

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

    let (direction, user_token_account_a, user_token_account_b) =
        if swap_accounts.swap_source_token.mint == swap_accounts.pool_token_account_a.mint
            && swap_accounts.swap_destination_token.mint == swap_accounts.pool_token_account_b.mint
        {
            (
                0u8,
                swap_accounts.swap_source_token,
                swap_accounts.swap_destination_token.clone(),
            )
        } else if swap_accounts.swap_source_token.mint == swap_accounts.pool_token_account_b.mint
            && swap_accounts.swap_destination_token.mint == swap_accounts.pool_token_account_a.mint
        {
            (
                1u8,
                swap_accounts.swap_destination_token.clone(),
                swap_accounts.swap_source_token,
            )
        } else {
            return Err(ErrorCode::InvalidTokenMint.into());
        };

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.push(7u8); //discriminator
    data.extend_from_slice(&amount_in.to_le_bytes()); //amount_in
    data.extend_from_slice(&0u64.to_le_bytes());
    data.extend_from_slice(&direction.to_le_bytes()); //swap direction

    let accounts = vec![
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new(swap_accounts.pair.key(), false),
        AccountMeta::new(swap_accounts.pool_token_account_a.key(), false),
        AccountMeta::new(swap_accounts.pool_token_account_b.key(), false),
        AccountMeta::new(user_token_account_a.key(), false),
        AccountMeta::new(user_token_account_b.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.sysvar_instructions.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.pair.to_account_info(),
        swap_accounts.pool_token_account_a.to_account_info(),
        swap_accounts.pool_token_account_b.to_account_info(),
        user_token_account_a.to_account_info(),
        user_token_account_b.to_account_info(),
        swap_accounts.token_program.to_account_info(),
        swap_accounts.sysvar_instructions.to_account_info(),
    ];

    let instruction = Instruction {
        program_id: swap_accounts.dex_program_id.key(),
        accounts,
        data,
    };

    let dex_processor = &SolfiProcessor;
    let amount_out = invoke_process(
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
    Ok(amount_out)
}
