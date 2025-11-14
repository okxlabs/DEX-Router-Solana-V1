use super::common::DexProcessor;
use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::utils::{close_token_account, log_sa_lamports_info, sync_wsol_account, transfer_sol};
use crate::{
    BUY_EXACT_IN_SELECTOR, HopAccounts, MIN_SOL_ACCOUNT_RENT, SA_AUTHORITY_SEED,
    SELL_EXACT_IN_SELECTOR, SOL_DIFF_LIMIT, ZERO_ADDRESS, authority_pda, sugar_money_program,
    wsol_sa,
};

use anchor_spl::token::Token;
use borsh::{BorshDeserialize, BorshSerialize};

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use arrayref::array_ref;

const ARGS_LEN: usize = 33;

#[derive(BorshDeserialize, BorshSerialize)]
pub struct SwapParams {
    pub bumps: InstructionBumps,
    pub amount_in: u64,
    pub min_amount_out: u64,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct InstructionBumps {
    pub bonding_curve: u8,
    pub bonding_curve_sol_associated_account: u8,
}

pub struct SugarMoneyAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority: &'info AccountInfo<'info>,
    pub swap_source_account: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_account: InterfaceAccount<'info, TokenAccount>,

    pub state: &'info AccountInfo<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub bonding_curve: &'info AccountInfo<'info>,
    pub bonding_curve_sol_associated_account: &'info AccountInfo<'info>,
    pub bonding_curve_token_associated_account: &'info AccountInfo<'info>,
    pub fee_receiver: &'info AccountInfo<'info>,
    pub wsol_program: Program<'info, Token>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: &'info AccountInfo<'info>,
    pub event_authority: &'info AccountInfo<'info>,
}
const ACCOUNTS_LEN: usize = 16;

pub struct SugarMoneyBuyProcessor;
impl DexProcessor for SugarMoneyBuyProcessor {
    fn before_invoke(&self, account_infos: &[AccountInfo]) -> Result<u64> {
        let source_token_account = account_infos.last().unwrap();
        let token_program = account_infos.get(9).unwrap();
        let authority = account_infos.get(6).unwrap();
        if authority.key() == authority_pda::ID {
            let before_sa_authority_lamports = authority.lamports();
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
            Ok(before_sa_authority_lamports)
        } else {
            close_token_account(
                source_token_account.to_account_info(),
                authority.to_account_info(),
                authority.to_account_info(),
                token_program.to_account_info(),
                None,
            )?;
            Ok(0)
        }
    }

    fn after_invoke(
        &self,
        account_infos: &[AccountInfo],
        _hop: usize,
        _owner_seeds: Option<&[&[&[u8]]]>,
        before_sa_authority_lamports: u64,
    ) -> Result<u64> {
        if before_sa_authority_lamports > 0 {
            let payer = account_infos.get(16).unwrap();
            let authority = account_infos.get(6).unwrap();
            if authority.key() == authority_pda::ID {
                let after_authority_lamports = authority.lamports();
                let diff_sa_lamports =
                    after_authority_lamports.saturating_sub(before_sa_authority_lamports);
                if diff_sa_lamports > 0 {
                    require!(
                        authority.lamports().checked_sub(diff_sa_lamports).unwrap()
                            >= MIN_SOL_ACCOUNT_RENT,
                        ErrorCode::InsufficientFunds
                    );
                    require!(diff_sa_lamports <= SOL_DIFF_LIMIT, ErrorCode::InvalidDiffLamports);
                    transfer_sol(
                        authority.to_account_info(),
                        payer.to_account_info(),
                        diff_sa_lamports,
                        Some(SA_AUTHORITY_SEED),
                    )?;
                    log_sa_lamports_info(
                        before_sa_authority_lamports,
                        after_authority_lamports,
                        diff_sa_lamports,
                    );
                }
            }
        }
        Ok(0)
    }
}

pub struct SugarMoneySellProcessor {
    pub sender_before_lamports: u64,
}

