use super::common::DexProcessor;
use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::utils::{close_token_account, log_sa_lamports_info, sync_wsol_account, transfer_sol};
use crate::{
    HopAccounts, MIN_SOL_ACCOUNT_RENT, PUMPFUN_BUY_SELECTOR, PUMPFUN_SELL_SELECTOR,
    SA_AUTHORITY_SEED, SOL_DIFF_LIMIT, ZERO_ADDRESS, authority_pda, pumpfun_program, wsol_sa,
};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_spl::token::Token;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use arrayref::array_ref;

const ARGS_LEN: usize = 24;
const PUMPFUN_NUMERATOR: u64 = 10_000;

#[derive(Debug, Clone)]
pub struct Fees {
    pub lp_fee_bps: u64,
    pub protocol_fee_bps: u64,
    pub creator_fee_bps: u64,
}

#[derive(Debug)]
pub struct FeeTier {
    pub market_cap_lamports_threshold: u128,
    pub fees: Fees,
}

#[derive(Debug)]
pub struct FeeConfig {
    pub bump: u8,
    pub admin: Pubkey,
    pub flat_fees: Fees,
    pub fee_tiers: Vec<FeeTier>,
}

pub fn pumpfun_before_check(
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

pub struct PumpfunBuyAccounts3<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,
    pub global: &'info AccountInfo<'info>,
    pub fee_recipient: &'info AccountInfo<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub bonding_curve: &'info AccountInfo<'info>,
    pub associated_bonding_curve: &'info AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub creator_vault: &'info AccountInfo<'info>,
    pub event_authority: &'info AccountInfo<'info>,
    pub global_volume_accumulator: &'info AccountInfo<'info>,
    pub user_volume_accumulator: &'info AccountInfo<'info>,
    pub fee_config: &'info AccountInfo<'info>,
    pub fee_program: &'info AccountInfo<'info>,
}
const BUY_ACCOUNTS_LEN3: usize = 17;

pub struct PumpfunBuyProcessor3;
impl DexProcessor for PumpfunBuyProcessor3 {
    fn before_invoke(&self, account_infos: &[AccountInfo]) -> Result<u64> {
        let source_token_account = account_infos.last().unwrap();
        let token_program = account_infos.get(8).unwrap();
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
            let payer = account_infos.get(account_infos.len() - 2).unwrap();
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

impl<'info> PumpfunBuyAccounts3<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            global,
            fee_recipient,
            mint,
            bonding_curve,
            associated_bonding_curve,
            system_program,
            token_program,
            creator_vault,
            event_authority,
            global_volume_accumulator,
            user_volume_accumulator,
            fee_config,
            fee_program,
        ]: &[AccountInfo<'info>; BUY_ACCOUNTS_LEN3] =
            array_ref![accounts, offset, BUY_ACCOUNTS_LEN3];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            global,
            fee_recipient,
            mint: InterfaceAccount::try_from(mint)?,
            bonding_curve,
            associated_bonding_curve,
            system_program: Program::try_from(system_program)?,
            token_program: Program::try_from(token_program)?,
            creator_vault,
            event_authority,
            global_volume_accumulator,
            user_volume_accumulator,
            fee_config,
            fee_program,
        })
    }

    fn cal_token_amount_out(
        &self,
        amount_in: u64,
        virtual_token_reserves: u64,
        virtual_sol_reserves: u64,
    ) -> Result<u128> {
        let amount_out = (amount_in as u128)
            .checked_mul(virtual_token_reserves as u128)
            .unwrap()
            .checked_div((virtual_sol_reserves as u128).checked_add(amount_in as u128).unwrap())
            .unwrap();
        Ok(amount_out as u128)
    }
}

