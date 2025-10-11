use super::common::DexProcessor;
use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::utils::{close_token_account, sync_wsol_account, transfer_sol};
use crate::{
    HopAccounts, MOONIT_BUY_SELECTOR, MOONIT_SELL_SELECTOR, SA_AUTHORITY_SEED, TOKEN_ACCOUNT_RENT,
    ZERO_ADDRESS, authority_pda, moonit_program, wsol_sa,
};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_spl::token_interface::TokenAccount;
use arrayref::array_ref;

pub struct MoonitAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub sender: &'info AccountInfo<'info>,
    pub sender_token_account: &'info AccountInfo<'info>,
    pub curve_account: &'info AccountInfo<'info>,
    pub curve_token_account: &'info AccountInfo<'info>,
    pub dex_fee: &'info AccountInfo<'info>,
    pub helio_fee: &'info AccountInfo<'info>,
    pub mint: &'info AccountInfo<'info>,
    pub config_account: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub associated_token_program: &'info AccountInfo<'info>,
    pub system_program: &'info AccountInfo<'info>,
    pub src_or_dst_token_account: &'info AccountInfo<'info>, // extra account for wrap/unwrap wSOL
}

const ARGS_LEN: usize = 33;
const MOONIT_ACCOUNTS_LEN: usize = 16;

impl<'info> MoonitAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            sender,
            sender_token_account,
            curve_account,
            curve_token_account,
            dex_fee,
            helio_fee,
            mint,
            config_account,
            token_program,
            associated_token_program,
            system_program,
            src_or_dst_token_account,
        ] = array_ref![accounts, offset, MOONIT_ACCOUNTS_LEN];
        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            sender,
            sender_token_account,
            curve_account,
            curve_token_account,
            dex_fee,
            helio_fee,
            mint,
            config_account,
            token_program,
            associated_token_program,
            system_program,
            src_or_dst_token_account,
        })
    }
}

pub struct MoonitBuyProcessor;
impl DexProcessor for MoonitBuyProcessor {
    fn before_invoke(&self, account_infos: &[AccountInfo]) -> Result<u64> {
        let payer = account_infos.get(12).unwrap();
        let source_token_account = account_infos.get(11).unwrap();
        let token_program = account_infos.get(8).unwrap();
        let authority = account_infos.get(0).unwrap();

        if authority.key() == authority_pda::ID {
            require!(
                source_token_account.key() != wsol_sa::ID,
                ErrorCode::InvalidSourceTokenAccount
            );
            close_token_account(
                source_token_account.to_account_info(),
                authority.to_account_info(),
                authority.to_account_info(),
                token_program.to_account_info(),
                Some(SA_AUTHORITY_SEED),
            )?;
            transfer_sol(
                authority.to_account_info(),
                payer.to_account_info(),
                TOKEN_ACCOUNT_RENT,
                Some(SA_AUTHORITY_SEED),
            )?;
        } else {
            close_token_account(
                source_token_account.to_account_info(),
                authority.to_account_info(),
                authority.to_account_info(),
                token_program.to_account_info(),
                None,
            )?;
        }
        Ok(0)
    }
}

pub struct MoonitSellProcessor {
    pub sender_before_lamports: u64,
}

impl DexProcessor for MoonitSellProcessor {
    fn after_invoke(
        &self,
        account_infos: &[AccountInfo],
        hop: usize,
        owner_seeds: Option<&[&[&[u8]]]>,
        _before_sa_authority_lamports: u64,
    ) -> Result<u64> {
        let destination_token_account = account_infos.last().unwrap();
        let authority = account_infos.get(0).unwrap();
        let token_program = account_infos.get(8).unwrap();
        let sender_after_lamports = authority.lamports();

        let signer_seeds: Option<&[&[&[u8]]]> = if authority.key() == authority_pda::ID {
            Some(SA_AUTHORITY_SEED)
        } else if hop == 0 && owner_seeds.is_some() {
            Some(owner_seeds.unwrap())
        } else {
            None
        };

        let received_lamports = sender_after_lamports
            .checked_sub(self.sender_before_lamports)
            .ok_or(ErrorCode::CalculationError)?;
        transfer_sol(
            authority.to_account_info(),
            destination_token_account.to_account_info(),
            received_lamports,
            signer_seeds,
        )?;
        sync_wsol_account(
            destination_token_account.to_account_info(),
            token_program.to_account_info(),
            signer_seeds,
        )?;
        Ok(received_lamports)
    }
}

