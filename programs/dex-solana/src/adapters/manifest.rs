use crate::adapters::common::{DexProcessor, before_check, invoke_process};
use crate::error::ErrorCode;
use crate::{HopAccounts, MANIFEST_SWAP_SELECTOR, manifest_program};
use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use arrayref::array_ref;
use borsh::{BorshDeserialize, BorshSerialize};

const ARGS_LEN: usize = 19; // Length after SwapParams serialization

pub struct ManifestProcessor;
impl DexProcessor for ManifestProcessor {}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct SwapParams {
    pub in_atoms: u64,
    pub out_atoms: u64,
    pub is_base_in: bool,
    pub is_exact_in: bool,
}

pub struct ManifestAccount<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    // Manifest pool accounts
    pub market: &'info AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub base_vault: InterfaceAccount<'info, TokenAccount>,
    pub quote_vault: InterfaceAccount<'info, TokenAccount>,
    pub token_program_base: Interface<'info, TokenInterface>,
    pub base_mint: InterfaceAccount<'info, Mint>,
    pub token_program_quote: Interface<'info, TokenInterface>,
    pub quote_mint: InterfaceAccount<'info, Mint>,
    pub global: &'info AccountInfo<'info>,
    pub global_vault: &'info AccountInfo<'info>,
}
const ACCOUNTS_LEN: usize = 14;

impl<'info> ManifestAccount<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            market,
            system_program,
            base_vault,
            quote_vault,
            token_program_base,
            base_mint,
            token_program_quote,
            quote_mint,
            global,
            global_vault,
        ]: &[AccountInfo<'info>; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            market,
            system_program: Program::try_from(system_program)?,
            base_vault: InterfaceAccount::try_from(base_vault)?,
            quote_vault: InterfaceAccount::try_from(quote_vault)?,
            token_program_base: Interface::try_from(token_program_base)?,
            base_mint: InterfaceAccount::try_from(base_mint)?,
            token_program_quote: Interface::try_from(token_program_quote)?,
            quote_mint: InterfaceAccount::try_from(quote_mint)?,
            global,
            global_vault,
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
    msg!("Dex::Manifest amount_in: {}, offset: {}", amount_in, offset);

    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);

    let mut swap_accounts = ManifestAccount::parse_accounts(remaining_accounts, *offset)?;

    // Verify program ID
    if swap_accounts.dex_program_id.key != &manifest_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }

    // Record market address
    swap_accounts.market.key().log();

    // Standard check
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

    // Determine swap direction: check if source token is base or quote
    let is_base_in = if swap_accounts.swap_source_token.mint == swap_accounts.base_mint.key() {
        // Source token is base mint, means selling base to buy quote
        true
    } else if swap_accounts.swap_source_token.mint == swap_accounts.quote_mint.key() {
        // Source token is quote mint, means selling quote to buy base
        false
    } else {
        return Err(ErrorCode::InvalidTokenMint.into());
    };

    // Verify the match of target token
    let expected_destination_mint = if is_base_in {
        swap_accounts.quote_mint.key() // base->quote, target should be quote
    } else {
        swap_accounts.base_mint.key() // quote->base, target should be base
    };

    if swap_accounts.swap_destination_token.mint != expected_destination_mint {
        return Err(ErrorCode::InvalidTokenMint.into());
    }

    // Build Manifest SwapParams
    let swap_params = SwapParams {
        in_atoms: amount_in,
        out_atoms: 1, // Minimum output, let the market decide actual output
        is_base_in,
        is_exact_in: true, // Exact input mode
    };

    // Build instruction data
    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(MANIFEST_SWAP_SELECTOR);
    data.extend_from_slice(&swap_params.try_to_vec()?);

    // Determine the correct order of user token accounts: base mint account first, quote mint account last
    let is_source_base = swap_accounts.swap_source_token.mint == swap_accounts.base_mint.key();
    let (trader_base_key, trader_quote_key) = if is_source_base {
        (swap_source_token, swap_destination_token)
    } else {
        (swap_destination_token, swap_source_token)
    };
    let (trader_base_info, trader_quote_info) = if is_source_base {
        (
            swap_accounts.swap_source_token.to_account_info(),
            swap_accounts.swap_destination_token.to_account_info(),
        )
    } else {
        (
            swap_accounts.swap_destination_token.to_account_info(),
            swap_accounts.swap_source_token.to_account_info(),
        )
    };

    // Build instruction accounts (based on Manifest swap account order)
    let accounts = vec![
        // Market related accounts
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true), // Payer
        AccountMeta::new(swap_accounts.market.key(), false),               // market
        AccountMeta::new_readonly(swap_accounts.system_program.key(), false), // system_program - must be read-only
        AccountMeta::new(trader_base_key, false), // trader_base (user account corresponding to base mint)
        AccountMeta::new(trader_quote_key, false), // trader_quote (user account corresponding to quote mint)
        AccountMeta::new(swap_accounts.base_vault.key(), false),
        AccountMeta::new(swap_accounts.quote_vault.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program_base.key(), false),
        AccountMeta::new_readonly(swap_accounts.base_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program_quote.key(), false),
        AccountMeta::new_readonly(swap_accounts.quote_mint.key(), false),
        AccountMeta::new(swap_accounts.global.key(), false), // global should be writable
        AccountMeta::new(swap_accounts.global_vault.key(), false), // global_vault should be writable
    ];

    // Build AccountInfo list (in the same order as AccountMeta)
    let account_infos = vec![
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.market.to_account_info(),
        swap_accounts.system_program.to_account_info(),
        trader_base_info,
        trader_quote_info,
        swap_accounts.base_vault.to_account_info(),
        swap_accounts.quote_vault.to_account_info(),
        swap_accounts.token_program_base.to_account_info(),
        swap_accounts.base_mint.to_account_info(),
        swap_accounts.token_program_quote.to_account_info(),
        swap_accounts.quote_mint.to_account_info(),
        swap_accounts.global.to_account_info(),
        swap_accounts.global_vault.to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &ManifestProcessor;
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