pub fn buy3<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
    payer: Option<&AccountInfo<'a>>,
) -> Result<u64> {
    msg!("Dex::Pumpfun amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + BUY_ACCOUNTS_LEN3,
        ErrorCode::InvalidAccountsLength
    );
    let mut swap_accounts = PumpfunBuyAccounts3::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &pumpfun_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.bonding_curve.key().log();

    pumpfun_before_check(
        swap_accounts.swap_authority_pubkey,
        &swap_accounts.swap_source_token,
        swap_accounts.swap_destination_token.key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let bonding_curve_data = swap_accounts.bonding_curve.try_borrow_data()?.to_vec();
    let virtual_token_reserves = u64::from_le_bytes(*array_ref![&bonding_curve_data, 8, 8]);
    let virtual_sol_reserves = u64::from_le_bytes(*array_ref![&bonding_curve_data, 16, 8]);
    let mint_supply = u64::from_le_bytes(*array_ref![&bonding_curve_data, 40, 8]);

    let creator =
        Pubkey::new_from_array(*array_ref![&bonding_curve_data, 8 + 8 + 8 + 8 + 8 + 8 + 1, 32]);

    let total_fee = get_fee_3(
        swap_accounts.global,
        Some(swap_accounts.fee_config),
        mint_supply,
        virtual_sol_reserves,
        virtual_token_reserves,
        amount_in,
        creator,
    )?;

    let real_amount_in = amount_in.saturating_sub(total_fee);
    let amount_out = swap_accounts
        .cal_token_amount_out(
            real_amount_in.checked_sub(1).unwrap(),
            virtual_token_reserves,
            virtual_sol_reserves,
        )
        .unwrap() as u64;

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(PUMPFUN_BUY_SELECTOR);
    data.extend_from_slice(&amount_out.to_le_bytes()); // token_amount_out
    data.extend_from_slice(&amount_in.to_le_bytes()); // max_amount_cost

    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.global.key(), false),
        AccountMeta::new(swap_accounts.fee_recipient.key(), false),
        AccountMeta::new_readonly(swap_accounts.mint.key(), false),
        AccountMeta::new(swap_accounts.bonding_curve.key(), false),
        AccountMeta::new(swap_accounts.associated_bonding_curve.key(), false),
        AccountMeta::new(swap_accounts.swap_destination_token.key(), false),
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.system_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
        AccountMeta::new(swap_accounts.creator_vault.key(), false),
        AccountMeta::new_readonly(swap_accounts.event_authority.key(), false),
        AccountMeta::new_readonly(swap_accounts.dex_program_id.key(), false),
        AccountMeta::new(swap_accounts.global_volume_accumulator.key(), false),
        AccountMeta::new(swap_accounts.user_volume_accumulator.key(), false),
        AccountMeta::new_readonly(swap_accounts.fee_config.key(), false),
        AccountMeta::new_readonly(swap_accounts.fee_program.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.global.to_account_info(),
        swap_accounts.fee_recipient.to_account_info(),
        swap_accounts.mint.to_account_info(),
        swap_accounts.bonding_curve.to_account_info(),
        swap_accounts.associated_bonding_curve.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.system_program.to_account_info(),
        swap_accounts.token_program.to_account_info(),
        swap_accounts.creator_vault.to_account_info(),
        swap_accounts.event_authority.to_account_info(),
        swap_accounts.dex_program_id.to_account_info(),
        swap_accounts.global_volume_accumulator.to_account_info(),
        swap_accounts.user_volume_accumulator.to_account_info(),
        swap_accounts.fee_config.to_account_info(),
        swap_accounts.fee_program.to_account_info(),
        payer.unwrap().to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &PumpfunBuyProcessor3;
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
        BUY_ACCOUNTS_LEN3,
        proxy_swap,
        None,
    )?;
    Ok(amount_out)
}
pub struct PumpfunSellProcessor3 {
    pub amount: u64,
}

impl DexProcessor for PumpfunSellProcessor3 {
    fn before_invoke(&self, account_infos: &[AccountInfo]) -> Result<u64> {
        let authority = account_infos.get(6).unwrap();
        if authority.key() == authority_pda::ID {
            let before_authority_lamports = authority.lamports();
            Ok(before_authority_lamports)
        } else {
            Ok(0)
        }
    }

