use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

pub trait PlatformFeeV3Processor<'info> {
    fn get_swap_accounts(
        &self,
        _payer: &AccountInfo<'info>,
        _source_token_account: &mut InterfaceAccount<'info, TokenAccount>,
        _destination_token_account: &mut InterfaceAccount<'info, TokenAccount>,
        _source_mint: &InterfaceAccount<'info, Mint>,
        _destination_mint: &InterfaceAccount<'info, Mint>,
        _sa_authority: &Option<UncheckedAccount<'info>>,
        _source_token_sa: &mut Option<UncheckedAccount<'info>>,
        _destination_token_sa: &mut Option<UncheckedAccount<'info>>,
        _source_token_program: &Option<Interface<'info, TokenInterface>>,
        _destination_token_program: &Option<Interface<'info, TokenInterface>>,
        _associated_token_program: &Option<Program<'info, AssociatedToken>>,
        _system_program: &Option<Program<'info, System>>,
    ) -> Result<(InterfaceAccount<'info, TokenAccount>, InterfaceAccount<'info, TokenAccount>)>
    {
        Ok((_source_token_account.clone(), _destination_token_account.clone()))
    }

    fn before_swap(
        &self,
        _payer: &AccountInfo<'info>,
        _sa_authority: &Option<UncheckedAccount<'info>>,
        _source_token_account: &mut InterfaceAccount<'info, TokenAccount>,
        _source_mint: &InterfaceAccount<'info, Mint>,
        _source_token_sa: &mut Option<UncheckedAccount<'info>>,
        _source_token_program: &Option<Interface<'info, TokenInterface>>,
        _amount_in: u64,
        // COMMISSION
        _commission_rate: u32,
        _commission_direction: bool,
        _commission_account: &Option<AccountInfo<'info>>,
        // PLATFORM FEE
        _platform_fee_rate: Option<u16>,
        _platform_fee_account: &Option<AccountInfo<'info>>,
    ) -> Result<u64> {
        Ok(_amount_in)
    }

    fn after_swap(
        &self,
        _payer: &AccountInfo<'info>,
        _sa_authority: &Option<UncheckedAccount<'info>>,
        _destination_token_account: &mut InterfaceAccount<'info, TokenAccount>,
        _destination_mint: &InterfaceAccount<'info, Mint>,
        _destination_token_sa: &mut Option<UncheckedAccount<'info>>,
        _destination_token_program: &Option<Interface<'info, TokenInterface>>,
        _expected_amount_out: u64,
        _amount_out: u64,
        // COMMISSION
        _commission_rate: u32,
        _commission_direction: bool,
        _commission_account: &Option<AccountInfo<'info>>,
        // PLATFORM FEE
        _platform_fee_rate: Option<u16>,
        _platform_fee_account: &Option<AccountInfo<'info>>,
        // TRIM
        _trim_rate: Option<u8>,
        _charge_rate: Option<u16>,
        _trim_account: Option<&AccountInfo<'info>>,
        _charge_account: Option<&AccountInfo<'info>>,
        _acc_close_flag: bool,
    ) -> Result<u64> {
        Ok(_amount_out)
    }
}
