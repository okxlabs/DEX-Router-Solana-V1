use crate::constants::*;
use crate::error::ErrorCode;
use crate::utils::*;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(AnchorDeserialize, AnchorSerialize, Clone)]
pub struct PlatformFeeWrapUnwrapArgs {
    pub order_id: u64,
    pub amount_in: u64,
    pub commission_info: u32,   // Commission info
    pub platform_fee_rate: u16, // Platform fee rate
    pub tob: bool,              // New: distinguish TOB/TOC mode
}

#[derive(Accounts)]
pub struct PlatformFeeWrapUnwrapAccounts<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        token::mint = wsol_mint,
        token::authority = payer,
        token::token_program = token_program,
    )]
    pub payer_wsol_account: InterfaceAccount<'info, TokenAccount>,

    #[account(address = wsol_program::id())]
    pub wsol_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init_if_needed,
        payer = payer,
        token::mint = wsol_mint,
        token::authority = payer,
        token::token_program = token_program,
        seeds = [SEED_TEMP_WSOL, payer.key().as_ref()],
        bump
    )]
    pub temp_wsol_account: Option<Box<InterfaceAccount<'info, TokenAccount>>>,

    /// CHECK: commission account
    #[account(mut)]
    pub commission_account: Option<AccountInfo<'info>>,

    /// CHECK: platform fee account
    #[account(mut)]
    pub platform_fee_account: Option<AccountInfo<'info>>,

    /// CHECK: Authority PDA for TOB mode. Required when tob=true
    /// Used for signing fee transfers from authority_pda (SOL) or wsol_sa (WSOL)
    #[account(mut)]
    pub authority_pda: Option<AccountInfo<'info>>,

    /// CHECK: WSOL SA account for TOB mode. Required when tob=true and charging WSOL fees
    /// This is the authority_pda's associated token account for WSOL
    #[account(mut)]
    pub wsol_sa: Option<AccountInfo<'info>>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

/// Commission information structure with improved clarity
#[derive(Debug, Clone, Copy)]
struct CommissionInfo {
    /// Whether commission is applied to input (true) or output (false)
    commission_direction: bool,
    /// Whether this is a wrap (true) or unwrap (false) operation
    wrap_direction: bool,
    /// Commission rate in basis points
    commission_rate: u32,
}

impl CommissionInfo {
    /// Parse commission information from u32
    fn from_u32(commission_info: u32) -> Self {
        Self {
            commission_direction: (commission_info >> 31) == 1,
            wrap_direction: ((commission_info & (1 << 30)) >> 30) == 1,
            commission_rate: commission_info & ((1 << 30) - 1),
        }
    }

    // If commission direction is the same as wrap direction, charge fees as SOL
    fn is_charge_sol(&self) -> bool {
        self.commission_direction == self.wrap_direction
    }
}