    fn after_invoke(
        &self,
        account_infos: &[AccountInfo],
        hop: usize,
        owner_seeds: Option<&[&[&[u8]]]>,
        before_sa_authority_lamports: u64,
    ) -> Result<u64> {
        let destination_token_account = account_infos.get(account_infos.len() - 2).unwrap();
        let authority = account_infos.get(6).unwrap();
        let token_program = account_infos.get(9).unwrap();
        let payer = account_infos.last().unwrap();

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

        if authority.key() == authority_pda::ID {
            let after_authority_lamports = authority.lamports();
            let diff_lamports =
                before_sa_authority_lamports.saturating_sub(after_authority_lamports);
            require!(diff_lamports <= SOL_DIFF_LIMIT, ErrorCode::InvalidDiffLamports);
            if diff_lamports > 0 {
                transfer_sol(
                    payer.to_account_info(),
                    authority.to_account_info(),
                    diff_lamports,
                    None,
                )?;
                msg!(
                    "before_sa_authority_lamports: {}, after_authority_lamports: {}, diff_lamports: {}",
                    before_sa_authority_lamports,
                    after_authority_lamports,
                    diff_lamports
                );
            }
        }
        Ok(self.amount)
    }
}

pub struct PumpfunSellAccounts3<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,
    pub global: &'info AccountInfo<'info>,
    pub fee_recipient: &'info AccountInfo<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub bonding_curve: &'info AccountInfo<'info>,
    pub associated_bonding_curve: &'info AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub creator_vault: &'info AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
    pub event_authority: &'info AccountInfo<'info>,
    pub fee_config: &'info AccountInfo<'info>,
    pub fee_program: &'info AccountInfo<'info>,
}

const SELL_ACCOUNTS_LEN3: usize = 15;

impl<'info> PumpfunSellAccounts3<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            global,
            fee_recipient,
            mint,
            bonding_curve,
            associated_bonding_curve,
            system_program,
            creator_vault,
            token_program,
            event_authority,
            fee_config,
            fee_program,
        ]: &[AccountInfo<'info>; SELL_ACCOUNTS_LEN3] =
            array_ref![accounts, offset, SELL_ACCOUNTS_LEN3];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            global,
            fee_recipient,
            mint: InterfaceAccount::try_from(mint)?,
            bonding_curve,
            associated_bonding_curve,
            system_program: Program::try_from(system_program)?,
            creator_vault,
            token_program: Program::try_from(token_program)?,
            event_authority,
            fee_config,
            fee_program,
        })
    }

    fn cal_sol_amount_out(
        &self,
        token_amount_in: u64,
        virtual_token_reserves: u64,
        virtual_sol_reserves: u64,
    ) -> Result<u128> {
        let numerator =
            (token_amount_in as u128).checked_mul(virtual_sol_reserves as u128).unwrap();
        let denominator =
            (virtual_token_reserves as u128).checked_add(token_amount_in as u128).unwrap();

        let sol_amount_out = numerator.checked_div(denominator).unwrap();
        Ok(sol_amount_out)
    }
}

