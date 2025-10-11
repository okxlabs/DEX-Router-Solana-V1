use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{HopAccounts, SWAP_EXACT_IN_SELECTOR, numeraire_program, numeraire_usdstar_mint};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::{
    token::Token,
    token_2022::Token2022,
    token_interface::{Mint, TokenAccount},
};
use arrayref::array_ref;

use super::common::DexProcessor;

pub struct NumeraireProcessor;
impl DexProcessor for NumeraireProcessor {}

pub struct NumeraireSwapAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub pool: &'info AccountInfo<'info>,
    pub in_mint: InterfaceAccount<'info, Mint>,
    pub out_mint: InterfaceAccount<'info, Mint>,
    pub in_vault: &'info AccountInfo<'info>,
    pub out_vault: &'info AccountInfo<'info>,
    pub numeraire_config: &'info AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
    pub token_2022_program: Program<'info, Token2022>,
}
const ACCOUNTS_LEN: usize = 12;

impl<'info> NumeraireSwapAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            pool,
            in_mint,
            out_mint,
            in_vault,
            out_vault,
            numeraire_config,
            token_program,
            token_2022_program,
        ]: &[AccountInfo<'info>; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];
        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            pool,
            in_mint: InterfaceAccount::try_from(in_mint)?,
            out_mint: InterfaceAccount::try_from(out_mint)?,
            in_vault,
            out_vault,
            numeraire_config,
            token_program: Program::try_from(token_program)?,
            token_2022_program: Program::try_from(token_2022_program)?,
        })
    }
}

fn get_mint_index(
    mint_key: &Pubkey,
    pool_pair_0_mint: &Pubkey,
    pool_pair_1_mint: &Pubkey,
    pool_pair_2_mint: &Pubkey,
) -> Result<u8> {
    if mint_key == &numeraire_usdstar_mint::id() {
        Ok(10u8)
    } else if mint_key == pool_pair_0_mint {
        Ok(0u8)
    } else if mint_key == pool_pair_1_mint {
        Ok(1u8)
    } else if mint_key == pool_pair_2_mint {
        Ok(2u8)
    } else {
        Err(ErrorCode::InvalidMint.into())
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
    msg!("Dex::Numeraire amount_in: {}, offset: {}", amount_in, offset);
    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);
    let mut swap_accounts = NumeraireSwapAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &numeraire_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.pool.key().log();

    before_check(
        swap_accounts.swap_authority_pubkey,
        &swap_accounts.swap_source_token,
        swap_accounts.swap_destination_token.key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;
    let pool_pair_0_mint;
    let pool_pair_1_mint;
    let pool_pair_2_mint;
    {
        let pool_data = swap_accounts.pool.try_borrow_data()?;
        pool_pair_0_mint = Pubkey::try_from_slice(array_ref![pool_data, 296, 32])?;
        pool_pair_1_mint = Pubkey::try_from_slice(array_ref![pool_data, 664, 32])?;
        pool_pair_2_mint = Pubkey::try_from_slice(array_ref![pool_data, 1032, 32])?;
    }

    let in_mint_key = swap_accounts.in_mint.key();
    let out_mint_key = swap_accounts.out_mint.key();
    let in_index =
        get_mint_index(&in_mint_key, &pool_pair_0_mint, &pool_pair_1_mint, &pool_pair_2_mint)?;
    let out_index =
        get_mint_index(&out_mint_key, &pool_pair_0_mint, &pool_pair_1_mint, &pool_pair_2_mint)?;

    let mut data = Vec::with_capacity(26);
    data.extend_from_slice(SWAP_EXACT_IN_SELECTOR);
    data.extend_from_slice(&in_index.to_le_bytes());
    data.extend_from_slice(&out_index.to_le_bytes());
    data.extend_from_slice(&amount_in.to_le_bytes()); // exactAmountIn
    data.extend_from_slice(&1u64.to_le_bytes()); // min_amount_out

    let accounts = vec![
        AccountMeta::new(swap_accounts.pool.key(), false),
        AccountMeta::new(in_mint_key, false),
        AccountMeta::new(out_mint_key, false),
        AccountMeta::new(swap_accounts.swap_source_token.key(), false),
        AccountMeta::new(swap_accounts.swap_destination_token.key(), false),
        {
            if swap_accounts.in_vault.key == &numeraire_program::id() {
                AccountMeta::new_readonly(swap_accounts.in_vault.key(), false)
            } else {
                AccountMeta::new(swap_accounts.in_vault.key(), false)
            }
        },
        {
            if swap_accounts.out_vault.key == &numeraire_program::id() {
                AccountMeta::new_readonly(swap_accounts.out_vault.key(), false)
            } else {
                AccountMeta::new(swap_accounts.out_vault.key(), false)
            }
        },
        AccountMeta::new_readonly(swap_accounts.numeraire_config.key(), false),
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_2022_program.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.pool.to_account_info(),
        swap_accounts.in_mint.to_account_info(),
        swap_accounts.out_mint.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.in_vault.to_account_info(),
        swap_accounts.out_vault.to_account_info(),
        swap_accounts.numeraire_config.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.token_program.to_account_info(),
        swap_accounts.token_2022_program.to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let amount_out = invoke_process(
        amount_in,
        &NumeraireProcessor,
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
