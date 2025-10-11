use crate::state::{config::*, event::*, order::*};
use crate::utils::*;
use crate::wsol_program;
use crate::{constants::*, error::LimitOrderError};
use anchor_lang::{prelude::*, solana_program::clock, solana_program::sysvar};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[event_cpi]
#[derive(Accounts)]
#[instruction(order_id: u64)]
pub struct CancelOrder<'info> {
    /// The payer of the transaction
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: The order maker
    #[account(mut)]
    pub maker: AccountInfo<'info>,

    /// The global config account
    #[account(
        seeds = [
            GLOBAL_CONFIG_SEED.as_bytes(),
        ],
        bump = global_config.load()?.bump,
        constraint = !global_config.load()?.paused @ LimitOrderError::TradingPaused,
    )]
    pub global_config: AccountLoader<'info, GlobalConfig>,

    /// The order PDA account
    #[account(
        mut,
        close = maker,
        seeds = [
            ORDER_V1_SEED.as_bytes(),
            &order_id.to_le_bytes(),
            maker.key().as_ref(),
        ],
        bump = order_pda.bump,
    )]
    pub order_pda: Account<'info, OrderV1>,

    /// The escrow token account for the order
    #[account(
        mut,
        token::mint = input_token_mint,
        token::authority = order_pda,
        token::token_program = input_token_program,
        seeds = [
            ESCROW_TOKEN_SEED.as_bytes(),
            order_pda.key().as_ref(),
            input_token_mint.key().as_ref(),
        ],
        bump,
    )]
    pub escrow_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The user token account for input token
    #[account(
        mut,
        token::mint = input_token_mint,
        token::token_program = input_token_program,
    )]
    pub input_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The mint of input token
    #[account(mut)]
    pub input_token_mint: Box<InterfaceAccount<'info, Mint>>,

    /// SPL program for input token transfers
    pub input_token_program: Interface<'info, TokenInterface>,

    /// CHECK: Solana Instructions Sysvar
    #[account(address = sysvar::instructions::ID)]
    pub instructions_sysvar: UncheckedAccount<'info>,
}

pub fn cancel_order_handler(ctx: Context<CancelOrder>, order_id: u64, tips: u64) -> Result<()> {
    msg!("CancelOrder orderId: {}", order_id);
    let global_config = ctx.accounts.global_config.load()?;

    let update_ts = clock::Clock::get()?.unix_timestamp as u64;
    let order = &ctx.accounts.order_pda;
    let payer = ctx.accounts.payer.key();
    let maker = ctx.accounts.maker.key();
    let is_maker = payer == maker;
    // Only the maker & resolver can cancel the order
    if !is_maker {
        msg!("CancelOrder by Resolver");

        // Check if the resolver is the payer
        require!(global_config.is_resolver(payer), LimitOrderError::OnlyResolver);

        // Check if the order has expired
        #[cfg(feature = "check-deadline")]
        require_gt!(update_ts, order.deadline, LimitOrderError::OrderNotExpired);
    } else {
        msg!("CancelOrder by Maker");
    }

    // Check the input token owner
    let input_token_mint = ctx.accounts.input_token_mint.key();
    if input_token_mint == wsol_program::ID && !is_maker {
        // Owner is payer, support user place order with sol, The following instruction will close input_token_account for user and recover the rent through tips.
        // Owner is maker, support user place order with wsol
        require!(
            ctx.accounts.input_token_account.owner == payer
                || ctx.accounts.input_token_account.owner == maker,
            LimitOrderError::InvalidInputTokenAccount
        );
    } else {
        // Owner is maker, support user place order with other token
        require!(
            ctx.accounts.input_token_account.owner == maker,
            LimitOrderError::InvalidInputTokenAccount
        );
    }

    let escrow_token_account = &ctx.accounts.escrow_token_account;
    let amount = escrow_token_account.amount;

    let order_pda_seeds: &[&[&[u8]]] =
        &[&[ORDER_V1_SEED.as_bytes(), &order_id.to_le_bytes(), maker.as_ref(), &[order.bump]]];

    // Transfer the escrow token from the escrow account to the maker
    transfer_token(
        ctx.accounts.order_pda.to_account_info(),
        ctx.accounts.escrow_token_account.to_account_info(),
        ctx.accounts.input_token_account.to_account_info(),
        ctx.accounts.input_token_mint.to_account_info(),
        ctx.accounts.input_token_program.to_account_info(),
        amount,
        ctx.accounts.input_token_mint.decimals,
        Some(order_pda_seeds),
    )?;

    // Harvest the transfer fee if it exists
    if get_transfer_fee(&ctx.accounts.input_token_mint.to_account_info(), amount)? > 0 {
        harvest_withheld_tokens_to_mint(
            ctx.accounts.input_token_program.to_account_info(),
            ctx.accounts.input_token_mint.to_account_info(),
            ctx.accounts.escrow_token_account.to_account_info(),
            Some(order_pda_seeds),
        )?;
    }

    // Close the escrow token account
    close_token_account(
        ctx.accounts.escrow_token_account.to_account_info(),
        ctx.accounts.maker.to_account_info(),
        ctx.accounts.order_pda.to_account_info(),
        ctx.accounts.input_token_program.to_account_info(),
        Some(order_pda_seeds),
    )?;

    // Collect fees has to be done towards the end, since native transfers have to happen first
    if !is_maker {
        collect_fees(
            tips,
            global_config.fee_multiplier,
            ctx.accounts.order_pda.to_account_info(),
            ctx.accounts.payer.to_account_info(),
            &ctx.accounts.instructions_sysvar.to_account_info(),
            ORDER_MIN_RENT,
        )?;
    }

    emit_cpi!(RefundEvent { order_id, maker, input_token_mint, amount });
    emit_cpi!(CancelOrderEvent { order_id, payer, maker, update_ts });
    Ok(())
}
