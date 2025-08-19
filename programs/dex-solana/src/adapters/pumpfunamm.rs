use super::common::DexProcessor;
use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::utils::transfer_sol;
use crate::{
    authority_pda, pumpfunamm_program, HopAccounts, PUMPFUN_BUY_SELECTOR, PUMPFUN_SELL_SELECTOR, SOL_DIFF_LIMIT, ZERO_ADDRESS
};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use arrayref::array_ref;

const ARGS_LEN: usize = 24;

pub struct PumpfunammSellAccounts<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub pool: &'info AccountInfo<'info>,
    pub global_config: &'info AccountInfo<'info>,
    pub base_mint: Box<InterfaceAccount<'info, Mint>>,
    pub quote_mint: Box<InterfaceAccount<'info, Mint>>,
    pub pool_base_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub pool_quote_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub protocol_fee_recipient: &'info AccountInfo<'info>,
    pub protocol_fee_recipient_token_account: UncheckedAccount<'info>,
    pub base_token_program: Interface<'info, TokenInterface>,
    pub quote_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub event_authority: &'info AccountInfo<'info>,
    pub coin_creator_vault_ata: UncheckedAccount<'info>,
    pub coin_creator_vault_authority: &'info AccountInfo<'info>,
}
const SELL_ACCOUNTS_LEN: usize = 19;

impl<'info> PumpfunammSellAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            pool,
            global_config,
            base_mint,
            quote_mint,
            pool_base_token_account,
            pool_quote_token_account,
            protocol_fee_recipient,
            protocol_fee_recipient_token_account,
            base_token_program,
            quote_token_program,
            system_program,
            associated_token_program,
            event_authority,
            coin_creator_vault_ata,
            coin_creator_vault_authority,
        ]: &[AccountInfo<'info>; SELL_ACCOUNTS_LEN] = array_ref![accounts, offset, SELL_ACCOUNTS_LEN];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            pool,
            global_config,
            base_mint: Box::new(InterfaceAccount::try_from(base_mint)?),
            quote_mint: Box::new(InterfaceAccount::try_from(quote_mint)?),
            pool_base_token_account: Box::new(InterfaceAccount::try_from(pool_base_token_account)?),
            pool_quote_token_account: Box::new(InterfaceAccount::try_from(
                pool_quote_token_account,
            )?),
            protocol_fee_recipient,
            protocol_fee_recipient_token_account: UncheckedAccount::try_from(
                protocol_fee_recipient_token_account,
            ),
            base_token_program: Interface::try_from(base_token_program)?,
            quote_token_program: Interface::try_from(quote_token_program)?,
            system_program: Program::try_from(system_program)?,
            associated_token_program: Program::try_from(associated_token_program)?,
            event_authority,
            coin_creator_vault_ata: UncheckedAccount::try_from(coin_creator_vault_ata),
            coin_creator_vault_authority,
        })
    }
}

pub struct PumpfunammSellProcessor;
impl DexProcessor for PumpfunammSellProcessor {
    fn before_invoke(&self, account_infos: &[AccountInfo]) -> Result<u64> {
        let authority = account_infos.get(1).unwrap();

        if authority.key() == authority_pda::ID {
            let before_authority_lamports = authority.lamports();
            Ok(before_authority_lamports)
        } else {
            Ok(0)
        }
    }

    fn after_invoke(&self, account_infos: &[AccountInfo], _hop: usize, _owner_seeds: Option<&[&[&[u8]]]>, before_sa_authority_lamports: u64) -> Result<u64> {
        let authority = account_infos.get(1).unwrap();
        let payer = account_infos.last().unwrap();
        if authority.key() == authority_pda::ID {
            let after_authority_lamports = authority.lamports();
            let diff_lamports = before_sa_authority_lamports.saturating_sub(after_authority_lamports);
            require!(diff_lamports <= SOL_DIFF_LIMIT, ErrorCode::InvalidDiffLamports);
            if diff_lamports > 0 {
                transfer_sol(
                    payer.to_account_info(),
                    authority.to_account_info(),
                    diff_lamports,
                    None,
                )?;
                msg!("before_sa_authority_lamports: {}, after_authority_lamports: {}, diff_lamports: {}", before_sa_authority_lamports, after_authority_lamports, diff_lamports);
            }
            Ok(diff_lamports)
        } else {
            Ok(0)
        }
    }
}

