use super::common::DexProcessor;
use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::utils::{close_token_account, sync_wsol_account, transfer_sol};
use crate::{
    BOOPFUN_BUY_SELECTOR, BOOPFUN_SELL_SELECTOR, HopAccounts, MIN_SOL_ACCOUNT_RENT,
    SA_AUTHORITY_SEED, SOL_DIFF_LIMIT, ZERO_ADDRESS, authority_pda, boopfun_program, wsol_sa,
};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::Token;
use anchor_spl::token_interface::{Mint, TokenAccount};
use arrayref::array_ref;

const ARGS_LEN: usize = 24;

pub fn boopfun_before_check(
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

pub struct BoopfunBuyAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    // Boop.fun specific account
    pub mint: InterfaceAccount<'info, Mint>,
    pub bonding_curve: &'info AccountInfo<'info>,
    pub trading_fees_vault: &'info AccountInfo<'info>,
    pub bonding_curve_vault: InterfaceAccount<'info, TokenAccount>,
    pub bonding_curve_sol_vault: &'info AccountInfo<'info>,
    pub config: &'info AccountInfo<'info>,
    pub vault_authority: &'info AccountInfo<'info>,
    pub wsol: &'info AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

pub struct BoopfunSellAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    // Boop.fun specific account
    pub mint: InterfaceAccount<'info, Mint>,
    pub bonding_curve: &'info AccountInfo<'info>,
    pub trading_fees_vault: &'info AccountInfo<'info>,
    pub bonding_curve_vault: InterfaceAccount<'info, TokenAccount>,
    pub bonding_curve_sol_vault: &'info AccountInfo<'info>,
    pub seller_token_account: InterfaceAccount<'info, TokenAccount>,
    pub seller: &'info AccountInfo<'info>,
    pub recipient: &'info AccountInfo<'info>,
    pub config: &'info AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

const BUY_ACCOUNTS_LEN: usize = 15;
const SELL_ACCOUNTS_LEN: usize = 16;

pub struct BoopfunBuyProcessor;
impl DexProcessor for BoopfunBuyProcessor {
    fn before_invoke(&self, account_infos: &[AccountInfo]) -> Result<u64> {
        let source_token_account = account_infos.last().unwrap();
        let token_program = account_infos.get(11).unwrap();
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
            let payer = account_infos.get(14).unwrap();
            let authority = account_infos.get(6).unwrap();

            if authority.key() == authority_pda::ID {
                let after_sa_authority_lamports = authority.lamports();
                let diff_sa_lamports =
                    after_sa_authority_lamports.saturating_sub(before_sa_authority_lamports);
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
                }
            }
        }
        Ok(0)
    }
}

impl<'info> BoopfunBuyAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            mint,
            bonding_curve,
            trading_fees_vault,
            bonding_curve_vault,
            bonding_curve_sol_vault,
            config,
            vault_authority,
            wsol,
            system_program,
            token_program,
            associated_token_program,
        ]: &[AccountInfo<'info>; BUY_ACCOUNTS_LEN] = array_ref![accounts, offset, BUY_ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            mint: InterfaceAccount::try_from(mint)?,
            bonding_curve,
            trading_fees_vault,
            bonding_curve_vault: InterfaceAccount::try_from(bonding_curve_vault)?,
            bonding_curve_sol_vault,
            config,
            vault_authority,
            wsol,
            system_program: Program::try_from(system_program)?,
            token_program: Program::try_from(token_program)?,
            associated_token_program: Program::try_from(associated_token_program)?,
        })
    }
}

