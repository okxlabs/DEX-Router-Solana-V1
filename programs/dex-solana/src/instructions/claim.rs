use crate::error::ErrorCode;
use crate::utils::{
    associate_convert_token_account, create_ata_if_needed, is_ata, is_token_account_initialized,
    log_claim_info_after, log_claim_info_before, transfer_sol, transfer_token,
};
use crate::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
pub struct ClaimAccounts<'info> {
    #[account(
        mut,
        address = claim_authority::id() @ ErrorCode::InvalidSigner
    )]
    pub signer: Signer<'info>,

    /// CHECK: receiver
    #[account(mut)]
    pub receiver: AccountInfo<'info>,

    /// CHECK: source token account
    #[account(
        mut,
        token::mint = token_mint,
        token::authority = sa_authority,
    )]
    pub source_token_account: Option<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: destination token account
    #[account(mut)]
    pub destination_token_account: Option<AccountInfo<'info>>,

    /// CHECK: sa authority
    #[account(
        mut,
        address = authority_pda::id() @ ErrorCode::InvalidAuthorityPda
    )]
    pub sa_authority: AccountInfo<'info>,

    /// CHECK: token mint
    pub token_mint: Option<InterfaceAccount<'info, Mint>>,

    pub token_program: Option<Interface<'info, TokenInterface>>,

    pub system_program: Program<'info, System>,

    pub associated_token_program: Option<Program<'info, AssociatedToken>>,
}

pub fn claim_handler<'a>(ctx: Context<'_, '_, 'a, 'a, ClaimAccounts<'a>>) -> Result<()> {
    let destination_account = ctx.accounts.destination_token_account.clone();

    if destination_account.is_some()
        && !is_token_account_initialized(&destination_account.as_ref().unwrap())
    {
        require!(ctx.accounts.token_mint.is_some(), ErrorCode::InvalidTokenMint);
        require!(ctx.accounts.token_program.is_some(), ErrorCode::InvalidTokenProgram);
        require!(
            ctx.accounts.associated_token_program.is_some(),
            ErrorCode::InvalidAssociatedTokenProgram
        );

        create_ata_if_needed(
            &ctx.accounts.receiver,
            &ctx.accounts.signer,
            &destination_account.as_ref().unwrap(),
            &ctx.accounts.token_mint.as_ref().unwrap().to_account_info(),
            &ctx.accounts.token_program.as_ref().unwrap().to_account_info(),
            &ctx.accounts.associated_token_program.as_ref().unwrap().to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
        )?;
    }

    // Handle SOL claim
    if destination_account.is_some()
        && is_ata(&ctx.accounts.destination_token_account.as_ref().unwrap())
    {
        handle_token_claim(&ctx)?;
    } else {
        handle_sol_claim(&ctx)?;
    }

    Ok(())
}

/// Handle SOL claim logic
fn handle_sol_claim<'a>(ctx: &Context<'_, '_, 'a, 'a, ClaimAccounts<'a>>) -> Result<()> {
    require!(
        ctx.accounts.sa_authority.lamports() > MIN_SOL_ACCOUNT_RENT,
        ErrorCode::InsufficientFunds
    );

    let amount = ctx
        .accounts
        .sa_authority
        .lamports()
        .checked_sub(MIN_SOL_ACCOUNT_RENT)
        .ok_or(ErrorCode::InsufficientFunds)?;

    // Record balances before transfer
    let before_authority_balance = ctx.accounts.sa_authority.lamports();
    let before_receiver_balance = ctx.accounts.receiver.lamports();

    log_claim_info_before(before_authority_balance, before_receiver_balance, amount);

    // Execute SOL transfer
    transfer_sol(
        ctx.accounts.sa_authority.to_account_info(),
        ctx.accounts.receiver.to_account_info(),
        amount,
        Some(SA_AUTHORITY_SEED),
    )?;

    let after_authority_balance = ctx.accounts.sa_authority.lamports();
    let after_receiver_balance = ctx.accounts.receiver.lamports();

    let authority_sol_change = before_authority_balance
        .checked_sub(after_authority_balance)
        .ok_or(ErrorCode::CalculationError)?;
    let receiver_sol_change = after_receiver_balance
        .checked_sub(before_receiver_balance)
        .ok_or(ErrorCode::CalculationError)?;

    log_claim_info_after(
        after_authority_balance,
        after_receiver_balance,
        authority_sol_change,
        receiver_sol_change,
    );

    Ok(())
}

/// Handle token claim logic
fn handle_token_claim<'a>(ctx: &Context<'_, '_, 'a, 'a, ClaimAccounts<'a>>) -> Result<()> {
    // Validate required accounts
    require!(ctx.accounts.token_mint.is_some(), ErrorCode::InvalidTokenMint);
    require!(ctx.accounts.source_token_account.is_some(), ErrorCode::InvalidSourceTokenAccount);
    require!(ctx.accounts.token_program.is_some(), ErrorCode::InvalidTokenProgram);

    // Extract accounts
    let mut source_token_account = ctx.accounts.source_token_account.as_ref().unwrap().clone();
    let mut destination_token_account =
        associate_convert_token_account(&ctx.accounts.destination_token_account.as_ref().unwrap())
            .map_err(|_| ErrorCode::InvalidTokenAccount)?;
    let token_mint = ctx.accounts.token_mint.as_ref().unwrap();
    let token_program = ctx.accounts.token_program.as_ref().unwrap();

    require!(
        destination_token_account.owner == ctx.accounts.receiver.key(),
        ErrorCode::InvalidDestinationTokenAccount
    );
    require!(source_token_account.amount > 0, ErrorCode::InsufficientFunds);
    require!(
        source_token_account.mint == destination_token_account.mint,
        ErrorCode::InvalidTokenMint
    );

    let amount = source_token_account.amount;

    // Record balances before transfer
    let before_source_balance = source_token_account.amount;
    let before_destination_balance = destination_token_account.amount;

    log_claim_info_before(before_source_balance, before_destination_balance, amount);

    transfer_token(
        ctx.accounts.sa_authority.to_account_info(),
        source_token_account.to_account_info(),
        destination_token_account.to_account_info(),
        token_mint.to_account_info(),
        token_program.to_account_info(),
        amount,
        token_mint.decimals,
        Some(SA_AUTHORITY_SEED),
    )?;

    source_token_account.reload()?;
    destination_token_account.reload()?;

    let after_source_balance = source_token_account.amount;
    let after_destination_balance = destination_token_account.amount;

    let source_token_change = before_source_balance
        .checked_sub(after_source_balance)
        .ok_or(ErrorCode::CalculationError)?;
    let destination_token_change = after_destination_balance
        .checked_sub(before_destination_balance)
        .ok_or(ErrorCode::CalculationError)?;

    // Log transfer results
    log_claim_info_after(
        after_source_balance,
        after_destination_balance,
        source_token_change,
        destination_token_change,
    );

    Ok(())
}