pub fn sell<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
    payer: Option<&AccountInfo<'a>>,
) -> Result<u64> {
    msg!(
        "Dex::Pumpfunamm amount_in: {}, offset: {}",
        amount_in,
        offset
    );
    require!(
        remaining_accounts.len() >= *offset + SELL_ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );

    let mut swap_accounts: PumpfunammSellAccounts<'_> =
        PumpfunammSellAccounts::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &pumpfunamm_program::id() {
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

    // amount_in base_mint_amount
    // amount_out quote_mint_amount
    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(PUMPFUN_SELL_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes()); // base_amount_in
    data.extend_from_slice(&1u64.to_le_bytes()); // min_quote_amount_out

    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.pool.key(), false),
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.global_config.key(), false),
        AccountMeta::new_readonly(swap_accounts.base_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.quote_mint.key(), false),
        AccountMeta::new(swap_accounts.swap_source_token.key(), false),
        AccountMeta::new(swap_accounts.swap_destination_token.key(), false),
        AccountMeta::new(swap_accounts.pool_base_token_account.key(), false),
        AccountMeta::new(swap_accounts.pool_quote_token_account.key(), false),
        AccountMeta::new_readonly(swap_accounts.protocol_fee_recipient.key(), false),
        AccountMeta::new(
            swap_accounts.protocol_fee_recipient_token_account.key(),
            false,
        ),
        AccountMeta::new_readonly(swap_accounts.base_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.quote_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.system_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.associated_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.event_authority.key(), false),
        AccountMeta::new_readonly(swap_accounts.dex_program_id.key(), false),
        AccountMeta::new(swap_accounts.coin_creator_vault_ata.key(), false),
        AccountMeta::new_readonly(swap_accounts.coin_creator_vault_authority.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.pool.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.global_config.to_account_info(),
        swap_accounts.base_mint.to_account_info(),
        swap_accounts.quote_mint.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.pool_base_token_account.to_account_info(),
        swap_accounts.pool_quote_token_account.to_account_info(),
        swap_accounts.protocol_fee_recipient.to_account_info(),
        swap_accounts
            .protocol_fee_recipient_token_account
            .to_account_info(),
        swap_accounts.base_token_program.to_account_info(),
        swap_accounts.quote_token_program.to_account_info(),
        swap_accounts.system_program.to_account_info(),
        swap_accounts.associated_token_program.to_account_info(),
        swap_accounts.event_authority.to_account_info(),
        swap_accounts.dex_program_id.to_account_info(),
        swap_accounts.coin_creator_vault_ata.to_account_info(),
        swap_accounts.coin_creator_vault_authority.to_account_info(),
        payer.unwrap().to_account_info(),
    ];

    let instruction = Instruction {
        program_id: swap_accounts.dex_program_id.key(),
        accounts,
        data,
    };

    let dex_processor = &PumpfunammSellProcessor;
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

pub struct PumpfunammBuyAccounts2<'info> {
    pub dex_program_id: &'info AccountInfo<'info>,
    pub swap_authority_pubkey: &'info AccountInfo<'info>,
    pub swap_source_token: InterfaceAccount<'info, TokenAccount>,
    pub swap_destination_token: InterfaceAccount<'info, TokenAccount>,

    pub pool: &'info AccountInfo<'info>,
    pub global_config: &'info AccountInfo<'info>,
    pub base_mint: Box<InterfaceAccount<'info, Mint>>,
    pub quote_mint: Box<InterfaceAccount<'info, Mint>>,
    pub pool_base_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub pool_quote_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub protocol_fee_recipient: &'info AccountInfo<'info>,
    pub protocol_fee_recipient_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub base_token_program: Interface<'info, TokenInterface>,
    pub quote_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub event_authority: &'info AccountInfo<'info>,
    pub coin_creator_vault_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    pub coin_creator_vault_authority: &'info AccountInfo<'info>,
    pub global_volume_accumulator: &'info AccountInfo<'info>,
    pub user_volume_accumulator: &'info AccountInfo<'info>,
}
const BUY_ACCOUNTS_LEN2: usize = 21;