pub fn buy<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
    payer: Option<&AccountInfo<'a>>,
) -> Result<u64> {
    msg!("Dex::Moonit amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + MOONIT_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );

    let mut swap_accounts = MoonitAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &moonit_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }

    // Record pool address
    swap_accounts.curve_account.key().log();

    // check hop accounts & swap authority
    moonit_before_check(
        &swap_accounts.swap_authority_pubkey,
        &swap_accounts.swap_source_token,
        swap_accounts.swap_destination_token.key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(MOONIT_BUY_SELECTOR);
    data.extend_from_slice(&1u64.to_le_bytes()); // token amount
    data.extend_from_slice(&amount_in.to_le_bytes()); // collateral amount
    data.extend_from_slice(&[0u8]); // fixed side (exact in)
    data.extend_from_slice(&1u64.to_le_bytes()); // slippage

    let account_metas = vec![
        AccountMeta::new(swap_accounts.sender.key(), true),
        AccountMeta::new(swap_accounts.sender_token_account.key(), false),
        AccountMeta::new(swap_accounts.curve_account.key(), false),
        AccountMeta::new(swap_accounts.curve_token_account.key(), false),
        AccountMeta::new(swap_accounts.dex_fee.key(), false),
        AccountMeta::new(swap_accounts.helio_fee.key(), false),
        AccountMeta::new_readonly(swap_accounts.mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.config_account.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.associated_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.system_program.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.sender.to_account_info(),
        swap_accounts.sender_token_account.to_account_info(),
        swap_accounts.curve_account.to_account_info(),
        swap_accounts.curve_token_account.to_account_info(),
        swap_accounts.dex_fee.to_account_info(),
        swap_accounts.helio_fee.to_account_info(),
        swap_accounts.mint.to_account_info(),
        swap_accounts.config_account.to_account_info(),
        swap_accounts.token_program.to_account_info(),
        swap_accounts.associated_token_program.to_account_info(),
        swap_accounts.system_program.to_account_info(),
        swap_accounts.src_or_dst_token_account.to_account_info(),
        payer.as_ref().unwrap().to_account_info(),
    ];

    let instruction =
        Instruction { program_id: moonit_program::id(), accounts: account_metas, data };

    let dex_processor = &MoonitBuyProcessor;
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
        MOONIT_ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )?;
    Ok(amount_out)
}

pub fn sell<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::Moonit amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + MOONIT_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );

    let mut swap_accounts = MoonitAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &moonit_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }

    // Record pool address
    swap_accounts.curve_account.key().log();

    // check hop accounts & swap authority
    before_check(
        &swap_accounts.swap_authority_pubkey,
        &swap_accounts.swap_source_token,
        swap_accounts.swap_destination_token.key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(MOONIT_SELL_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes()); // token amount
    data.extend_from_slice(&1u64.to_le_bytes()); // collateral amount
    data.extend_from_slice(&[0u8]); // fixed side (exact in mode)
    data.extend_from_slice(&1u64.to_le_bytes()); // slippage

    let account_metas = vec![
        AccountMeta::new(swap_accounts.sender.key(), true),
        AccountMeta::new(swap_accounts.sender_token_account.key(), false),
        AccountMeta::new(swap_accounts.curve_account.key(), false),
        AccountMeta::new(swap_accounts.curve_token_account.key(), false),
        AccountMeta::new(swap_accounts.dex_fee.key(), false),
        AccountMeta::new(swap_accounts.helio_fee.key(), false),
        AccountMeta::new_readonly(swap_accounts.mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.config_account.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.associated_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.system_program.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.sender.to_account_info(),
        swap_accounts.sender_token_account.to_account_info(),
        swap_accounts.curve_account.to_account_info(),
        swap_accounts.curve_token_account.to_account_info(),
        swap_accounts.dex_fee.to_account_info(),
        swap_accounts.helio_fee.to_account_info(),
        swap_accounts.mint.to_account_info(),
        swap_accounts.config_account.to_account_info(),
        swap_accounts.token_program.to_account_info(),
        swap_accounts.associated_token_program.to_account_info(),
        swap_accounts.system_program.to_account_info(),
        swap_accounts.src_or_dst_token_account.to_account_info(),
    ];

    let instruction =
        Instruction { program_id: moonit_program::id(), accounts: account_metas, data };

    let dex_processor =
        MoonitSellProcessor { sender_before_lamports: swap_accounts.sender.lamports() };
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
        MOONIT_ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )?;
    Ok(amount_out)
}

fn moonit_before_check(
    swap_authority_pubkey: &AccountInfo,
    swap_source_token_account: &InterfaceAccount<TokenAccount>,
    swap_destination_token: Pubkey,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<()> {
    // check hop accounts
    let swap_source_token = swap_source_token_account.key();
    if hop_accounts.from_account != ZERO_ADDRESS {
        require_keys_eq!(
            swap_source_token,
            hop_accounts.from_account,
            ErrorCode::InvalidHopAccounts
        );
        require_keys_eq!(
            swap_destination_token,
            hop_accounts.to_account,
            ErrorCode::InvalidHopAccounts
        );
    }
    if hop_accounts.last_to_account != ZERO_ADDRESS {
        require_keys_eq!(
            swap_source_token,
            hop_accounts.last_to_account,
            ErrorCode::InvalidHopFromAccount
        );
    }

    // check swap authority
    require!(
        swap_authority_pubkey.key() == swap_source_token_account.owner,
        ErrorCode::InvalidSwapAuthority
    );
    if !proxy_swap && hop == 0 && owner_seeds.is_none() {
        require!(swap_authority_pubkey.is_signer, ErrorCode::SwapAuthorityIsNotSigner);
    }
    Ok(())
}