pub fn sell3<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
    payer: Option<&AccountInfo<'a>>,
) -> Result<u64> {
    msg!("Dex::Pumpfun amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + SELL_ACCOUNTS_LEN3,
        ErrorCode::InvalidAccountsLength
    );

    let mut swap_accounts = PumpfunSellAccounts3::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &pumpfun_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
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

    let bonding_curve_data = swap_accounts.bonding_curve.try_borrow_data()?.to_vec();
    let virtual_token_reserves = u64::from_le_bytes(*array_ref![&bonding_curve_data, 8, 8]);
    let virtual_sol_reserves = u64::from_le_bytes(*array_ref![&bonding_curve_data, 16, 8]);
    let mint_supply = u64::from_le_bytes(*array_ref![&bonding_curve_data, 40, 8]);
    let creator =
        Pubkey::new_from_array(*array_ref![&bonding_curve_data, 8 + 8 + 8 + 8 + 8 + 8 + 1, 32]);

    let sol_amount_out =
        swap_accounts.cal_sol_amount_out(amount_in, virtual_token_reserves, virtual_sol_reserves)?
            as u64;
    let total_fee = get_fee_3(
        swap_accounts.global,
        Some(swap_accounts.fee_config),
        mint_supply,
        virtual_sol_reserves,
        virtual_token_reserves,
        sol_amount_out,
        creator,
    )?;

    let min_sol_amount_out = sol_amount_out.checked_sub(total_fee).unwrap();
    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(PUMPFUN_SELL_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes()); // token_amount_in
    data.extend_from_slice(&1u64.to_le_bytes()); // min_sol_amount_out

    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.global.key(), false),
        AccountMeta::new(swap_accounts.fee_recipient.key(), false),
        AccountMeta::new_readonly(swap_accounts.mint.key(), false),
        AccountMeta::new(swap_accounts.bonding_curve.key(), false),
        AccountMeta::new(swap_accounts.associated_bonding_curve.key(), false),
        AccountMeta::new(swap_accounts.swap_source_token.key(), false),
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.system_program.key(), false),
        AccountMeta::new(swap_accounts.creator_vault.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.event_authority.key(), false),
        AccountMeta::new_readonly(swap_accounts.dex_program_id.key(), false),
        AccountMeta::new_readonly(swap_accounts.fee_config.key(), false),
        AccountMeta::new_readonly(swap_accounts.fee_program.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.global.to_account_info(),
        swap_accounts.fee_recipient.to_account_info(),
        swap_accounts.mint.to_account_info(),
        swap_accounts.bonding_curve.to_account_info(),
        swap_accounts.associated_bonding_curve.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.system_program.to_account_info(),
        swap_accounts.creator_vault.to_account_info(),
        swap_accounts.token_program.to_account_info(),
        swap_accounts.event_authority.to_account_info(),
        swap_accounts.dex_program_id.to_account_info(),
        swap_accounts.fee_config.to_account_info(),
        swap_accounts.fee_program.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        payer.unwrap().to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };
    let dex_processor: &PumpfunSellProcessor3 =
        &PumpfunSellProcessor3 { amount: min_sol_amount_out };

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
        SELL_ACCOUNTS_LEN3,
        proxy_swap,
        owner_seeds,
    )?;
    Ok(amount_out)
}

pub fn get_fee(
    amount: u64,
    fee_basis_points: u64,
    creator_fee_basis_points: u64,
    creator: Pubkey,
) -> u64 {
    let fee: u64 = compute_fee(amount, fee_basis_points);
    if creator != Pubkey::default() && creator_fee_basis_points != 0 {
        fee.checked_add(compute_fee(amount, creator_fee_basis_points)).unwrap()
    } else {
        fee
    }
}

pub fn compute_fee(amount: u64, fee_basis_points: u64) -> u64 {
    let amount = amount as u128;
    let fee_basis_points = fee_basis_points as u128;
    let numerator = PUMPFUN_NUMERATOR as u128;

    ceil_div(amount.checked_mul(fee_basis_points).unwrap(), numerator)
}

pub fn ceil_div(a: u128, b: u128) -> u64 {
    a.checked_add(b.checked_sub(1).unwrap()).unwrap().checked_div(b).unwrap() as u64
}

/*============================= pumpfun 3 function ============================= */
pub fn get_fee_3(
    global: &AccountInfo,
    fee_config: Option<&AccountInfo>,
    mint_supply: u64,
    virtual_token_reserves: u64,
    virtual_sol_reserves: u64,
    amount: u64,
    creator: Pubkey,
) -> Result<u64> {
    let (protocol_fee_bps, creator_fee_bps) = compute_fees_bps(
        global,
        fee_config,
        mint_supply,
        virtual_sol_reserves,
        virtual_token_reserves,
    )?;
    let creator_fee =
        if creator != Pubkey::default() { compute_fee(amount, creator_fee_bps) } else { 0 };
    Ok(compute_fee(amount, protocol_fee_bps).checked_add(creator_fee).unwrap())
}

pub fn bonding_curve_market_cap(
    mint_supply: u64,
    virtual_sol_reserves: u64,
    virtual_token_reserves: u64,
) -> Result<u64> {
    if virtual_token_reserves == 0 {
        return Err(ErrorCode::CalculationError.into());
    }

    let result = (virtual_sol_reserves as u128)
        .checked_mul(mint_supply as u128)
        .ok_or(ErrorCode::CalculationError)?
        .checked_div(virtual_token_reserves as u128)
        .ok_or(ErrorCode::CalculationError)?;

    Ok(result as u64)
}