impl<'info> BoopfunSellAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            mint,
            bonding_curve,
            trading_fees_vault,
            bonding_curve_vault,
            bonding_curve_sol_vault,
            seller_token_account,
            seller,
            recipient,
            config,
            system_program,
            token_program,
            associated_token_program,
        ]: &[AccountInfo<'info>; SELL_ACCOUNTS_LEN] =
            array_ref![accounts, offset, SELL_ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            mint: InterfaceAccount::try_from(mint)?,
            bonding_curve,
            trading_fees_vault,
            bonding_curve_vault: InterfaceAccount::try_from(bonding_curve_vault)?,
            bonding_curve_sol_vault,
            seller_token_account: InterfaceAccount::try_from(seller_token_account)?,
            seller,
            recipient,
            config,
            system_program: Program::try_from(system_program)?,
            token_program: Program::try_from(token_program)?,
            associated_token_program: Program::try_from(associated_token_program)?,
        })
    }

    fn calculate_sol_amount_out(
        accounts: &BoopfunSellAccounts,
        token_amount_in: u64,
    ) -> Result<u64> {
        let data = accounts.bonding_curve.try_borrow_data()?;

        let virtual_sol_reserves = u64::from_le_bytes(*array_ref![data, 8 + 32 + 32, 8]);
        let virtual_token_reserves = u64::from_le_bytes(*array_ref![data, 8 + 32 + 32 + 8, 8]);
        let sol_reserves = u64::from_le_bytes(*array_ref![data, 8 + 32 + 32 + 8 + 8 + 8 + 8, 8]);
        let token_reserves =
            u64::from_le_bytes(*array_ref![data, 8 + 32 + 32 + 8 + 8 + 8 + 8 + 8, 8]);
        let damping_term = data[8 + 32 + 32 + 8 + 8 + 8 + 8 + 8 + 8];
        let swap_fee_basis_points = data[8 + 32 + 32 + 8 + 8 + 8 + 8 + 8 + 8 + 1];

        let lamports_per_sol = 1_000_000_000;
        match damping_term {
            30 => Self::calculate_output_damping(
                damping_term as u64,
                virtual_token_reserves,
                token_reserves,
                token_amount_in,
                swap_fee_basis_points as u64,
                lamports_per_sol,
            ),
            31 => Self::calculate_output_xyk(
                virtual_sol_reserves,
                sol_reserves,
                token_reserves,
                token_amount_in,
                swap_fee_basis_points as u64,
                lamports_per_sol,
            ),
            _ => Err(error!(ErrorCode::InvalidDampingTerm)),
        }
    }
    fn calculate_output_damping(
        damping_term: u64,
        virtual_token_reserves: u64,
        token_reserves: u64,
        amount_in: u64,
        swap_fee_basis_points: u64,
        lamports_per_sol: u64,
    ) -> Result<u64> {
        let scaling_factor: u128 = (damping_term as u128)
            .checked_mul(virtual_token_reserves as u128)
            .and_then(|v| v.checked_mul(lamports_per_sol as u128))
            .ok_or(ErrorCode::CalculationError)?;

        let tokens_issued: u128 = (1_000_000_000u128)
            .checked_mul(lamports_per_sol as u128)
            .and_then(|v| v.checked_sub(token_reserves as u128))
            .ok_or(ErrorCode::CalculationError)?;

        let base_denominator: u128 = (virtual_token_reserves as u128)
            .checked_sub(tokens_issued)
            .ok_or(ErrorCode::CalculationError)?;

        let amount_out_before_fee: u128 = scaling_factor
            .checked_div(base_denominator)
            .and_then(|a| {
                scaling_factor
                    .checked_div(base_denominator.checked_add(amount_in as u128).unwrap_or(0))
                    .and_then(|b| a.checked_sub(b))
            })
            .ok_or(ErrorCode::CalculationError)?;

        let amount_out: u128 = amount_out_before_fee
            .checked_mul(10_000u128 - swap_fee_basis_points as u128)
            .and_then(|v| v.checked_div(10_000u128))
            .ok_or(ErrorCode::CalculationError)?;

        Ok(amount_out.try_into().map_err(|_| ErrorCode::CalculationError)?)
    }

    fn calculate_output_xyk(
        virtual_sol_reserves: u64,
        sol_reserves: u64,
        token_reserves: u64,
        amount_in: u64,
        swap_fee_basis_points: u64,
        lamports_per_sol: u64,
    ) -> Result<u64> {
        let current_x: u128 = (virtual_sol_reserves as u128)
            .checked_add(sol_reserves as u128)
            .ok_or(ErrorCode::CalculationError)?;

        let token_total_supply: u128 = (1_000_000_000u128)
            .checked_mul(lamports_per_sol as u128)
            .ok_or(ErrorCode::CalculationError)?;

        let invariant: u128 = (virtual_sol_reserves as u128)
            .checked_mul(token_total_supply)
            .ok_or(ErrorCode::CalculationError)?;

        let denominator: u128 = (token_reserves as u128)
            .checked_add(amount_in as u128)
            .ok_or(ErrorCode::CalculationError)?;

        let new_x: u128 = invariant.checked_div(denominator).ok_or(ErrorCode::CalculationError)?;

        let amount_out_before_fee: u128 =
            current_x.checked_sub(new_x).ok_or(ErrorCode::CalculationError)?;

        let amount_out: u128 = amount_out_before_fee
            .checked_mul(10_000u128 - swap_fee_basis_points as u128)
            .and_then(|v| v.checked_div(10_000u128))
            .ok_or(ErrorCode::CalculationError)?;

        Ok(amount_out.try_into().map_err(|_| ErrorCode::CalculationError)?)
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
    msg!("Dex::Boopfun amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + BUY_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );

    let mut swap_accounts = BoopfunBuyAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &boopfun_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }

    // Record pool address
    swap_accounts.bonding_curve.key().log();

    boopfun_before_check(
        swap_accounts.swap_authority_pubkey,
        &swap_accounts.swap_source_token,
        swap_accounts.swap_destination_token.key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(BOOPFUN_BUY_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&1u64.to_le_bytes());

    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.mint.key(), false),
        AccountMeta::new(swap_accounts.bonding_curve.key(), false),
        AccountMeta::new(swap_accounts.trading_fees_vault.key(), false),
        AccountMeta::new(swap_accounts.bonding_curve_vault.key(), false),
        AccountMeta::new(swap_accounts.bonding_curve_sol_vault.key(), false),
        AccountMeta::new(swap_accounts.swap_destination_token.key(), false),
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.config.key(), false),
        AccountMeta::new_readonly(swap_accounts.vault_authority.key(), false),
        AccountMeta::new_readonly(swap_accounts.wsol.key(), false),
        AccountMeta::new_readonly(swap_accounts.system_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.associated_token_program.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.mint.to_account_info(),
        swap_accounts.bonding_curve.to_account_info(),
        swap_accounts.trading_fees_vault.to_account_info(),
        swap_accounts.bonding_curve_vault.to_account_info(),
        swap_accounts.bonding_curve_sol_vault.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.config.to_account_info(),
        swap_accounts.vault_authority.to_account_info(),
        swap_accounts.wsol.to_account_info(),
        swap_accounts.system_program.to_account_info(),
        swap_accounts.token_program.to_account_info(),
        swap_accounts.associated_token_program.to_account_info(),
        swap_accounts.dex_program_id.to_account_info(),
        payer.unwrap().to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &BoopfunBuyProcessor;
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
        BUY_ACCOUNTS_LEN,
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
    msg!("Dex::BoopfunSell amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + SELL_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );

    let mut swap_accounts = BoopfunSellAccounts::parse_accounts(remaining_accounts, *offset)?;

    if swap_accounts.dex_program_id.key != &boopfun_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }

    swap_accounts.bonding_curve.key().log();

    before_check(
        swap_accounts.swap_authority_pubkey,
        &swap_accounts.swap_source_token,
        swap_accounts.swap_destination_token.key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    // Calculate expected output SOL amount, fee already deducted
    let expected_amount_out =
        BoopfunSellAccounts::calculate_sol_amount_out(&swap_accounts, amount_in)?;
    msg!("calculate_sol_amount_out: {}", expected_amount_out);

    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(BOOPFUN_SELL_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&1u64.to_le_bytes());

    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.mint.key(), false),
        AccountMeta::new(swap_accounts.bonding_curve.key(), false),
        AccountMeta::new(swap_accounts.trading_fees_vault.key(), false),
        AccountMeta::new(swap_accounts.bonding_curve_vault.key(), false),
        AccountMeta::new(swap_accounts.bonding_curve_sol_vault.key(), false),
        AccountMeta::new(swap_accounts.seller_token_account.key(), false),
        AccountMeta::new(swap_accounts.seller.key(), true),
        AccountMeta::new(swap_accounts.recipient.key(), true),
        AccountMeta::new_readonly(swap_accounts.config.key(), false),
        AccountMeta::new_readonly(swap_accounts.system_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.associated_token_program.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.mint.to_account_info(),
        swap_accounts.bonding_curve.to_account_info(),
        swap_accounts.trading_fees_vault.to_account_info(),
        swap_accounts.bonding_curve_vault.to_account_info(),
        swap_accounts.bonding_curve_sol_vault.to_account_info(),
        swap_accounts.seller_token_account.to_account_info(),
        swap_accounts.seller.to_account_info(),
        swap_accounts.recipient.to_account_info(),
        swap_accounts.config.to_account_info(),
        swap_accounts.system_program.to_account_info(),
        swap_accounts.token_program.to_account_info(),
        swap_accounts.associated_token_program.to_account_info(),
        swap_accounts.dex_program_id.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
    ];

    let instruction = Instruction { program_id: boopfun_program::id(), accounts, data };

    let dex_processor = &BoopfunSellProcessor { amount: expected_amount_out };
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
        SELL_ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )?;
    Ok(amount_out)
}

