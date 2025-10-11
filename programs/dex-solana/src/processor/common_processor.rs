use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

pub trait CommonSwapProcessor<'info> {
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
        _owner: &AccountInfo<'info>,
        _source_token_account: &mut InterfaceAccount<'info, TokenAccount>,
        _source_mint: &InterfaceAccount<'info, Mint>,
        _source_token_sa: &mut Option<UncheckedAccount<'info>>,
        _source_token_program: &Option<Interface<'info, TokenInterface>>,
        _amount_in: u64,
        _owner_seeds: Option<&[&[&[u8]]]>,
        _fee_rate: Option<u32>,
        _fee_direction: Option<bool>,
        _fee_token_account: Option<&InterfaceAccount<'info, TokenAccount>>,
    ) -> Result<u64> {
        Ok(_amount_in)
    }

    fn after_swap(
        &self,
        _sa_authority: &Option<UncheckedAccount<'info>>,
        _destination_token_account: &mut InterfaceAccount<'info, TokenAccount>,
        _destination_mint: &InterfaceAccount<'info, Mint>,
        _destination_token_sa: &mut Option<UncheckedAccount<'info>>,
        _destination_token_program: &Option<Interface<'info, TokenInterface>>,
        _amount_out: u64,
        _owner_seeds: Option<&[&[&[u8]]]>,
        _fee_rate: Option<u32>,
        _fee_direction: Option<bool>,
        _fee_token_account: Option<&InterfaceAccount<'info, TokenAccount>>,
    ) -> Result<()> {
        Ok(())
    }
}