pub fn compute_fees_bps(
    global: &AccountInfo,
    fee_config: Option<&AccountInfo>,
    mint_supply: u64,
    virtual_sol_reserves: u64,
    virtual_token_reserves: u64,
) -> Result<(u64, u64)> {
    if fee_config.is_some() {
        let market_cap =
            bonding_curve_market_cap(mint_supply, virtual_sol_reserves, virtual_token_reserves)?;
        let fee_config = fee_config.unwrap();
        let fee_config_data = fee_config.try_borrow_data()?.to_vec();
        let fee_config = parse_fee_config(fee_config_data.as_slice())?;
        return Ok(calculate_fee_tier(&fee_config.fee_tiers, market_cap)?);
    }

    let global_data = global.try_borrow_data()?.to_vec();
    let protocol_fee_bps =
        u64::from_le_bytes(*array_ref![global_data, 8 + 1 + 32 + 32 + 8 + 8 + 8 + 8, 8]);
    let creator_fee_bps = u64::from_le_bytes(*array_ref![
        global_data,
        8 + 1 + 32 + 32 + 8 + 8 + 8 + 8 + 8 + 32 + 1 + 8,
        8
    ]);
    return Ok((protocol_fee_bps, creator_fee_bps));
}

pub fn calculate_fee_tier(fee_tiers: &[FeeTier], market_cap: u64) -> Result<(u64, u64)> {
    if fee_tiers.is_empty() {
        return Err(ErrorCode::InvalidAccountData.into());
    }

    let first_tier = &fee_tiers[0];

    if market_cap < first_tier.market_cap_lamports_threshold as u64 {
        return Ok((first_tier.fees.protocol_fee_bps, first_tier.fees.creator_fee_bps));
    }

    for tier in fee_tiers.iter().rev() {
        if market_cap >= tier.market_cap_lamports_threshold as u64 {
            return Ok((tier.fees.protocol_fee_bps, tier.fees.creator_fee_bps));
        }
    }

    Ok((first_tier.fees.protocol_fee_bps, first_tier.fees.creator_fee_bps))
}

pub fn parse_fees(data: &[u8], offset: &mut usize) -> Result<Fees> {
    if *offset + 24 > data.len() {
        return Err(ErrorCode::InvalidAccountData.into());
    }

    let lp_fee_bps = u64::from_le_bytes(*array_ref![data, *offset, 8]);
    *offset += 8;
    let protocol_fee_bps = u64::from_le_bytes(*array_ref![data, *offset, 8]);
    *offset += 8;
    let creator_fee_bps = u64::from_le_bytes(*array_ref![data, *offset, 8]);
    *offset += 8;

    Ok(Fees { lp_fee_bps, protocol_fee_bps, creator_fee_bps })
}

pub fn parse_fee_tier(data: &[u8], offset: &mut usize) -> Result<FeeTier> {
    if *offset + 32 > data.len() {
        return Err(ErrorCode::InvalidAccountData.into());
    }

    let market_cap_lamports_threshold = u128::from_le_bytes(*array_ref![data, *offset, 16]);
    *offset += 16;
    let fees = parse_fees(data, offset)?;

    Ok(FeeTier { market_cap_lamports_threshold, fees })
}

pub fn parse_fee_config(data: &[u8]) -> Result<FeeConfig> {
    if data.len() < 8 + 1 + 32 + 24 {
        // discriminator + bump + admin + flat_fees
        return Err(ErrorCode::InvalidAccountData.into());
    }

    let mut offset = 8; // Skip discriminator

    // Parse bump
    let bump = data[offset];
    offset += 1;

    // Parse admin
    let admin = Pubkey::new_from_array(*array_ref![data, offset, 32]);
    offset += 32;

    // Parse flat_fees
    let flat_fees = parse_fees(data, &mut offset)?;

    // Parse fee_tiers vector
    if offset + 4 > data.len() {
        return Err(ErrorCode::InvalidAccountData.into());
    }

    // Read vector length (4 bytes for u32)
    let vector_length = u32::from_le_bytes(*array_ref![data, offset, 4]);
    offset += 4;

    let mut fee_tiers = Vec::with_capacity(vector_length as usize);

    for _ in 0..vector_length {
        let fee_tier = parse_fee_tier(data, &mut offset)?;
        fee_tiers.push(fee_tier);
    }

    Ok(FeeConfig { bump, admin, flat_fees, fee_tiers })
}

