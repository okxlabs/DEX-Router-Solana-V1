use super::common::DexProcessor;
use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{zerofi_program, HopAccounts};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token_interface::{TokenAccount, TokenInterface};
use arrayref::array_ref;

const ARGS_LEN: usize = 17;

pub struct ZeroFiProcessor;
impl DexProcessor for ZeroFiProcessor {}

pub struct ZeroFiAccount<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub pair: &'info AccountInfo<'info>,
    pub vault_info_base: &'info AccountInfo<'info>,
    pub vault_base: InterfaceAccount<'info, TokenAccount>,
    pub vault_info_quote: &'info AccountInfo<'info>,
    pub vault_quote: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
    pub sysvar_instructions: &'info AccountInfo<'info>,
}
const ACCOUNTS_LEN: usize = 11;

impl<'info> ZeroFiAccount<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            pair,
            vault_info_base,
            vault_base,
            vault_info_quote,
            vault_quote,
            token_program,
            sysvar_instructions
        ]: & [AccountInfo<'info>; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];
        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            pair,
            vault_info_base,
            vault_base: InterfaceAccount::try_from(vault_base)?,
            vault_info_quote,
            vault_quote: InterfaceAccount::try_from(vault_quote)?,
            token_program: Interface::try_from(token_program)?,
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
    msg!("Dex::ZeroFi amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );

    let mut swap_accounts = ZeroFiAccount::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &zerofi_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
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

    let (vault_info_in, vault_in, vault_info_out, vault_out) =
        if swap_accounts.swap_source_token.mint == swap_accounts.vault_base.mint
            && swap_accounts.swap_destination_token.mint == swap_accounts.vault_quote.mint
        {
            (
                swap_accounts.vault_info_base,
                swap_accounts.vault_base,
                swap_accounts.vault_info_quote,
                swap_accounts.vault_quote,
            )
        } else if swap_accounts.swap_source_token.mint == swap_accounts.vault_quote.mint
            && swap_accounts.swap_destination_token.mint == swap_accounts.vault_base.mint
        {
            (
                swap_accounts.vault_info_quote,
                swap_accounts.vault_quote,
                swap_accounts.vault_info_base,
                swap_accounts.vault_base,
            )
        } else {
            return Err(ErrorCode::InvalidTokenMint.into());
        };

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.push(6u8); //discriminator
    data.extend_from_slice(&amount_in.to_le_bytes()); //amount_in
    data.extend_from_slice(&0u64.to_le_bytes()); //desired ouput token amount

    let accounts = vec![
        AccountMeta::new(swap_accounts.pair.key(), false),
        AccountMeta::new(vault_info_in.key(), false),
        AccountMeta::new(vault_in.key(), false),
        AccountMeta::new(vault_info_out.key(), false),
        AccountMeta::new(vault_out.key(), false),
        AccountMeta::new(swap_source_token.key(), false),
        AccountMeta::new(swap_destination_token.key(), false),
        AccountMeta::new_readonly(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.sysvar_instructions.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.pair.to_account_info(),
        vault_info_in.to_account_info(),
        vault_in.to_account_info(),
        vault_info_out.to_account_info(),
        vault_out.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.token_program.to_account_info(),
        swap_accounts.sysvar_instructions.to_account_info(),
    ];

    let instruction = Instruction {
        program_id: zerofi_program::id(),
        accounts,
        data,
    };

    let amount_out = invoke_process(
        amount_in,
        &ZeroFiProcessor,
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
    require!(amount_out > 0, ErrorCode::AmountOutMustBeGreaterThanZero);
    Ok(amount_out)
}