impl DexProcessor for SugarMoneySellProcessor {
    fn after_invoke(
        &self,
        account_infos: &[AccountInfo],
        hop: usize,
        owner_seeds: Option<&[&[&[u8]]]>,
        _before_sa_authority_lamports: u64,
    ) -> Result<u64> {
        let destination_token_account = account_infos.last().unwrap();
        let authority = account_infos.get(6).unwrap();
        let token_program = account_infos.get(9).unwrap();
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

impl<'info> SugarMoneyAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority,
            swap_source_account,
            swap_destination_account,
            state,
            mint,
            bonding_curve,
            bonding_curve_sol_associated_account,
            bonding_curve_token_associated_account,
            fee_receiver,
            wsol_program,
            token_program,
            associated_token_program,
            system_program,
            rent,
            event_authority,
        ]: &[AccountInfo<'info>; ACCOUNTS_LEN] = array_ref![accounts, offset, ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority,
            swap_source_account: InterfaceAccount::try_from(swap_source_account)?,
            swap_destination_account: InterfaceAccount::try_from(swap_destination_account)?,
            state,
            mint: InterfaceAccount::try_from(mint)?,
            bonding_curve,
            bonding_curve_sol_associated_account,
            bonding_curve_token_associated_account,
            fee_receiver,
            wsol_program: Program::try_from(wsol_program)?,
            token_program: Interface::try_from(token_program)?,
            associated_token_program: Program::try_from(associated_token_program)?,
            system_program: Program::try_from(system_program)?,
            rent,
            event_authority,
        })
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
    bonding_curve_bump: u8,
    bonding_curve_sol_associated_account_bump: u8,
) -> Result<u64> {
    msg!("Dex::SugarMoney amount_in: {}, offset: {}", amount_in, offset);
    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);

    let mut swap_accounts = SugarMoneyAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &sugar_money_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }

    // Record pool address
    swap_accounts.bonding_curve.key().log();

    sugar_money_before_check(
        &swap_accounts.swap_authority,
        &swap_accounts.swap_source_account,
        swap_accounts.swap_destination_account.key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let buy_params = SwapParams {
        bumps: InstructionBumps {
            bonding_curve: bonding_curve_bump,
            bonding_curve_sol_associated_account: bonding_curve_sol_associated_account_bump,
        },
        amount_in,
        min_amount_out: 1,
    };

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(BUY_EXACT_IN_SELECTOR);
    data.extend_from_slice(&buy_params.try_to_vec()?);

    let mut accounts = Vec::with_capacity(ACCOUNTS_LEN);
    accounts.push(AccountMeta::new_readonly(swap_accounts.state.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.mint.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.bonding_curve.key(), false));
    accounts
        .push(AccountMeta::new(swap_accounts.bonding_curve_sol_associated_account.key(), false));
    accounts
        .push(AccountMeta::new(swap_accounts.bonding_curve_token_associated_account.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.swap_destination_account.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.swap_authority.key(), true));
    accounts.push(AccountMeta::new(swap_accounts.swap_authority.key(), true));
    accounts.push(AccountMeta::new(swap_accounts.fee_receiver.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.token_program.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.associated_token_program.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.system_program.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.rent.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.event_authority.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.dex_program_id.key(), false));

    let mut account_infos = Vec::with_capacity(ACCOUNTS_LEN);
    account_infos.push(swap_accounts.state.to_account_info());
    account_infos.push(swap_accounts.mint.to_account_info());
    account_infos.push(swap_accounts.bonding_curve.to_account_info());
    account_infos.push(swap_accounts.bonding_curve_sol_associated_account.to_account_info());
    account_infos.push(swap_accounts.bonding_curve_token_associated_account.to_account_info());
    account_infos.push(swap_accounts.swap_destination_account.to_account_info());
    account_infos.push(swap_accounts.swap_authority.to_account_info());
    account_infos.push(swap_accounts.swap_authority.to_account_info());
    account_infos.push(swap_accounts.fee_receiver.to_account_info());
    account_infos.push(swap_accounts.wsol_program.to_account_info());
    account_infos.push(swap_accounts.token_program.to_account_info());
    account_infos.push(swap_accounts.associated_token_program.to_account_info());
    account_infos.push(swap_accounts.system_program.to_account_info());
    account_infos.push(swap_accounts.rent.to_account_info());
    account_infos.push(swap_accounts.event_authority.to_account_info());
    account_infos.push(swap_accounts.dex_program_id.to_account_info());
    account_infos.push(payer.unwrap().to_account_info());
    account_infos.push(swap_accounts.swap_source_account.to_account_info());

    let instruction = Instruction { program_id: sugar_money_program::id(), accounts, data };

    let dex_processor = &SugarMoneyBuyProcessor;
    let amount_out = invoke_process(
        amount_in,
        dex_processor,
        &account_infos,
        &mut swap_accounts.swap_source_account,
        &mut swap_accounts.swap_destination_account,
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

pub fn sell<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
    bonding_curve_bump: u8,
    bonding_curve_sol_associated_account_bump: u8,
) -> Result<u64> {
    msg!("Dex::SugarMoney amount_in: {}, offset: {}", amount_in, offset);
    require!(remaining_accounts.len() >= *offset + ACCOUNTS_LEN, ErrorCode::InvalidAccountsLength);

    let mut swap_accounts = SugarMoneyAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &sugar_money_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }

    // Record pool address
    swap_accounts.bonding_curve.key().log();

    // check hop accounts & swap authority
    before_check(
        &swap_accounts.swap_authority,
        &swap_accounts.swap_source_account,
        swap_accounts.swap_destination_account.key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let sell_params = SwapParams {
        bumps: InstructionBumps {
            bonding_curve: bonding_curve_bump,
            bonding_curve_sol_associated_account: bonding_curve_sol_associated_account_bump,
        },
        amount_in,
        min_amount_out: 1,
    };

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(SELL_EXACT_IN_SELECTOR);
    data.extend_from_slice(&sell_params.try_to_vec()?);

    let mut accounts = Vec::with_capacity(ACCOUNTS_LEN);
    accounts.push(AccountMeta::new_readonly(swap_accounts.state.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.mint.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.bonding_curve.key(), false));
    accounts
        .push(AccountMeta::new(swap_accounts.bonding_curve_sol_associated_account.key(), false));
    accounts
        .push(AccountMeta::new(swap_accounts.bonding_curve_token_associated_account.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.swap_source_account.key(), false));
    accounts.push(AccountMeta::new(swap_accounts.swap_authority.key(), true));
    accounts.push(AccountMeta::new(swap_accounts.swap_authority.key(), true));
    accounts.push(AccountMeta::new(swap_accounts.fee_receiver.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.token_program.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.associated_token_program.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.system_program.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.rent.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.event_authority.key(), false));
    accounts.push(AccountMeta::new_readonly(swap_accounts.dex_program_id.key(), false));

    let mut account_infos = Vec::with_capacity(ACCOUNTS_LEN);
    account_infos.push(swap_accounts.state.to_account_info());
    account_infos.push(swap_accounts.mint.to_account_info());
    account_infos.push(swap_accounts.bonding_curve.to_account_info());
    account_infos.push(swap_accounts.bonding_curve_sol_associated_account.to_account_info());
    account_infos.push(swap_accounts.bonding_curve_token_associated_account.to_account_info());
    account_infos.push(swap_accounts.swap_source_account.to_account_info());
    account_infos.push(swap_accounts.swap_authority.to_account_info());
    account_infos.push(swap_accounts.swap_authority.to_account_info());
    account_infos.push(swap_accounts.fee_receiver.to_account_info());
    account_infos.push(swap_accounts.wsol_program.to_account_info());
    account_infos.push(swap_accounts.token_program.to_account_info());
    account_infos.push(swap_accounts.associated_token_program.to_account_info());
    account_infos.push(swap_accounts.system_program.to_account_info());
    account_infos.push(swap_accounts.rent.to_account_info());
    account_infos.push(swap_accounts.event_authority.to_account_info());
    account_infos.push(swap_accounts.dex_program_id.to_account_info());
    account_infos.push(swap_accounts.swap_destination_account.to_account_info());

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &SugarMoneySellProcessor {
        sender_before_lamports: swap_accounts.swap_authority.lamports(),
    };
    let amount_out = invoke_process(
        amount_in,
        dex_processor,
        &account_infos,
        &mut swap_accounts.swap_source_account,
        &mut swap_accounts.swap_destination_account,
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

pub fn sugar_money_before_check(
    swap_authority: &AccountInfo,
    swap_source_account: &InterfaceAccount<TokenAccount>,
    swap_destination_account: Pubkey,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<()> {
    // check hop accounts
    let swap_source_token = swap_source_account.key();
    if hop_accounts.from_account != ZERO_ADDRESS {
        require_keys_eq!(
            swap_source_token,
            hop_accounts.from_account,
            ErrorCode::InvalidHopAccounts
        );
        require_keys_eq!(
            swap_destination_account,
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
    require!(swap_authority.key() == swap_source_account.owner, ErrorCode::InvalidSwapAuthority);
    if !proxy_swap && hop == 0 && owner_seeds.is_none() {
        require!(swap_authority.is_signer, ErrorCode::SwapAuthorityIsNotSigner);
    }
    Ok(())
}
