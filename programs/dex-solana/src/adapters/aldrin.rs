use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{HopAccounts, SWAP_SELECTOR, aldrin_v1_program, aldrin_v2_program};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token::Token;
use anchor_spl::token_interface::{Mint, TokenAccount};
use arrayref::array_ref;
use num_enum::{IntoPrimitive, TryFromPrimitive};

use super::common::DexProcessor;

#[derive(
    AnchorDeserialize, AnchorSerialize, Clone, Debug, PartialEq, TryFromPrimitive, IntoPrimitive,
)]
#[repr(u8)]
pub enum Side {
    Bid = 0,
    Ask = 1,
}

const ARGS_LEN: usize = 25;

pub struct AldrinProcessor;
impl DexProcessor for AldrinProcessor {}

pub struct AldrinSwapAccountsV1<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub pool_info: &'info AccountInfo<'info>,
    pub pool_authority: &'info AccountInfo<'info>,
    pub pool_mint: InterfaceAccount<'info, Mint>,
    pub pool_coin_token_vault: InterfaceAccount<'info, TokenAccount>,
    pub pool_pc_token_vault: InterfaceAccount<'info, TokenAccount>,
    pub pool_fee_account: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}
const V1_ACCOUNTS_LEN: usize = 11;

pub struct AldrinSwapAccountsV2<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub pool_info: &'info AccountInfo<'info>,
    pub pool_authority: &'info AccountInfo<'info>,
    pub pool_mint: InterfaceAccount<'info, Mint>,
    pub pool_coin_token_vault: InterfaceAccount<'info, TokenAccount>,
    pub pool_pc_token_vault: InterfaceAccount<'info, TokenAccount>,
    pub pool_fee_account: InterfaceAccount<'info, TokenAccount>,
    pub pool_curve: &'info AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}
const V2_ACCOUNTS_LEN: usize = 12;

impl<'info> AldrinSwapAccountsV1<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            pool_info,
            pool_authority,
            pool_mint,
            pool_coin_token_vault,
            pool_pc_token_vault,
            pool_fee_account,
            token_program,
        ]: &[AccountInfo<'info>; V1_ACCOUNTS_LEN] = array_ref![accounts, offset, V1_ACCOUNTS_LEN];
        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            pool_info,
            pool_authority,
            pool_mint: InterfaceAccount::try_from(pool_mint)?,
            pool_coin_token_vault: InterfaceAccount::try_from(pool_coin_token_vault)?,
            pool_pc_token_vault: InterfaceAccount::try_from(pool_pc_token_vault)?,
            pool_fee_account: InterfaceAccount::try_from(pool_fee_account)?,
            token_program: Program::try_from(token_program)?,
        })
    }
}

impl<'info> AldrinSwapAccountsV2<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            pool_info,
            pool_authority,
            pool_mint,
            pool_coin_token_vault,
            pool_pc_token_vault,
            pool_fee_account,
            pool_curve,
            token_program,
        ]: &[AccountInfo<'info>; V2_ACCOUNTS_LEN] = array_ref![accounts, offset, V2_ACCOUNTS_LEN];
        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            pool_info,
            pool_authority,
            pool_mint: InterfaceAccount::try_from(pool_mint)?,
            pool_coin_token_vault: InterfaceAccount::try_from(pool_coin_token_vault)?,
            pool_pc_token_vault: InterfaceAccount::try_from(pool_pc_token_vault)?,
            pool_fee_account: InterfaceAccount::try_from(pool_fee_account)?,
            pool_curve,
            token_program: Program::try_from(token_program)?,
        })
    }
}

