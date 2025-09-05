use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{solfi_program, solfi_v2_program, HopAccounts};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token::Token;
use anchor_spl::token_interface::{TokenAccount, TokenInterface};
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
                swap_accounts.swap_source_token.clone(),
                swap_accounts.swap_destination_token.clone(),
            )
        } else if swap_accounts.swap_source_token.mint == swap_accounts.pool_token_account_b.mint
            && swap_accounts.swap_destination_token.mint == swap_accounts.pool_token_account_a.mint
        {
            (
                1u8,
                swap_accounts.swap_destination_token.clone(),
                swap_accounts.swap_source_token.clone(),
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


pub struct SolfiAccountV2<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub market: &'info AccountInfo<'info>,
    pub oracle: &'info AccountInfo<'info>,
    pub global_config_account: &'info AccountInfo<'info>,
    pub base_vault: InterfaceAccount<'info, TokenAccount>,
    pub quote_vault: InterfaceAccount<'info, TokenAccount>,
    pub base_mint: &'info AccountInfo<'info>,
    pub quote_mint: &'info AccountInfo<'info>,
    pub base_token_program: Interface<'info, TokenInterface>,
    pub quote_token_program: Interface<'info, TokenInterface>,
    pub instruction_sysvar: &'info AccountInfo<'info>,
}

const V2_ACCOUNTS_LEN: usize = 14;

impl<'info> SolfiAccountV2<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            market,
            oracle,
            global_config_account,
            base_vault,
            quote_vault,
            base_mint,
            quote_mint,
            base_token_program, 
            quote_token_program,
            instruction_sysvar,
        ]: & [AccountInfo<'info>; V2_ACCOUNTS_LEN] = array_ref![accounts, offset, V2_ACCOUNTS_LEN];
        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            market,
            oracle,
            global_config_account,
            base_vault: InterfaceAccount::try_from(base_vault)?,
            quote_vault: InterfaceAccount::try_from(quote_vault)?,
            base_mint,
            quote_mint,
            base_token_program: Interface::try_from(base_token_program)?,
            quote_token_program: Interface::try_from(quote_token_program)?,
            instruction_sysvar,
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
    msg!("Dex::SolfiV2 amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + V2_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );
    let mut swap_accounts = SolfiAccountV2::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &solfi_v2_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.market.key().log();
    
    let (direction, user_base_token_account, user_quote_token_account) =
    if swap_accounts.swap_source_token.mint == swap_accounts.base_mint.key()
        && swap_accounts.swap_destination_token.mint == swap_accounts.quote_mint.key()
    {
        (
            0u8,
            swap_accounts.swap_source_token.clone(),
            swap_accounts.swap_destination_token.clone(),
        )
    } else if swap_accounts.swap_source_token.mint == swap_accounts.quote_mint.key()
        && swap_accounts.swap_destination_token.mint == swap_accounts.base_mint.key()
    {
        
        (
            1u8,
            swap_accounts.swap_destination_token.clone(),
            swap_accounts.swap_source_token.clone(),
        )
        
    } else {
        return Err(ErrorCode::InvalidTokenMint.into());
    };

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.push(7u8); //discriminator
    data.extend_from_slice(&amount_in.to_le_bytes()); //amount_in
    data.extend_from_slice(&1u64.to_le_bytes());
    data.extend_from_slice(&direction.to_le_bytes()); //swap direction

    let accounts = vec![
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new(swap_accounts.market.key(), false),
        AccountMeta::new_readonly(swap_accounts.oracle.key(), false),
        AccountMeta::new_readonly(swap_accounts.global_config_account.key(), false),
        AccountMeta::new(swap_accounts.base_vault.key(), false),
        AccountMeta::new(swap_accounts.quote_vault.key(), false),
        AccountMeta::new(user_base_token_account.key(), false),
        AccountMeta::new(user_quote_token_account.key(), false),
        AccountMeta::new_readonly(swap_accounts.base_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.quote_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.base_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.quote_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.instruction_sysvar.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.market.to_account_info(),
        swap_accounts.oracle.to_account_info(),
        swap_accounts.global_config_account.to_account_info(),
        swap_accounts.base_vault.to_account_info(),
        swap_accounts.quote_vault.to_account_info(),
        user_base_token_account.to_account_info(),
        user_quote_token_account.to_account_info(),
        swap_accounts.base_mint.to_account_info(),
        swap_accounts.quote_mint.to_account_info(),
        swap_accounts.base_token_program.to_account_info(),
        swap_accounts.quote_token_program.to_account_info(),
        swap_accounts.instruction_sysvar.to_account_info(),
    ];

    let instruction = Instruction {
        program_id: swap_accounts.dex_program_id.key(),
        accounts,
        data,
    };

    let dex_processor = &SolfiProcessor;
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