pub fn platform_fee_wrap_unwrap_handler_v3<'a>(
    ctx: Context<'_, '_, 'a, 'a, PlatformFeeWrapUnwrapAccounts<'a>>,
    args: PlatformFeeWrapUnwrapArgs,
) -> Result<()> {
    let PlatformFeeWrapUnwrapArgs { order_id, amount_in, commission_info, platform_fee_rate, tob } =
        args;

    // Validate input
    require!(amount_in > 0, ErrorCode::AmountInMustBeGreaterThanZero);

    // Parse commission info
    let commission_info = CommissionInfo::from_u32(commission_info);

    // Log rate information
    log_rate_info_v3(
        commission_info.commission_rate,
        Some(platform_fee_rate),
        None,
        commission_info.commission_direction,
        false,
    );

    // Log initial state and get balances
    log_wrap_unwrap_initial_info(
        &ctx.accounts.payer,
        &ctx.accounts.payer_wsol_account,
        commission_info.wrap_direction,
        amount_in,
        order_id,
    )?;

    // Calculate fees
    let (commission_amount, platform_fee_amount) = calculate_fee_amounts(
        amount_in,
        commission_info.commission_rate,
        commission_info.commission_direction,
        Some(platform_fee_rate),
    )?;

    // Execute wrap/unwrap process
    execute_wrap_unwrap_process_v3(&ctx.accounts, commission_info.wrap_direction, amount_in)?;

    // Transfer fees if any
    if commission_amount > 0 || platform_fee_amount > 0 {
        // Validate TOB accounts
        validate_tob_accounts(
            tob,
            commission_info.is_charge_sol(),
            &ctx.accounts.authority_pda,
            &ctx.accounts.wsol_sa,
        )?;

        transfer_wrap_unwrap_fees_and_log(
            &ctx.accounts.payer.to_account_info(),
            &ctx.accounts.payer_wsol_account,
            &ctx.accounts.wsol_mint,
            &ctx.accounts.token_program,
            commission_amount,
            platform_fee_amount,
            &ctx.accounts.commission_account,
            &ctx.accounts.platform_fee_account,
            &commission_info,
            tob,                         // Pass tob flag
            &ctx.accounts.authority_pda, // Pass authority_pda
            &ctx.accounts.wsol_sa,       // Pass wsol_sa
        )?;
    }

    // Log final state and calculate changes
    log_wrap_unwrap_final_info(
        &ctx.accounts.payer,
        &mut ctx.accounts.payer_wsol_account,
        commission_info.wrap_direction,
        amount_in,
        commission_amount,
        platform_fee_amount,
    )?;

    Ok(())
}

/// Execute wrap or unwrap process based on direction
fn execute_wrap_unwrap_process_v3<'info>(
    accounts: &PlatformFeeWrapUnwrapAccounts<'info>,
    wrap_direction: bool,
    amount: u64,
) -> Result<()> {
    if wrap_direction {
        wrap_process_v3(
            accounts.payer.clone(),
            accounts.payer_wsol_account.clone(),
            amount,
            accounts.token_program.clone(),
        )
    } else {
        let temp_wsol_account = accounts
            .temp_wsol_account
            .as_ref()
            .ok_or(ErrorCode::InvalidCommissionTemporaryTokenAccount)?;

        unwrap_process_v3(
            accounts.payer.clone(),
            accounts.wsol_mint.clone(),
            accounts.payer_wsol_account.clone(),
            (**temp_wsol_account).clone(),
            amount,
            accounts.token_program.clone(),
        )
    }
}

/// Wrap SOL to WSOL process
pub fn wrap_process_v3<'info>(
    payer: Signer<'info>,
    wsol_account: InterfaceAccount<'info, TokenAccount>,
    amount: u64,
    token_program: Interface<'info, TokenInterface>,
) -> Result<()> {
    // Transfer SOL to WSOL account
    transfer_sol(payer.to_account_info(), wsol_account.to_account_info(), amount, None)?;

    // Sync WSOL account to reflect the transferred SOL
    sync_wsol_account(wsol_account.to_account_info(), token_program.to_account_info(), None)?;

    Ok(())
}

/// Unwrap WSOL to SOL process
pub fn unwrap_process_v3<'info>(
    payer: Signer<'info>,
    wsol_mint: InterfaceAccount<'info, Mint>,
    wsol_account: InterfaceAccount<'info, TokenAccount>,
    temp_wsol_account: InterfaceAccount<'info, TokenAccount>,
    amount: u64,
    token_program: Interface<'info, TokenInterface>,
) -> Result<()> {
    // Transfer WSOL to temporary account
    transfer_token(
        payer.to_account_info(),
        wsol_account.to_account_info(),
        temp_wsol_account.to_account_info(),
        wsol_mint.to_account_info(),
        token_program.to_account_info(),
        amount,
        wsol_mint.decimals,
        None,
    )?;

    // Close temporary account to unwrap WSOL to SOL
    close_token_account(
        temp_wsol_account.to_account_info(),
        payer.to_account_info(),
        payer.to_account_info(),
        token_program.to_account_info(),
        None,
    )?;

    Ok(())
}

