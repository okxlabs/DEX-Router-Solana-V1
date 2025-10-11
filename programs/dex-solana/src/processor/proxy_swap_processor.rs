use crate::constants::*;
use crate::error::ErrorCode;
use crate::processor::common_processor::CommonSwapProcessor;
use crate::utils::*;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::program_pack::Pack;
use anchor_spl::token::spl_token::state::Account as SplTokenAccount;
use anchor_spl::token_2022::spl_token_2022::state::Account as SplToken2022Account;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

pub struct ProxySwapProcessor;

fn unpack_token_owner_and_mint(account_info: &AccountInfo) -> Result<(Pubkey, Pubkey)> {
    if *account_info.owner == anchor_spl::token::Token::id() {
        let acc = SplTokenAccount::unpack(&account_info.try_borrow_data()?)?;
        Ok((acc.owner, acc.mint))
    } else if *account_info.owner == anchor_spl::token_2022::Token2022::id() {
        let acc = SplToken2022Account::unpack_from_slice(&account_info.try_borrow_data()?)?;
        Ok((acc.owner, acc.mint))
    } else {
        Err(ErrorCode::InvalidTokenAccount.into())
    }
}

impl ProxySwapProcessor {
    pub fn get_swap_accounts<'info>(
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

    /// Proxy handle before swap
    pub fn proxy_handle_before<'info>(
        &self,
        payer: &AccountInfo<'info>,
        source_token_account: &InterfaceAccount<'info, TokenAccount>,
        source_token_sa: &Option<UncheckedAccount<'info>>,
        source_mint: &InterfaceAccount<'info, Mint>,
        source_token_program: &Option<Interface<'info, TokenInterface>>,
        amount: u64,
        owner_seeds: Option<&[&[&[u8]]]>,
    ) -> Result<()> {
        if source_token_sa.is_none() || source_token_program.is_none() {
            return Ok(());
        }
        let source_token_program = source_token_program.as_ref().unwrap();
        let source_token_sa_info = source_token_sa.as_ref().unwrap().to_account_info();
        let (acc_owner, acc_mint) = unpack_token_owner_and_mint(&source_token_sa_info)?;
        require_keys_eq!(acc_owner, authority_pda::ID, ErrorCode::InvalidSourceTokenSa);
        require_keys_eq!(acc_mint, source_mint.key(), ErrorCode::InvalidSourceTokenSaMint);

        transfer_token(
            payer.to_account_info(),
            source_token_account.to_account_info(),
            source_token_sa_info,
            source_mint.to_account_info(),
            source_token_program.to_account_info(),
            amount,
            source_mint.decimals,
            owner_seeds,
        )?;
        Ok(())
    }

    /// Proxy handle after swap
    pub fn proxy_handle_after<'info>(
        &self,
        sa_authority: &Option<UncheckedAccount<'info>>,
        destination_token_account: &InterfaceAccount<'info, TokenAccount>,
        destination_mint: &InterfaceAccount<'info, Mint>,
        destination_token_sa: &Option<UncheckedAccount<'info>>,
        destination_token_program: &Option<Interface<'info, TokenInterface>>,
        amount_out: u64,
        owner_seeds: Option<&[&[&[u8]]]>,
    ) -> Result<()> {
        if sa_authority.is_none()
            || destination_token_sa.is_none()
            || destination_token_program.is_none()
        {
            return Ok(());
        }

        let sa_authority = sa_authority.as_ref().unwrap();
        let destination_token_sa_info = destination_token_sa.as_ref().unwrap().to_account_info();
        let destination_token_program = destination_token_program.as_ref().unwrap();
        let (acc_owner, acc_mint) = unpack_token_owner_and_mint(&destination_token_sa_info)?;
        require_keys_eq!(
            acc_mint,
            destination_mint.key(),
            ErrorCode::InvalidDestinationTokenSaMint
        );
        require_keys_eq!(acc_owner, authority_pda::ID, ErrorCode::InvalidSourceTokenSa);

        transfer_token(
            sa_authority.to_account_info(),
            destination_token_sa_info,
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
    ) -> Result<(InterfaceAccount<'info, TokenAccount>, InterfaceAccount<'info, TokenAccount>)>
    {
        self.get_swap_accounts(
            payer,
            source_token_account,
            destination_token_account,
            source_mint,
            destination_mint,
            sa_authority,
            source_token_sa,
            destination_token_sa,
            source_token_program,
            destination_token_program,
            associated_token_program,
            system_program,
        )
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
        self.proxy_handle_before(
            owner,
            source_token_account,
            source_token_sa,
            source_mint,
            source_token_program,
            amount_in,
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
        // Proxy handle after swap
        self.proxy_handle_after(
            sa_authority,
            destination_token_account,
            destination_mint,
            destination_token_sa,
            destination_token_program,
            amount_out,
            owner_seeds,
        )?;
        Ok(())
    }
}
