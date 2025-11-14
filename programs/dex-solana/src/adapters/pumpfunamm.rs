use super::common::DexProcessor;
use crate::adapters::common::{before_check, invoke_process};
use crate::error::ErrorCode;
use crate::utils::transfer_sol;
use crate::{
    BUY_EXACT_QUOTE_IN_SELECTOR, HopAccounts, PUMPFUN_SELL_SELECTOR, SOL_DIFF_LIMIT, authority_pda,
    pumpfunamm_program,
};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use arrayref::array_ref;

const ARGS_LEN: usize = 24;
pub struct PumpfunammSellAccounts3<'info> {
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
    pub fee_config: &'info AccountInfo<'info>,
    pub fee_program: &'info AccountInfo<'info>,
}
const SELL_ACCOUNTS_LEN3: usize = 21;

impl<'info> PumpfunammSellAccounts3<'info> {
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
            fee_config,
            fee_program,
        ]: &[AccountInfo<'info>; SELL_ACCOUNTS_LEN3] =
            array_ref![accounts, offset, SELL_ACCOUNTS_LEN3];

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
            fee_config,
            fee_program,
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

    fn after_invoke(
        &self,
        account_infos: &[AccountInfo],
        _hop: usize,
        _owner_seeds: Option<&[&[&[u8]]]>,
        before_sa_authority_lamports: u64,
    ) -> Result<u64> {
        let authority = account_infos.get(1).unwrap();
        let payer = account_infos.last().unwrap();
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
            Ok(diff_lamports)
        } else {
            Ok(0)
        }
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
    msg!("Dex::Pumpfunamm amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + SELL_ACCOUNTS_LEN3,
        ErrorCode::InvalidAccountsLength
    );

    let mut swap_accounts: PumpfunammSellAccounts3<'_> =
        PumpfunammSellAccounts3::parse_accounts(remaining_accounts, *offset)?;
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
        AccountMeta::new(swap_accounts.pool.key(), false),
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.global_config.key(), false),
        AccountMeta::new_readonly(swap_accounts.base_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.quote_mint.key(), false),
        AccountMeta::new(swap_accounts.swap_source_token.key(), false),
        AccountMeta::new(swap_accounts.swap_destination_token.key(), false),
        AccountMeta::new(swap_accounts.pool_base_token_account.key(), false),
        AccountMeta::new(swap_accounts.pool_quote_token_account.key(), false),
        AccountMeta::new_readonly(swap_accounts.protocol_fee_recipient.key(), false),
        AccountMeta::new(swap_accounts.protocol_fee_recipient_token_account.key(), false),
        AccountMeta::new_readonly(swap_accounts.base_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.quote_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.system_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.associated_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.event_authority.key(), false),
        AccountMeta::new_readonly(swap_accounts.dex_program_id.key(), false),
        AccountMeta::new(swap_accounts.coin_creator_vault_ata.key(), false),
        AccountMeta::new_readonly(swap_accounts.coin_creator_vault_authority.key(), false),
        AccountMeta::new_readonly(swap_accounts.fee_config.key(), false),
        AccountMeta::new_readonly(swap_accounts.fee_program.key(), false),
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
        swap_accounts.protocol_fee_recipient_token_account.to_account_info(),
        swap_accounts.base_token_program.to_account_info(),
        swap_accounts.quote_token_program.to_account_info(),
        swap_accounts.system_program.to_account_info(),
        swap_accounts.associated_token_program.to_account_info(),
        swap_accounts.event_authority.to_account_info(),
        swap_accounts.dex_program_id.to_account_info(),
        swap_accounts.coin_creator_vault_ata.to_account_info(),
        swap_accounts.coin_creator_vault_authority.to_account_info(),
        swap_accounts.fee_config.to_account_info(),
        swap_accounts.fee_program.to_account_info(),
        payer.unwrap().to_account_info(),
    ];

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

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
        SELL_ACCOUNTS_LEN3,
        proxy_swap,
        owner_seeds,
    )?;

    Ok(amount_out)
}
pub struct PumpfunammBuyAccounts3<'info> {
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
    pub fee_config: &'info AccountInfo<'info>,
    pub fee_program: &'info AccountInfo<'info>,
}
const BUY_ACCOUNTS_LEN3: usize = 23;

