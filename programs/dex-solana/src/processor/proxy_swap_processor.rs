use crate::processor::common_processor::CommonSwapProcessor;
use crate::utils::*;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

pub struct ProxySwapProcessor;

impl<'info> CommonSwapProcessor<'info> for ProxySwapProcessor {
    fn get_swap_accounts(
        &self,
        payer: &AccountInfo<'info>,
        source_token_account: &mut InterfaceAccount<'info, TokenAccount>,
        destination_token_account: &mut InterfaceAccount<'info, TokenAccount>,
        source_mint: &InterfaceAccount<'info, Mint>,
        destination_mint: &InterfaceAccount<'info, Mint>,
        sa_authority: &Option<UncheckedAccount<'info>>,
        source_token_sa: &mut Option<UncheckedAccount<'info>>,
        destination_token_sa: &mut Option<UncheckedAccount<'info>>,
        source_token_program: &Option<Interface<'info, TokenInterface>>,
        destination_token_program: &Option<Interface<'info, TokenInterface>>,
        associated_token_program: &Option<Program<'info, AssociatedToken>>,
        system_program: &Option<Program<'info, System>>,
    ) -> Result<(
        InterfaceAccount<'info, TokenAccount>,
        InterfaceAccount<'info, TokenAccount>,
    )> {
        let source_account = create_sa_if_needed(
            payer,
            source_mint,
            sa_authority,
            source_token_sa,
            source_token_program,
            associated_token_program,
            system_program,
        )?
        .unwrap_or_else(|| source_token_account.clone());

        let destination_account = create_sa_if_needed(
            payer,
            destination_mint,
            sa_authority,
            destination_token_sa,
            destination_token_program,
            associated_token_program,
            system_program,
        )?
        .unwrap_or_else(|| destination_token_account.clone());

        Ok((source_account, destination_account))
    }

    fn before_swap(
        &self,
        owner: &AccountInfo<'info>,
        source_token_account: &mut InterfaceAccount<'info, TokenAccount>,
        source_mint: &InterfaceAccount<'info, Mint>,
        source_token_sa: &mut Option<UncheckedAccount<'info>>,
        source_token_program: &Option<Interface<'info, TokenInterface>>,
        amount_in: u64,
        owner_seeds: Option<&[&[&[u8]]]>,
        _fee_rate: Option<u32>,
        _fee_direction: Option<bool>,
        _fee_token_account: Option<&InterfaceAccount<'info, TokenAccount>>,
    ) -> Result<u64> {
        if source_token_sa.is_none() || source_token_program.is_none() {
            return Ok(amount_in);
        }
        let source_token_sa = source_token_sa.as_ref().unwrap();
        let source_token_program = source_token_program.as_ref().unwrap();
        transfer_token(
            owner.to_account_info(),
            source_token_account.to_account_info(),
            source_token_sa.to_account_info(),
            source_mint.to_account_info(),
            source_token_program.to_account_info(),
            amount_in,
            source_mint.decimals,
            owner_seeds,
        )?;
        Ok(amount_in)
    }

    fn after_swap(
        &self,
        sa_authority: &Option<UncheckedAccount<'info>>,
        destination_token_account: &mut InterfaceAccount<'info, TokenAccount>,
        destination_mint: &InterfaceAccount<'info, Mint>,
        destination_token_sa: &mut Option<UncheckedAccount<'info>>,
        destination_token_program: &Option<Interface<'info, TokenInterface>>,
        amount_out: u64,
        owner_seeds: Option<&[&[&[u8]]]>,
        _fee_rate: Option<u32>,
        _fee_direction: Option<bool>,
        _fee_token_account: Option<&InterfaceAccount<'info, TokenAccount>>,
    ) -> Result<()> {
        if sa_authority.is_none()
            || destination_token_sa.is_none()
            || destination_token_program.is_none()
        {
            return Ok(());
        }
        let sa_authority = sa_authority.as_ref().unwrap();
        let destination_token_sa = destination_token_sa.as_ref().unwrap();
        let destination_token_program = destination_token_program.as_ref().unwrap();
        transfer_token(
            sa_authority.to_account_info(),
            destination_token_sa.to_account_info(),
            destination_token_account.to_account_info(),
            destination_mint.to_account_info(),
            destination_token_program.to_account_info(),
            amount_out,
            destination_mint.decimals,
            owner_seeds,
        )?;
        Ok(())
    }
}
