use crate::error::ErrorCode;
use crate::instructions::commission_wrap_unwrap::{
    log_wrap_unwrap_final_info, log_wrap_unwrap_initial_info,
};
use crate::utils::log_rate_info;
use crate::utils::logging::{log_commission_info, log_platform_fee_info};
use crate::utils::token::{sync_wsol_account, transfer_sol, transfer_token};
use crate::{
    COMMISSION_DENOMINATOR_V2, COMMISSION_RATE_LIMIT_V2, PLATFORM_FEE_DENOMINATOR_V2,
    PLATFORM_FEE_RATE_LIMIT_V2, SA_AUTHORITY_SEED, SEED_TEMP_WSOL, unwrap_process, wrap_process,
    wsol_program,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(AnchorDeserialize, AnchorSerialize, Clone)]
pub struct PlatformFeeWrapUnwrapArgsV2 {
    pub amount_in: u64,
    pub commission_info: u32,   // Commission rate
    pub platform_fee_rate: u32, // Platform fee rate
}

#[derive(Accounts)]
pub struct PlatformFeeWrapUnwrapAccountsV2<'info> {
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

    /// CHECK: system account or pda
    #[account(mut)]
    pub commission_sol_account: UncheckedAccount<'info>,

    #[account(
        mut,
        token::mint = wsol_mint,
        token::token_program = token_program,
    )]
    pub commission_wsol_account: InterfaceAccount<'info, TokenAccount>,

    #[account(mut)]
    pub source_token_sa: Option<UncheckedAccount<'info>>,

    #[account(mut)]
    pub destination_token_sa: Option<UncheckedAccount<'info>>,

    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
}