impl<'info> PumpfunammBuyAccounts3<'info> {
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
            fee_config,
            fee_program,
        ]: &[AccountInfo<'info>; BUY_ACCOUNTS_LEN3] =
            array_ref![accounts, offset, BUY_ACCOUNTS_LEN3];

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
            fee_config,
            fee_program,
        })
    }
}
pub struct PumpfunammBuyProcessor;
impl DexProcessor for PumpfunammBuyProcessor {}

pub fn buy3<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!("Dex::Pumpfunamm amount_in: {}, offset: {}", amount_in, offset);
    require!(
        remaining_accounts.len() >= *offset + BUY_ACCOUNTS_LEN3,
        ErrorCode::InvalidAccountsLength
    );

    let mut swap_accounts = PumpfunammBuyAccounts3::parse_accounts(remaining_accounts, *offset)?;
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

    let mut data = Vec::with_capacity(ARGS_LEN);
    data.extend_from_slice(BUY_EXACT_QUOTE_IN_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes()); // spendable_quote_in
    data.extend_from_slice(&1u64.to_le_bytes()); // min_base_amount_out

    let accounts = vec![
        AccountMeta::new(swap_accounts.pool.key(), false),
        AccountMeta::new(swap_accounts.swap_authority_pubkey.key(), true),
        AccountMeta::new_readonly(swap_accounts.global_config.key(), false),
        AccountMeta::new_readonly(swap_accounts.base_mint.key(), false), // wsol
        AccountMeta::new_readonly(swap_accounts.quote_mint.key(), false), // usdc
        AccountMeta::new(swap_accounts.swap_destination_token.key(), false), // wsol-ata
        AccountMeta::new(swap_accounts.swap_source_token.key(), false),  // usdc-ata
        AccountMeta::new(swap_accounts.pool_base_token_account.key(), false), // wsol-ata
        AccountMeta::new(swap_accounts.pool_quote_token_account.key(), false), //usdc-ata
        AccountMeta::new_readonly(swap_accounts.protocol_fee_recipient.key(), false),
        AccountMeta::new(swap_accounts.protocol_fee_recipient_token_account.key(), false),
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
        AccountMeta::new_readonly(swap_accounts.fee_config.key(), false),
        AccountMeta::new_readonly(swap_accounts.fee_program.key(), false),
    ];

    let mut account_infos = Vec::with_capacity(BUY_ACCOUNTS_LEN3);
    account_infos.push(swap_accounts.pool.to_account_info());
    account_infos.push(swap_accounts.swap_authority_pubkey.to_account_info());
    account_infos.push(swap_accounts.global_config.to_account_info());
    account_infos.push(swap_accounts.base_mint.to_account_info());
    account_infos.push(swap_accounts.quote_mint.to_account_info());
    account_infos.push(swap_accounts.swap_destination_token.to_account_info());
    account_infos.push(swap_accounts.swap_source_token.to_account_info());
    account_infos.push(swap_accounts.pool_base_token_account.to_account_info());
    account_infos.push(swap_accounts.pool_quote_token_account.to_account_info());
    account_infos.push(swap_accounts.protocol_fee_recipient.to_account_info());
    account_infos.push(swap_accounts.protocol_fee_recipient_token_account.to_account_info());
    account_infos.push(swap_accounts.base_token_program.to_account_info());
    account_infos.push(swap_accounts.quote_token_program.to_account_info());
    account_infos.push(swap_accounts.system_program.to_account_info());
    account_infos.push(swap_accounts.associated_token_program.to_account_info());
    account_infos.push(swap_accounts.event_authority.to_account_info());
    account_infos.push(swap_accounts.dex_program_id.to_account_info());
    account_infos.push(swap_accounts.coin_creator_vault_ata.to_account_info());
    account_infos.push(swap_accounts.coin_creator_vault_authority.to_account_info());
    account_infos.push(swap_accounts.global_volume_accumulator.to_account_info());
    account_infos.push(swap_accounts.user_volume_accumulator.to_account_info());
    account_infos.push(swap_accounts.fee_config.to_account_info());
    account_infos.push(swap_accounts.fee_program.to_account_info());

    let instruction =
        Instruction { program_id: swap_accounts.dex_program_id.key(), accounts, data };

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
        BUY_ACCOUNTS_LEN3,
        proxy_swap,
        owner_seeds,
    )?;

    Ok(amount_out)
}

/*============================= pumpfunamm abort function ============================= */
pub fn sell<'a>(
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