pub struct PumpfunBuyAccounts2<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,
    pub global: &'info AccountInfo<'info>,
    pub fee_recipient: &'info AccountInfo<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub bonding_curve: &'info AccountInfo<'info>,
    pub associated_bonding_curve: &'info AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub wsol_program: Program<'info, Token>,
    pub token_program: Interface<'info, TokenInterface>,
    pub creator_vault: &'info AccountInfo<'info>,
    pub event_authority: &'info AccountInfo<'info>,
    pub global_volume_accumulator: &'info AccountInfo<'info>,
    pub user_volume_accumulator: &'info AccountInfo<'info>,
    pub fee_config: &'info AccountInfo<'info>,
    pub fee_program: &'info AccountInfo<'info>,
}
const BUY_ACCOUNTS_LEN2: usize = 18;

impl<'info> PumpfunBuyAccounts2<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            global,
            fee_recipient,
            mint,
            bonding_curve,
            associated_bonding_curve,
            system_program,
            wsol_program,
            token_program,
            creator_vault,
            event_authority,
            global_volume_accumulator,
            user_volume_accumulator,
            fee_config,
            fee_program,
        ]: &[AccountInfo<'info>; BUY_ACCOUNTS_LEN2] =
            array_ref![accounts, offset, BUY_ACCOUNTS_LEN2];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            global,
            fee_recipient,
            mint: InterfaceAccount::try_from(mint)?,
            bonding_curve,
            associated_bonding_curve,
            system_program: Program::try_from(system_program)?,
            wsol_program: Program::try_from(wsol_program)?,
            token_program: Interface::try_from(token_program)?,
            creator_vault,
            event_authority,
            global_volume_accumulator,
            user_volume_accumulator,
            fee_config,
            fee_program,
        })
    }

    fn cal_token_amount_out(
        &self,
        amount_in: u64,
        virtual_token_reserves: u64,
        virtual_sol_reserves: u64,
    ) -> Result<u128> {
        let amount_out = (amount_in as u128)
            .checked_mul(virtual_token_reserves as u128)
            .unwrap()
            .checked_div((virtual_sol_reserves as u128).checked_add(amount_in as u128).unwrap())
            .unwrap();
        Ok(amount_out as u128)
    }
}