/// Get wrap/unwrap mint addresses
pub fn get_wrap_unwrap_mints(wrap_direction: bool) -> (Pubkey, Pubkey) {
    let sol_mint_id = system_program::id();
    let wsol_mint_id = wsol_program::id();

    if wrap_direction {
        // Wrap: SOL -> WSOL
        (sol_mint_id, wsol_mint_id)
    } else {
        // Unwrap: WSOL -> SOL
        (wsol_mint_id, sol_mint_id)
    }
}

/// Log wrap/unwrap initial information
fn log_wrap_unwrap_initial_info<'info>(
    payer: &Signer<'info>,
    payer_wsol_account: &InterfaceAccount<'info, TokenAccount>,
    wrap_direction: bool,
    amount_in: u64,
    order_id: u64,
) -> Result<(Pubkey, Pubkey, u64, u64)> {
    let (source_mint, destination_mint) = get_wrap_unwrap_mints(wrap_direction);

    log_swap_basic_info(order_id, &source_mint, &destination_mint, &payer.key(), &payer.key());

    let sol_balance = payer.lamports();
    let wsol_balance = payer_wsol_account.amount;

    let (before_source_balance, before_destination_balance) = if wrap_direction {
        // Wrap: SOL -> WSOL
        (sol_balance, wsol_balance)
    } else {
        // Unwrap: WSOL -> SOL
        (wsol_balance, sol_balance)
    };

    log_swap_balance_before(
        before_source_balance,
        before_destination_balance,
        amount_in,
        amount_in, // For wrap/unwrap, expect_amount_out equals amount_in
        amount_in,
    );

    Ok((source_mint, destination_mint, before_source_balance, before_destination_balance))
}

/// Log wrap/unwrap final information and calculate changes
fn log_wrap_unwrap_final_info<'info>(
    payer: &Signer<'info>,
    payer_wsol_account: &mut InterfaceAccount<'info, TokenAccount>,
    wrap_direction: bool,
    amount_in: u64,
    commission_amount: u64,
    platform_fee_amount: u64,
) -> Result<()> {
    let sol_balance_after = payer.lamports();

    payer_wsol_account.reload()?;
    let wsol_balance_after = payer_wsol_account.amount;

    let (after_source_balance, after_destination_balance) = if wrap_direction {
        // Wrap: SOL -> WSOL
        (sol_balance_after, wsol_balance_after)
    } else {
        // Unwrap: WSOL -> SOL
        (wsol_balance_after, sol_balance_after)
    };

    // Log output of actual business amount
    let source_token_change = amount_in;
    let destination_token_change = amount_in
        .checked_sub(commission_amount)
        .ok_or(ErrorCode::CalculationError)?
        .checked_sub(platform_fee_amount)
        .ok_or(ErrorCode::CalculationError)?;

    log_swap_end(
        after_source_balance,
        after_destination_balance,
        source_token_change,
        destination_token_change,
    );
    Ok(())
}

/// Transfer wrap/unwrap fees and log results
fn transfer_wrap_unwrap_fees_and_log<'info>(
    payer: &AccountInfo<'info>,
    payer_wsol_account: &InterfaceAccount<'info, TokenAccount>,
    wsol_mint: &InterfaceAccount<'info, Mint>,
    token_program: &Interface<'info, TokenInterface>,
    commission_amount: u64,
    platform_fee_amount: u64,
    commission_account: &Option<AccountInfo<'info>>,
    platform_fee_account: &Option<AccountInfo<'info>>,
    commission_info: &CommissionInfo,
    tob: bool,                                          // New parameter
    authority_pda_account: &Option<AccountInfo<'info>>, // New parameter
    wsol_sa_account: &Option<AccountInfo<'info>>,       // New parameter
) -> Result<()> {
    // Validate accounts before transfer
    if commission_amount > 0 {
        require!(commission_account.is_some(), ErrorCode::CommissionAccountIsNone);
    }
    if platform_fee_amount > 0 {
        require!(platform_fee_account.is_some(), ErrorCode::PlatformFeeAccountIsNone);
    }

    if commission_info.is_charge_sol() {
        // Transfer SOL fees
        transfer_sol_fees(
            payer,
            commission_amount,
            platform_fee_amount,
            commission_account,
            platform_fee_account,
            commission_info.commission_direction,
            tob,                   // Pass tob flag
            authority_pda_account, // Pass authority_pda for TOB mode
        )?;
    } else {
        // Transfer token fees
        transfer_token_fees(
            payer,
            payer_wsol_account,
            wsol_mint,
            token_program,
            commission_amount,
            platform_fee_amount,
            commission_account,
            platform_fee_account,
            commission_info.commission_direction,
            tob,                   // Pass tob flag
            authority_pda_account, // Pass for signing
            wsol_sa_account,       // Pass for WSOL transfers
        )?;
    }

    Ok(())
}

