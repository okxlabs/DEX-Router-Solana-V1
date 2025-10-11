use crate::constants::MIN_SOL_ACCOUNT_RENT;
use crate::error::ErrorCode;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::{invoke, invoke_signed};
use anchor_lang::solana_program::system_instruction::transfer;
use anchor_spl::associated_token::{AssociatedToken, create};
use anchor_spl::token::Token;
use anchor_spl::token_2022::spl_token_2022::extension::BaseStateWithExtensions;
use anchor_spl::token_2022::spl_token_2022::{
    self,
    extension::{StateWithExtensions, transfer_fee::TransferFeeConfig},
};
use anchor_spl::token_2022::{self, Token2022};
use anchor_spl::token_2022_extensions;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

pub fn transfer_token<'a>(
    authority: AccountInfo<'a>,
    from: AccountInfo<'a>,
    to: AccountInfo<'a>,
    mint: AccountInfo<'a>,
    token_program: AccountInfo<'a>,
    amount: u64,
    mint_decimals: u8,
    signer_seeds: Option<&[&[&[u8]]]>,
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }
    if let Some(signer_seeds) = signer_seeds {
        token_2022::transfer_checked(
            CpiContext::new_with_signer(
                token_program.to_account_info(),
                token_2022::TransferChecked { from, to, authority, mint },
                signer_seeds,
            ),
            amount,
            mint_decimals,
        )
    } else {
        token_2022::transfer_checked(
            CpiContext::new(
                token_program.to_account_info(),
                token_2022::TransferChecked { from, to, authority, mint },
            ),
            amount,
            mint_decimals,
        )
    }
}

pub fn transfer_sol<'a>(
    from: AccountInfo<'a>,
    to: AccountInfo<'a>,
    lamports: u64,
    signer_seeds: Option<&[&[&[u8]]]>,
) -> Result<()> {
    if lamports == 0 {
        return Ok(());
    }
    let ix = transfer(from.key, to.key, lamports);
    if let Some(signer_seeds) = signer_seeds {
        invoke_signed(&ix, &[from, to], signer_seeds)?;
    } else {
        invoke(&ix, &[from, to])?;
    }
    Ok(())
}

/// Transfer SOL ensuring the recipient has rent-exempt balance
/// If the final balance would be below rent exemption, transfers enough to reach it
/// Returns the actual amount transferred
pub fn transfer_sol_with_rent_exemption<'a>(
    from: &AccountInfo<'a>,
    to: &AccountInfo<'a>,
    requested_amount: u64,
    from_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    // Skip if transferring to self or amount is 0
    if from.key() == to.key() || requested_amount == 0 {
        return Ok(0);
    }

    // Calculate actual transfer amount to ensure receiver rent exemption
    let receiver_current_balance = to.lamports();
    let receiver_final_balance = receiver_current_balance
        .checked_add(requested_amount)
        .ok_or(ErrorCode::CalculationError)?;

    // Determine actual transfer amount
    let actual_transfer_amount = if receiver_final_balance < MIN_SOL_ACCOUNT_RENT {
        // Need to top up to minimum rent
        MIN_SOL_ACCOUNT_RENT
            .checked_sub(receiver_current_balance)
            .ok_or(ErrorCode::CalculationError)?
    } else {
        // Receiver will have enough, use original amount
        requested_amount
    };

    // Check if sender has enough balance
    let sender_balance = from.lamports();
    if sender_balance < actual_transfer_amount {
        msg!("Insufficient sender balance: {} < {}", sender_balance, actual_transfer_amount);
        return Err(ErrorCode::InsufficientBalance.into());
    }

    // Perform transfer using existing transfer_sol function
    transfer_sol(from.to_account_info(), to.to_account_info(), actual_transfer_amount, from_seeds)?;

    msg!(
        "SOL transferred with rent exemption: {} lamports (requested: {}) to {}",
        actual_transfer_amount,
        requested_amount,
        to.key()
    );

    Ok(actual_transfer_amount)
}

pub fn sync_wsol_account<'a>(
    wsol_account: AccountInfo<'a>,
    token_program: AccountInfo<'a>,
    signer_seeds: Option<&[&[&[u8]]]>,
) -> Result<()> {
    if let Some(signer_seeds) = signer_seeds {
        token_2022::sync_native(CpiContext::new_with_signer(
            token_program.to_account_info(),
            token_2022::SyncNative { account: wsol_account.to_account_info() },
            signer_seeds,
        ))
    } else {
        token_2022::sync_native(CpiContext::new(
            token_program.to_account_info(),
            token_2022::SyncNative { account: wsol_account.to_account_info() },
        ))
    }
}

pub fn close_token_account<'a>(
    token_account: AccountInfo<'a>,
    destination: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    token_program: AccountInfo<'a>,
    signer_seeds: Option<&[&[&[u8]]]>,
) -> Result<()> {
    if token_account.get_lamports() == 0 {
        return Ok(());
    }
    if let Some(signer_seeds) = signer_seeds {
        token_2022::close_account(CpiContext::new_with_signer(
            token_program.to_account_info(),
            token_2022::CloseAccount { account: token_account, destination, authority },
            signer_seeds,
        ))
    } else {
        token_2022::close_account(CpiContext::new(
            token_program.to_account_info(),
            token_2022::CloseAccount { account: token_account, destination, authority },
        ))
    }
}