impl<'info> PumpfunammBuyAccounts2<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token,
            swap_destination_token,
            pool,
            global_config,
            base_mint,
            quote_mint,
            pool_base_token_account,
            pool_quote_token_account,
            protocol_fee_recipient,
            protocol_fee_recipient_token_account,
            base_token_program,
            quote_token_program,
            system_program,
            associated_token_program,
            event_authority,
            coin_creator_vault_ata,
            coin_creator_vault_authority,
            global_volume_accumulator,
            user_volume_accumulator,
        ]: &[AccountInfo<'info>; BUY_ACCOUNTS_LEN2] = array_ref![accounts, offset, BUY_ACCOUNTS_LEN2];

        Ok(Self {
            dex_program_id,
            swap_authority_pubkey,
            swap_source_token: InterfaceAccount::try_from(swap_source_token)?,
            swap_destination_token: InterfaceAccount::try_from(swap_destination_token)?,
            pool,
            global_config,
            base_mint: Box::new(InterfaceAccount::try_from(base_mint)?),
            quote_mint: Box::new(InterfaceAccount::try_from(quote_mint)?),
            pool_base_token_account: Box::new(InterfaceAccount::try_from(pool_base_token_account)?),
            pool_quote_token_account: Box::new(InterfaceAccount::try_from(
                pool_quote_token_account,
            )?),
            protocol_fee_recipient,
            protocol_fee_recipient_token_account: Box::new(InterfaceAccount::try_from(
                protocol_fee_recipient_token_account,
            )?),
            base_token_program: Interface::try_from(base_token_program)?,
            quote_token_program: Interface::try_from(quote_token_program)?,
            system_program: Program::try_from(system_program)?,
            associated_token_program: Program::try_from(associated_token_program)?,
            event_authority,
            coin_creator_vault_ata: Box::new(InterfaceAccount::try_from(coin_creator_vault_ata)?),
            coin_creator_vault_authority,
            global_volume_accumulator,
            user_volume_accumulator,
        })
    }

    fn cal_base_amount_out(&self, amount_in: u128) -> Result<u128> {
        let base_reserves = self.pool_base_token_account.amount;
        let quote_reserves = self.pool_quote_token_account.amount;

        if base_reserves == 0 || quote_reserves == 0 {
            return Err(ErrorCode::InvalidPool.into());
        }

        let data = self.global_config.try_borrow_data()?;
        let lp_fee_bps = u64::from_le_bytes(*array_ref![data, 40, 8]);
        let protocol_fee_bps = u64::from_le_bytes(*array_ref![data, 48, 8]);
        let creator_fee_bps = u64::from_le_bytes(*array_ref![data, 313, 8]);

        let pool_data = self.pool.try_borrow_data()?;
        let coin_creator = Pubkey::new_from_array(*array_ref![pool_data, 211, 32]);
        let effective_creator_fee_bps = if coin_creator == ZERO_ADDRESS {
            0u64
        } else {
            creator_fee_bps
        };

        let total_fee_bps = lp_fee_bps
            .checked_add(protocol_fee_bps)
            .unwrap()
            .checked_add(effective_creator_fee_bps)
            .unwrap();
        let denominator = (total_fee_bps as u128).checked_add(10000).unwrap();

        let effective_quote = (amount_in as u128)
            .checked_mul(10000)
            .unwrap()
            .checked_div(denominator)
            .unwrap();
        let numerator = (base_reserves as u128)
            .checked_mul(effective_quote)
            .unwrap();
        let denominator_effective = (quote_reserves as u128)
            .checked_add(effective_quote)
            .unwrap();
        let base_amount_out = numerator.checked_div(denominator_effective).unwrap();

        Ok(base_amount_out as u128)
    }
}

pub struct PumpfunammBuyProcessor;
impl DexProcessor for PumpfunammBuyProcessor {}