pub fn buy2<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
    payer: Option<&AccountInfo<'a>>,
) -> Result<u64> {
    msg!("Dex::Pumpfun amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + BUY_ACCOUNTS_LEN2,
        ErrorCode::InvalidAccountsLength
    );
    let mut swap_accounts = PumpfunBuyAccounts2::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &pumpfun_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
    swap_accounts.bonding_curve.key().log();

    pumpfun_before_check(
        swap_accounts.swap_authority_pubkey,
        &swap_accounts.swap_source_token,
        swap_accounts.swap_destination_token.key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let bonding_curve_data = swap_accounts.bonding_curve.try_borrow_data()?.to_vec();
    let virtual_token_reserves = u64::from_le_bytes(*array_ref![&bonding_curve_data, 8, 8]);
    let virtual_sol_reserves = u64::from_le_bytes(*array_ref![&bonding_curve_data, 16, 8]);
    let mint_supply = u64::from_le_bytes(*array_ref![&bonding_curve_data, 40, 8]);

    let creator =
        Pubkey::new_from_array(*array_ref![&bonding_curve_data, 8 + 8 + 8 + 8 + 8 + 8 + 1, 32]);

    let total_fee = get_fee_3(
        swap_accounts.global,
        Some(swap_accounts.fee_config),
        mint_supply,
        virtual_sol_reserves,
        virtual_token_reserves,
        amount_in,
        creator,
    )?;

    let real_amount_in = amount_in.saturating_sub(total_fee);
    let amount_out = swap_accounts
        .cal_token_amount_out(
            real_amount_in.checked_sub(1).unwrap(),
            virtual_token_reserves,
            virtual_sol_reserves,
        )
        .unwrap() as u64;

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(PUMPFUN_BUY_SELECTOR);
    data.extend_from_slice(&amount_out.to_le_bytes()); // token_amount_out
    data.extend_from_slice(&amount_in.to_le_bytes()); // max_amount_cost

    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.global.key(), false),
        AccountMeta::new(swap_accounts.fee_recipient.key(), false),
        AccountMeta::new_readonly(swap_accounts.mint.key(), false),
        AccountMeta::new(swap_accounts.bonding_curve.key(), false),
        AccountMeta::new(swap_accounts.associated_bonding_curve.key(), false),
        AccountMeta::new(swap_accounts.swap_destination_token.key(), false),
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.system_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
        AccountMeta::new(swap_accounts.creator_vault.key(), false),
        AccountMeta::new_readonly(swap_accounts.event_authority.key(), false),
        AccountMeta::new_readonly(swap_accounts.dex_program_id.key(), false),
        AccountMeta::new(swap_accounts.global_volume_accumulator.key(), false),
        AccountMeta::new(swap_accounts.user_volume_accumulator.key(), false),
        AccountMeta::new_readonly(swap_accounts.fee_config.key(), false),
        AccountMeta::new_readonly(swap_accounts.fee_program.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.global.to_account_info(),
        swap_accounts.fee_recipient.to_account_info(),
        swap_accounts.mint.to_account_info(),
        swap_accounts.bonding_curve.to_account_info(),
        swap_accounts.associated_bonding_curve.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.system_program.to_account_info(),
        swap_accounts.wsol_program.to_account_info(),
        swap_accounts.token_program.to_account_info(),
        swap_accounts.creator_vault.to_account_info(),
        swap_accounts.event_authority.to_account_info(),
        swap_accounts.dex_program_id.to_account_info(),
        swap_accounts.global_volume_accumulator.to_account_info(),
        swap_accounts.user_volume_accumulator.to_account_info(),
        swap_accounts.fee_config.to_account_info(),
        swap_accounts.fee_program.to_account_info(),
        payer.unwrap().to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

    let dex_processor = &PumpfunBuyProcessor3;
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
        BUY_ACCOUNTS_LEN2,
        proxy_swap,
        None,
    )?;
    Ok(amount_out)
}

pub struct PumpfunSellAccounts2<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,
    pub global: &'info AccountInfo<'info>,
    pub fee_recipient: &'info AccountInfo<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub bonding_curve: &'info AccountInfo<'info>,
    pub associated_bonding_curve: &'info AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub creator_vault: &'info AccountInfo<'info>,
    pub wsol_program: Program<'info, Token>,
    pub token_program: Interface<'info, TokenInterface>,
    pub event_authority: &'info AccountInfo<'info>,
    pub fee_config: &'info AccountInfo<'info>,
    pub fee_program: &'info AccountInfo<'info>,
}

const SELL_ACCOUNTS_LEN2: usize = 16;

impl<'info> PumpfunSellAccounts2<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            global,
            fee_recipient,
            mint,
            bonding_curve,
            associated_bonding_curve,
            system_program,
            creator_vault,
            wsol_program,
            token_program,
            event_authority,
            fee_config,
            fee_program,
        ]: &[AccountInfo<'info>; SELL_ACCOUNTS_LEN2] =
            array_ref![accounts, offset, SELL_ACCOUNTS_LEN2];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            global,
            fee_recipient,
            mint: InterfaceAccount::try_from(mint)?,
            bonding_curve,
            associated_bonding_curve,
            system_program: Program::try_from(system_program)?,
            creator_vault,
            wsol_program: Program::try_from(wsol_program)?,
            token_program: Interface::try_from(token_program)?,
            event_authority,
            fee_config,
            fee_program,
        })
    }

    fn cal_sol_amount_out(
        &self,
        token_amount_in: u64,
        virtual_token_reserves: u64,
        virtual_sol_reserves: u64,
    ) -> Result<u128> {
        let numerator =
            (token_amount_in as u128).checked_mul(virtual_sol_reserves as u128).unwrap();
        let denominator =
            (virtual_token_reserves as u128).checked_add(token_amount_in as u128).unwrap();

        let sol_amount_out = numerator.checked_div(denominator).unwrap();
        Ok(sol_amount_out)
    }
}