/// Transfer SOL fees
fn transfer_sol_fees<'info>(
    payer: &AccountInfo<'info>,
    commission_amount: u64,
    platform_fee_amount: u64,
    commission_account: &Option<AccountInfo<'info>>,
    platform_fee_account: &Option<AccountInfo<'info>>,
    commission_direction: bool,
    tob: bool,                                          // New parameter
    authority_pda_account: &Option<AccountInfo<'info>>, // New parameter
) -> Result<()> {
    if tob {
        // TOB mode: require authority_pda
        let authority_pda =
            authority_pda_account.as_ref().ok_or(ErrorCode::TobAuthorityPdaRequired)?;

        // TOB mode: two-step transfer
        // Step 1: payer → authority_pda (normal transfer, not fee)
        let total_amount = commission_amount
            .checked_add(platform_fee_amount)
            .ok_or(ErrorCode::CalculationError)?;

        transfer_sol(
            payer.to_account_info(),
            authority_pda.to_account_info(),
            total_amount,
            None, // User signs
        )?;

        // Step 2: authority_pda → commission/platform accounts (actual fee payment)
        if commission_amount > 0 {
            let commission_account = commission_account.as_ref().unwrap();
            let adjust_amount = transfer_sol_fee(
                authority_pda,
                commission_account,
                commission_amount,
                Some(SA_AUTHORITY_SEED),
            )?;
            log_commission_info(commission_direction, commission_amount, adjust_amount);
            commission_account.key().log();
        }

        if platform_fee_amount > 0 {
            let platform_fee_account = platform_fee_account.as_ref().unwrap();
            let adjust_amount = transfer_sol_fee(
                authority_pda,
                platform_fee_account,
                platform_fee_amount,
                Some(SA_AUTHORITY_SEED),
            )?;
            log_platform_fee_info(platform_fee_amount, adjust_amount, &platform_fee_account.key());
        }
    } else {
        // TOC mode: direct transfer (existing logic)
        if commission_amount > 0 {
            let commission_account = commission_account.as_ref().unwrap();
            let adjust_amount =
                transfer_sol_fee(payer, commission_account, commission_amount, None)?;
            log_commission_info(commission_direction, commission_amount, adjust_amount);
            commission_account.key().log();
        }

        if platform_fee_amount > 0 {
            let platform_fee_account = platform_fee_account.as_ref().unwrap();
            let adjust_amount =
                transfer_sol_fee(payer, platform_fee_account, platform_fee_amount, None)?;
            log_platform_fee_info(platform_fee_amount, adjust_amount, &platform_fee_account.key());
        }
    }

    Ok(())
}