pub fn buy<'a>(
    _remaining_accounts: &'a [AccountInfo<'a>],
    _amount_in: u64,
    _offset: &mut usize,
    _hop_accounts: &mut HopAccounts,
    _hop: usize,
    _proxy_swap: bool,
    _owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::Pumpfunamm ABORT");
    require!(true == false, ErrorCode::AdapterAbort);
    Ok(0)
}

pub fn buy2<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!(
        "Dex::Pumpfunamm amount_in: {}, offset: {}",
        amount_in,
        offset
    );
    require!(
        remaining_accounts.len() >= *offset + BUY_ACCOUNTS_LEN2,
        ErrorCode::InvalidAccountsLength
    );

    let mut swap_accounts = PumpfunammBuyAccounts2::parse_accounts(remaining_accounts, *offset)?;
    if swap_accounts.dex_program_id.key != &pumpfunamm_program::id() {
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

    let amount_out =
        swap_accounts.cal_base_amount_out((amount_in as u128).saturating_sub(2))? as u64;

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(PUMPFUN_BUY_SELECTOR);
    data.extend_from_slice(&amount_out.to_le_bytes()); // base_amount_out
    data.extend_from_slice(&amount_in.to_le_bytes()); // max_quote_amount_in

    let accounts = vec![
        AccountMeta::new_readonly(swap_accounts.pool.key(), false),
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.global_config.key(), false),
        AccountMeta::new_readonly(swap_accounts.base_mint.key(), false), // wsol
        AccountMeta::new_readonly(swap_accounts.quote_mint.key(), false), // usdc
        AccountMeta::new(swap_accounts.swap_destination_token.key(), false), // wsol-ata
        AccountMeta::new(swap_accounts.swap_source_token.key(), false),  // usdc-ata
        AccountMeta::new(swap_accounts.pool_base_token_account.key(), false), // wsol-ata
        AccountMeta::new(swap_accounts.pool_quote_token_account.key(), false), //usdc-ata
        AccountMeta::new_readonly(swap_accounts.protocol_fee_recipient.key(), false),
        AccountMeta::new(
            swap_accounts.protocol_fee_recipient_token_account.key(),
            false,
        ),
        AccountMeta::new_readonly(swap_accounts.base_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.quote_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.system_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.associated_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.event_authority.key(), false),
        AccountMeta::new_readonly(swap_accounts.dex_program_id.key(), false),
        AccountMeta::new(swap_accounts.coin_creator_vault_ata.key(), false),
        AccountMeta::new_readonly(swap_accounts.coin_creator_vault_authority.key(), false),
        AccountMeta::new(swap_accounts.global_volume_accumulator.key(), false),
        AccountMeta::new(swap_accounts.user_volume_accumulator.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.pool.to_account_info(),
        swap_accounts.swap_authority_pubkey.to_account_info(),
        swap_accounts.global_config.to_account_info(),
        swap_accounts.base_mint.to_account_info(),
        swap_accounts.quote_mint.to_account_info(),
        swap_accounts.swap_destination_token.to_account_info(),
        swap_accounts.swap_source_token.to_account_info(),
        swap_accounts.pool_base_token_account.to_account_info(),
        swap_accounts.pool_quote_token_account.to_account_info(),
        swap_accounts.protocol_fee_recipient.to_account_info(),
        swap_accounts
            .protocol_fee_recipient_token_account
            .to_account_info(),
        swap_accounts.base_token_program.to_account_info(),
        swap_accounts.quote_token_program.to_account_info(),
        swap_accounts.system_program.to_account_info(),
        swap_accounts.associated_token_program.to_account_info(),
        swap_accounts.event_authority.to_account_info(),
        swap_accounts.dex_program_id.to_account_info(),
        swap_accounts.coin_creator_vault_ata.to_account_info(),
        swap_accounts.coin_creator_vault_authority.to_account_info(),
        swap_accounts.global_volume_accumulator.to_account_info(),
        swap_accounts.user_volume_accumulator.to_account_info(),
    ];

    let instruction = Instruction {
        program_id: swap_accounts.dex_program_id.key(),
        accounts,
        data,
    };

    let dex_processor = &PumpfunammBuyProcessor;
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
        owner_seeds,
    )?;

    Ok(amount_out)
}
