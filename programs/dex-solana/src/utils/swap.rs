use crate::constants::*;
use crate::error::LimitOrderError;
use crate::processor::common_processor::CommonSwapProcessor;
use crate::utils::{create_sa_if_needed, transfer_token};
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

pub struct FillOrderSwapProcessor;

impl<'info> CommonSwapProcessor<'info> for FillOrderSwapProcessor {
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
    ) -> Result<(InterfaceAccount<'info, TokenAccount>, InterfaceAccount<'info, TokenAccount>)>
    {
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
        fee_rate: Option<u32>,
        fee_direction: Option<bool>,
        fee_token_account: Option<&InterfaceAccount<'info, TokenAccount>>,
    ) -> Result<u64> {
        if source_token_sa.is_none() || source_token_program.is_none() {
            return Ok(amount_in);
        }
        let source_token_sa = source_token_sa.as_ref().unwrap();
        let source_token_program = source_token_program.as_ref().unwrap();

        // Collect fee from source token
        let mut real_amount_in = amount_in;
        if fee_direction.is_some()
            && fee_direction.unwrap()
            && fee_rate.is_some()
            && fee_token_account.is_some()
        {
            let fee_rate = fee_rate.unwrap();
            let fee_token_account = fee_token_account.unwrap();

            let fee_amount = amount_in
                .checked_mul(fee_rate as u64)
                .ok_or(LimitOrderError::MathOverflow)?
                .checked_div(COMMISSION_DENOMINATOR_V2)
                .ok_or(LimitOrderError::MathOverflow)?;

            msg!(
                "fee_direction: {:?}, fee_rate: {:?}, fee_amount: {:?}",
                true,
                fee_rate,
                fee_amount
            );

            // Transfer token to fee token account
            transfer_token(
                owner.to_account_info(),
                source_token_account.to_account_info(),
                fee_token_account.to_account_info(),
                source_mint.to_account_info(),
                source_token_program.to_account_info(),
                fee_amount,
                source_mint.decimals,
                owner_seeds,
            )?;

            real_amount_in =
                amount_in.checked_sub(fee_amount).ok_or(LimitOrderError::MathOverflow)?
        };

        // Transfer token to source token sa
        transfer_token(
            owner.to_account_info(),
            source_token_account.to_account_info(),
            source_token_sa.to_account_info(),
            source_mint.to_account_info(),
            source_token_program.to_account_info(),
            real_amount_in,
            source_mint.decimals,
            owner_seeds,
        )?;
        Ok(real_amount_in)
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
        fee_rate: Option<u32>,
        fee_direction: Option<bool>,
        fee_token_account: Option<&InterfaceAccount<'info, TokenAccount>>,
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

        // Collect fee from destination token
        let mut real_amount_out = amount_out;
        if fee_direction.is_some()
            && !fee_direction.unwrap()
            && fee_rate.is_some()
            && fee_token_account.is_some()
        {
            let fee_rate = fee_rate.unwrap();
            let fee_token_account = fee_token_account.unwrap();

            let fee_amount = amount_out
                .checked_mul(fee_rate as u64)
                .ok_or(LimitOrderError::MathOverflow)?
                .checked_div(COMMISSION_DENOMINATOR_V2)
                .ok_or(LimitOrderError::MathOverflow)?;

            msg!(
                "fee_direction: {:?}, fee_rate: {:?}, fee_amount: {:?}",
                false,
                fee_rate,
                fee_amount
            );

            // Transfer token to fee token account
            transfer_token(
                sa_authority.to_account_info(),
                destination_token_sa.to_account_info(),
                fee_token_account.to_account_info(),
                destination_mint.to_account_info(),
                destination_token_program.to_account_info(),
                fee_amount,
                destination_mint.decimals,
                owner_seeds,
            )?;

            real_amount_out =
                amount_out.checked_sub(fee_amount).ok_or(LimitOrderError::MathOverflow)?
        };

        transfer_token(
            sa_authority.to_account_info(),
            destination_token_sa.to_account_info(),
            destination_token_account.to_account_info(),
            destination_mint.to_account_info(),
            destination_token_program.to_account_info(),
            real_amount_out,
            destination_mint.decimals,
            owner_seeds,
        )?;
        Ok(())
    }
}