/// Transfer token fees
fn transfer_token_fees<'info>(
    payer: &AccountInfo<'info>,
    payer_wsol_account: &InterfaceAccount<'info, TokenAccount>,
    wsol_mint: &InterfaceAccount<'info, Mint>,
    token_program: &Interface<'info, TokenInterface>,
    commission_amount: u64,
    platform_fee_amount: u64,
    commission_account: &Option<AccountInfo<'info>>,
    platform_fee_account: &Option<AccountInfo<'info>>,
    commission_direction: bool,
    tob: bool,                                          // New parameter
    authority_pda_account: &Option<AccountInfo<'info>>, // New parameter
    wsol_sa_account: &Option<AccountInfo<'info>>,       // New parameter
) -> Result<()> {
    if tob {
        // TOB mode: require both accounts
        let authority_pda =
            authority_pda_account.as_ref().ok_or(ErrorCode::TobAuthorityPdaRequired)?;
        let wsol_sa = wsol_sa_account.as_ref().ok_or(ErrorCode::TobWsolSaRequired)?;

        // TOB mode: two-step transfer
        // Step 1: payer_wsol_account → wsol_sa (normal transfer, not fee)
        let total_amount = commission_amount
            .checked_add(platform_fee_amount)
            .ok_or(ErrorCode::CalculationError)?;

        transfer_token(
            payer.to_account_info(),              // authority (payer is the owner)
            payer_wsol_account.to_account_info(), // from
            wsol_sa.to_account_info(),            // to
            wsol_mint.to_account_info(),          // mint
            token_program.to_account_info(),      // token_program
            total_amount,
            wsol_mint.decimals,
            None, // User signs
        )?;

        // Step 2: wsol_sa → commission/platform accounts (actual fee payment)
        if commission_amount > 0 {
            let commission_account = commission_account.as_ref().unwrap();
            transfer_token_fee(
                authority_pda, // authority for wsol_sa
                wsol_sa,       // from account
                wsol_mint,
                token_program,
                commission_account,
                commission_amount,
                Some(SA_AUTHORITY_SEED),
            )?;
            log_commission_info(commission_direction, commission_amount, 0);
            commission_account.key().log();
        }

        if platform_fee_amount > 0 {
            let platform_fee_account = platform_fee_account.as_ref().unwrap();
            transfer_token_fee(
                authority_pda, // authority for wsol_sa
                wsol_sa,       // from account
                wsol_mint,
                token_program,
                platform_fee_account,
                platform_fee_amount,
                Some(SA_AUTHORITY_SEED),
            )?;
            log_platform_fee_info(platform_fee_amount, 0, &platform_fee_account.key());
        }
    } else {
        // TOC mode: direct transfer (existing logic)
        if commission_amount > 0 {
            let commission_account = commission_account.as_ref().unwrap();
            transfer_token_fee(
                payer,
                &payer_wsol_account.to_account_info(),
                wsol_mint,
                token_program,
                commission_account,
                commission_amount,
                None,
            )?;
            log_commission_info(commission_direction, commission_amount, 0);
            commission_account.key().log();
        }

        if platform_fee_amount > 0 {
            let platform_fee_account = platform_fee_account.as_ref().unwrap();
            transfer_token_fee(
                payer,
                &payer_wsol_account.to_account_info(),
                wsol_mint,
                token_program,
                platform_fee_account,
                platform_fee_amount,
                None,
            )?;
            log_platform_fee_info(platform_fee_amount, 0, &platform_fee_account.key());
        }
    }

    Ok(())
}

/// Validate TOB accounts
fn validate_tob_accounts<'info>(
    tob: bool,
    is_charge_sol: bool,
    authority_pda_account: &Option<AccountInfo<'info>>,
    wsol_sa_account: &Option<AccountInfo<'info>>,
) -> Result<()> {
    if !tob {
        return Ok(());
    }

    // Always need authority_pda in TOB mode
    let authority = authority_pda_account.as_ref().ok_or(ErrorCode::TobAuthorityPdaRequired)?;

    require_keys_eq!(authority.key(), authority_pda::id(), ErrorCode::InvalidAuthorityPda);

    // Need wsol_sa only when charging WSOL
    if !is_charge_sol {
        let wsol_sa = wsol_sa_account.as_ref().ok_or(ErrorCode::TobWsolSaRequired)?;

        require_keys_eq!(wsol_sa.key(), wsol_sa::id(), ErrorCode::InvalidWsolSa);
    }

    Ok(())
}