pub fn swap_v1<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::AldrinSwapV1 amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + V1_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );
    let mut swap_accounts = AldrinSwapAccountsV1::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &aldrin_v1_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.pool_info.key().log();

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

    let side;
    if swap_accounts.swap_source_token.mint == swap_accounts.pool_coin_token_vault.mint {
        side = Side::Ask;
    } else {
        side = Side::Bid;
    }
    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(SWAP_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&1u64.to_le_bytes());
    data.push(Side::into(side));

    let (user_coin_token_acc, user_pc_token_acc) = if swap_accounts.swap_source_token.mint
        == swap_accounts.pool_coin_token_vault.mint
        && swap_accounts.swap_destination_token.mint == swap_accounts.pool_pc_token_vault.mint
    {
        (swap_source_token, swap_destination_token)
    } else if swap_accounts.swap_source_token.mint == swap_accounts.pool_pc_token_vault.mint
        && swap_accounts.swap_destination_token.mint == swap_accounts.pool_coin_token_vault.mint
    {
        (swap_destination_token, swap_source_token)
    } else {
        return Err(ErrorCode::InvalidTokenMint.into());
    };

    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.pool_info.key(), false),
        AccountMeta::new_readonly(swap_accounts.pool_authority.key(), false),
        AccountMeta::new(swap_accounts.pool_mint.key(), false),
        AccountMeta::new(swap_accounts.pool_coin_token_vault.key(), false),
        AccountMeta::new(swap_accounts.pool_pc_token_vault.key(), false),
        AccountMeta::new(swap_accounts.pool_fee_account.key(), false),
        AccountMeta::new_readonly(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new(user_coin_token_acc, false),
        AccountMeta::new(user_pc_token_acc, false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.pool_info.to_account_info(),
        swap_accounts.pool_authority.to_account_info(),
        swap_accounts.pool_mint.to_account_info(),
        swap_accounts.pool_coin_token_vault.to_account_info(),
        swap_accounts.pool_pc_token_vault.to_account_info(),
        swap_accounts.pool_fee_account.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.token_program.to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &AldrinProcessor;
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
        V1_ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )?;
    Ok(amount_out)
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
    msg!("Dex::AldrinSwapV2 amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + V2_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );
    let mut swap_accounts = AldrinSwapAccountsV2::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &aldrin_v2_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.pool_info.key().log();

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

    let side;
    if swap_accounts.swap_source_token.mint == swap_accounts.pool_coin_token_vault.mint {
        side = Side::Ask;
    } else {
        side = Side::Bid;
    }
    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(SWAP_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&1u64.to_le_bytes());
    data.push(Side::into(side));

    let (user_coin_token_acc, user_pc_token_acc) = if swap_accounts.swap_source_token.mint
        == swap_accounts.pool_coin_token_vault.mint
        && swap_accounts.swap_destination_token.mint == swap_accounts.pool_pc_token_vault.mint
    {
        (swap_source_token, swap_destination_token)
    } else if swap_accounts.swap_source_token.mint == swap_accounts.pool_pc_token_vault.mint
        && swap_accounts.swap_destination_token.mint == swap_accounts.pool_coin_token_vault.mint
    {
        (swap_destination_token, swap_source_token)
    } else {
        return Err(ErrorCode::InvalidTokenMint.into());
    };

    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.pool_info.key(), false),
        AccountMeta::new_readonly(swap_accounts.pool_authority.key(), false),
        AccountMeta::new(swap_accounts.pool_mint.key(), false),
        AccountMeta::new(swap_accounts.pool_coin_token_vault.key(), false),
        AccountMeta::new(swap_accounts.pool_pc_token_vault.key(), false),
        AccountMeta::new(swap_accounts.pool_fee_account.key(), false),
        AccountMeta::new_readonly(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new(user_coin_token_acc, false),
        AccountMeta::new(user_pc_token_acc, false),
        AccountMeta::new_readonly(swap_accounts.pool_curve.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.pool_info.to_account_info(),
        swap_accounts.pool_authority.to_account_info(),
        swap_accounts.pool_mint.to_account_info(),
        swap_accounts.pool_coin_token_vault.to_account_info(),
        swap_accounts.pool_pc_token_vault.to_account_info(),
        swap_accounts.pool_fee_account.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.pool_curve.to_account_info(),
        swap_accounts.token_program.to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &AldrinProcessor;
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
        V2_ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )?;
    Ok(amount_out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_pack_swap_instruction() {
        let amount_in = 100u64;
        let mut data = Vec::with_capacity(ARGS_LEN);
        data.extend_from_slice(SWAP_SELECTOR);
        data.extend_from_slice(&amount_in.to_le_bytes());
        data.extend_from_slice(&99u64.to_le_bytes());
        data.push(Side::Ask.into());

        msg!("data.len: {}", data.len());
        assert!(data.len() == ARGS_LEN);
        assert!(
            data == vec![
                248, 198, 158, 145, 225, 117, 135, 200, 100, 0, 0, 0, 0, 0, 0, 0, 99, 0, 0, 0, 0,
                0, 0, 0, 1
            ]
        );
    }
}