pub fn sell2<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
    payer: Option<&AccountInfo<'a>>,
) -> Result<u64> {
    msg!("Dex::Pumpfun amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + SELL_ACCOUNTS_LEN2,
        ErrorCode::InvalidAccountsLength
    );

    let mut swap_accounts = PumpfunSellAccounts2::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &pumpfun_program::id() {
        return Err(ErrorCode::InvalidProgramId.into());
    }
    // log pool address
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

    let bonding_curve_data = swap_accounts.bonding_curve.try_borrow_data()?.to_vec();
    let virtual_token_reserves = u64::from_le_bytes(*array_ref![&bonding_curve_data, 8, 8]);
    let virtual_sol_reserves = u64::from_le_bytes(*array_ref![&bonding_curve_data, 16, 8]);
    let mint_supply = u64::from_le_bytes(*array_ref![&bonding_curve_data, 40, 8]);
    let creator =
        Pubkey::new_from_array(*array_ref![&bonding_curve_data, 8 + 8 + 8 + 8 + 8 + 8 + 1, 32]);

    let sol_amount_out =
        swap_accounts.cal_sol_amount_out(amount_in, virtual_token_reserves, virtual_sol_reserves)?
            as u64;
    let total_fee = get_fee_3(
        swap_accounts.global,
        Some(swap_accounts.fee_config),
        mint_supply,
        virtual_sol_reserves,
        virtual_token_reserves,
        sol_amount_out,
        creator,
    )?;

    let min_sol_amount_out = sol_amount_out.checked_sub(total_fee).unwrap();
    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(PUMPFUN_SELL_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes()); // token_amount_in
    data.extend_from_slice(&1u64.to_le_bytes()); // min_sol_amount_out

    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.global.key(), false),
        AccountMeta::new(swap_accounts.fee_recipient.key(), false),
        AccountMeta::new_readonly(swap_accounts.mint.key(), false),
        AccountMeta::new(swap_accounts.bonding_curve.key(), false),
        AccountMeta::new(swap_accounts.associated_bonding_curve.key(), false),
        AccountMeta::new(swap_accounts.swap_source_token.key(), false),
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.system_program.key(), false),
        AccountMeta::new(swap_accounts.creator_vault.key(), false),
        AccountMeta::new_readonly(swap_accounts.token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.event_authority.key(), false),
        AccountMeta::new_readonly(swap_accounts.dex_program_id.key(), false),
        AccountMeta::new_readonly(swap_accounts.fee_config.key(), false),
        AccountMeta::new_readonly(swap_accounts.fee_program.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.global.to_account_info(),
        swap_accounts.fee_recipient.to_account_info(),
        swap_accounts.mint.to_account_info(),
        swap_accounts.bonding_curve.to_account_info(),
        swap_accounts.associated_bonding_curve.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.system_program.to_account_info(),
        swap_accounts.creator_vault.to_account_info(),
        swap_accounts.wsol_program.to_account_info(),
        swap_accounts.token_program.to_account_info(),
        swap_accounts.event_authority.to_account_info(),
        swap_accounts.dex_program_id.to_account_info(),
        swap_accounts.fee_config.to_account_info(),
        swap_accounts.fee_program.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        payer.unwrap().to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };
    let dex_processor: &PumpfunSellProcessor3 =
        &PumpfunSellProcessor3 { amount: min_sol_amount_out };

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
        SELL_ACCOUNTS_LEN2,
        proxy_swap,
        owner_seeds,
    )?;
    Ok(amount_out)
}

/*============================= pumpfun abort function ============================= */
pub fn buy<'a>(
    _remaining_accounts: &'a [AccountInfo<'a>],
    _amount_in: u64,
    _offset: &mut usize,
    _hop_accounts: &mut HopAccounts,
    _hop: usize,
    _proxy_swap: bool,
    _owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::Pumpfun ABORT");
    require!(true == false, ErrorCode::AdapterAbort);
    Ok(0)
}

pub fn sell<'a>(
    _remaining_accounts: &'a [AccountInfo<'a>],
    _amount_in: u64,
    _offset: &mut usize,
    _hop_accounts: &mut HopAccounts,
    _hop: usize,
    _proxy_swap: bool,
    _owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::Pumpfun ABORT");
    require!(true == false, ErrorCode::AdapterAbort);
    Ok(0)
}