pub struct BoopfunSellProcessor {
    pub amount: u64,
}

impl DexProcessor for BoopfunSellProcessor {
    fn before_invoke(&self, account_infos: &[AccountInfo]) -> Result<u64> {
        let destination_token_account = account_infos.last().unwrap();
        let balance = destination_token_account.get_lamports();
        Ok(balance)
    }

    fn after_invoke(
        &self,
        account_infos: &[AccountInfo],
        hop: usize,
        owner_seeds: Option<&[&[&[u8]]]>,
        _before_sa_authority_lamports: u64,
    ) -> Result<u64> {
        let destination_token_account = account_infos.last().unwrap();
        let authority = account_infos.get(6).unwrap();
        let token_program = account_infos.get(10).unwrap();

        let signer_seeds: Option<&[&[&[u8]]]> = if authority.key() == authority_pda::ID {
            Some(SA_AUTHORITY_SEED)
        } else if hop == 0 && owner_seeds.is_some() {
            Some(owner_seeds.unwrap())
        } else {
            None
        };
        transfer_sol(
            authority.to_account_info(),
            destination_token_account.to_account_info(),
            self.amount,
            signer_seeds,
        )?;
        sync_wsol_account(
            destination_token_account.to_account_info(),
            token_program.to_account_info(),
            signer_seeds,
        )?;
        Ok(self.amount)
    }
}