pub fn create_ata_if_needed<'a>(
    authority: &AccountInfo<'a>,
    payer: &AccountInfo<'a>,
    token_account: &AccountInfo<'a>,
    token_mint: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
    associated_token_program: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
) -> Result<()> {
    if is_token_account_initialized(token_account) {
        return Ok(());
    }
    create(CpiContext::new(
        associated_token_program.to_account_info(),
        anchor_spl::associated_token::Create {
            payer: payer.to_account_info(),
            associated_token: token_account.to_account_info(),
            authority: authority.to_account_info(),
            mint: token_mint.to_account_info(),
            system_program: system_program.to_account_info(),
            token_program: token_program.to_account_info(),
        },
    ))?;
    Ok(())
}

pub fn create_sa_if_needed<'info>(
    payer: &AccountInfo<'info>,
    mint: &InterfaceAccount<'info, Mint>,
    sa_authority: &Option<UncheckedAccount<'info>>,
    token_sa: &mut Option<UncheckedAccount<'info>>,
    token_program: &Option<Interface<'info, TokenInterface>>,
    associated_token_program: &Option<Program<'info, AssociatedToken>>,
    system_program: &Option<Program<'info, System>>,
) -> Result<Option<InterfaceAccount<'info, TokenAccount>>> {
    if sa_authority.is_none()
        || token_sa.is_none()
        || token_program.is_none()
        || associated_token_program.is_none()
        || system_program.is_none()
    {
        return Ok(None);
    }
    let sa_authority = sa_authority.as_ref().unwrap();
    let token_sa = token_sa.as_ref().unwrap();
    let associated_token_program = associated_token_program.as_ref().unwrap();
    let system_program = system_program.as_ref().unwrap();
    let token_program = token_program.as_ref().unwrap();

    if !is_token_account_initialized(token_sa) {
        create(CpiContext::new(
            associated_token_program.to_account_info(),
            anchor_spl::associated_token::Create {
                payer: payer.to_account_info(),
                associated_token: token_sa.to_account_info(),
                authority: sa_authority.to_account_info(),
                mint: mint.to_account_info(),
                system_program: system_program.to_account_info(),
                token_program: token_program.to_account_info(),
            },
        ))?;
    }
    let token_sa_box = Box::leak(Box::new(token_sa.clone()));
    Ok(Some(InterfaceAccount::<TokenAccount>::try_from(token_sa_box)?))
}

/// Check if the token account is initialized
pub fn is_token_account_initialized(account: &AccountInfo) -> bool {
    // Check if the account has been rented (has allocated space) or is empty
    if account.lamports() == 0 || account.data_is_empty() {
        return false;
    }
    // Check if the account owner is the Token program
    if *account.owner != Token::id() && *account.owner != Token2022::id() {
        return false;
    }
    true
}

/// Calculate the fee for input amount
pub fn get_transfer_fee(mint_info: &AccountInfo, pre_fee_amount: u64) -> Result<u64> {
    if *mint_info.owner == Token::id() {
        return Ok(0);
    }
    let mint_data = mint_info.try_borrow_data()?;
    let mint = StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&mint_data)?;

    let fee = if let Ok(transfer_fee_config) = mint.get_extension::<TransferFeeConfig>() {
        transfer_fee_config.calculate_epoch_fee(Clock::get()?.epoch, pre_fee_amount).unwrap()
    } else {
        0
    };
    Ok(fee)
}

pub fn harvest_withheld_tokens_to_mint<'a>(
    token_program: AccountInfo<'a>,
    token_mint: AccountInfo<'a>,
    token_account: AccountInfo<'a>,
    signer_seeds: Option<&[&[&[u8]]]>,
) -> Result<()> {
    if *token_mint.owner == Token::id() || token_program.key() == Token::id() {
        return Ok(());
    }
    if let Some(signer_seeds) = signer_seeds {
        token_2022_extensions::transfer_fee::harvest_withheld_tokens_to_mint(
            CpiContext::new_with_signer(
                token_program.to_account_info(),
                token_2022_extensions::transfer_fee::HarvestWithheldTokensToMint {
                    token_program_id: token_program,
                    mint: token_mint,
                },
                signer_seeds,
            ),
            vec![token_account.to_account_info()],
        )?;
    } else {
        token_2022_extensions::transfer_fee::harvest_withheld_tokens_to_mint(
            CpiContext::new(
                token_program.to_account_info(),
                token_2022_extensions::transfer_fee::HarvestWithheldTokensToMint {
                    token_program_id: token_program.to_account_info(),
                    mint: token_mint.to_account_info(),
                },
            ),
            vec![token_account.to_account_info()],
        )?;
    }
    Ok(())
}

pub fn associate_convert_token_account<'info>(
    token_account: &AccountInfo<'info>,
) -> Result<InterfaceAccount<'info, TokenAccount>> {
    let account_box = Box::leak(Box::new(token_account.as_ref().to_account_info()));
    InterfaceAccount::<TokenAccount>::try_from(account_box)
        .map_err(|_| ErrorCode::InvalidTokenAccount.into())
}

pub fn is_ata(account: &AccountInfo) -> bool {
    account.as_ref().owner == &crate::token_program::ID
        || account.as_ref().owner == &crate::token_2022_program::ID
}

pub fn is_system_account(account: &AccountInfo) -> bool {
    account.as_ref().owner == &crate::system_program::ID
}