pub fn platform_fee_wrap_unwrap_handler_v2<'a>(
    ctx: Context<'_, '_, 'a, 'a, PlatformFeeWrapUnwrapAccountsV2<'a>>,
    args: PlatformFeeWrapUnwrapArgsV2,
    order_id: u64,
) -> Result<()> {
    let commission_direction = args.commission_info >> 31 == 1;
    let wrap_direction = ((args.commission_info & (1 << 30)) >> 30) == 1;
    let commission_rate = args.commission_info & ((1 << 30) - 1);

    log_rate_info(commission_rate, args.platform_fee_rate, None);

    if args.platform_fee_rate > 0 {
        require!(
            ctx.accounts.source_token_sa.is_some() || ctx.accounts.destination_token_sa.is_some(),
            ErrorCode::MissingSaAccount
        );
    }

    require!(
        args.platform_fee_rate as u64 <= PLATFORM_FEE_RATE_LIMIT_V2,
        ErrorCode::InvalidPlatformFeeRate
    );
    // CHECK: CommissionSwapArgs
    require!(commission_rate <= COMMISSION_RATE_LIMIT_V2, ErrorCode::InvalidCommissionRate);

    require!(
        ctx.accounts.wsol_mint.key() == wsol_program::id(),
        ErrorCode::InvalidCommissionTokenAccount
    );

    let (before_source_balance, before_destination_balance) = log_wrap_unwrap_initial_info(
        &ctx.accounts.wsol_mint,
        &ctx.accounts.payer,
        &ctx.accounts.payer_wsol_account,
        wrap_direction,
        args.amount_in,
        order_id,
    )?;

    let amount_out = args.amount_in;

    let commission_amount: u64;
    let mut platform_fee_amount: u64 = 0;

    if commission_direction {
        // Commission direction: true-fromToken
        // Commission for fromToken
        commission_amount = u64::try_from(
            u128::from(args.amount_in)
                .checked_mul(commission_rate as u128)
                .ok_or(ErrorCode::CalculationError)?
                .checked_div(COMMISSION_DENOMINATOR_V2 as u128 - commission_rate as u128)
                .ok_or(ErrorCode::CalculationError)?,
        )
        .unwrap();
    } else {
        // Commission direction: false-toToken
        // Commission for toToken
        commission_amount = u64::try_from(
            u128::from(amount_out)
                .checked_mul(commission_rate as u128)
                .ok_or(ErrorCode::CalculationError)?
                .checked_div(COMMISSION_DENOMINATOR_V2 as u128)
                .ok_or(ErrorCode::CalculationError)?,
        )
        .unwrap();
    }

    if args.platform_fee_rate > 0 {
        platform_fee_amount = u64::try_from(
            u128::from(commission_amount)
                .checked_mul(args.platform_fee_rate as u128)
                .ok_or(ErrorCode::CalculationError)?
                .checked_div(PLATFORM_FEE_DENOMINATOR_V2 as u128)
                .ok_or(ErrorCode::CalculationError)?,
        )
        .unwrap();
    }

    if wrap_direction {
        wrap_process(
            ctx.accounts.payer.clone(),
            ctx.accounts.payer_wsol_account.clone(),
            args.amount_in,
            ctx.accounts.token_program.clone(),
        )?;
    } else {
        if let Some(ref temp_wsol_account) = ctx.accounts.temp_wsol_account {
            unwrap_process(
                ctx.accounts.payer.clone(),
                ctx.accounts.wsol_mint.clone(),
                ctx.accounts.payer_wsol_account.clone(),
                (**temp_wsol_account).clone(),
                args.amount_in,
                ctx.accounts.token_program.clone(),
            )?;
        } else {
            return err!(ErrorCode::InvalidCommissionTemporaryTokenAccount);
        }
    }

    log_wrap_unwrap_final_info(
        &ctx.accounts.payer,
        &mut ctx.accounts.payer_wsol_account,
        wrap_direction,
        before_source_balance,
        before_destination_balance,
    )?;

    let sa_account = if commission_direction {
        ctx.accounts.source_token_sa.as_ref().unwrap()
    } else {
        ctx.accounts.destination_token_sa.as_ref().unwrap()
    };

    let sa_account_key = if sa_account.owner == &crate::token_program::ID {
        let sa_account_box = Box::leak(Box::new(sa_account.to_account_info()));
        InterfaceAccount::<TokenAccount>::try_from(sa_account_box).unwrap().owner
    } else {
        sa_account.key()
    };

    let commission_account_info =
        if (commission_direction && wrap_direction) || (!commission_direction && !wrap_direction) {
            ctx.accounts.commission_sol_account.to_account_info()
        } else {
            ctx.accounts.commission_wsol_account.to_account_info()
        };

    if (commission_direction && wrap_direction) || (!commission_direction && !wrap_direction) {
        // decide sa according to the direction
        // Transfer platform fee to sa
        if args.platform_fee_rate > 0 && platform_fee_amount > 0 {
            transfer_sol(
                ctx.accounts.payer.to_account_info(),
                sa_account.to_account_info(),
                platform_fee_amount,
                None,
            )?;

            // Sync source token sa
            sync_wsol_account(
                sa_account.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
                Some(SA_AUTHORITY_SEED),
            )?;
            log_platform_fee_info(platform_fee_amount, 0, &sa_account_key);
        }
        transfer_sol(
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.commission_sol_account.to_account_info(),
            commission_amount.checked_sub(platform_fee_amount).unwrap(),
            None,
        )?;
    } else {
        if args.platform_fee_rate > 0 && platform_fee_amount > 0 {
            transfer_token(
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.payer_wsol_account.to_account_info(),
                sa_account.to_account_info(),
                ctx.accounts.wsol_mint.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
                platform_fee_amount,
                ctx.accounts.wsol_mint.decimals,
                None,
            )?;

            log_platform_fee_info(platform_fee_amount, 0, &sa_account_key);
        }

        transfer_token(
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.payer_wsol_account.to_account_info(),
            ctx.accounts.commission_wsol_account.to_account_info(),
            ctx.accounts.wsol_mint.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            commission_amount.checked_sub(platform_fee_amount).unwrap(),
            ctx.accounts.wsol_mint.decimals,
            None,
        )?;
    }

    log_commission_info(
        commission_direction,
        commission_amount.checked_sub(platform_fee_amount).unwrap(),
        0,
    );
    commission_account_info.key().log();

    Ok(())
}
