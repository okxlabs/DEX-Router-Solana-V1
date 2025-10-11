use crate::constants::*;
use crate::error::LimitOrderError;
use crate::state::{config::*, event::*, order::*};
use crate::utils::{transfer_sol, transfer_token};
use anchor_lang::{prelude::*, solana_program};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[event_cpi]
#[derive(Accounts)]
#[instruction(order_id: u64)]
pub struct PlaceOrder<'info> {
    /// The maker of the order
    #[account(mut)]
    pub maker: Signer<'info>,

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
        init,
        seeds = [
            ORDER_V1_SEED.as_bytes(),
            &order_id.to_le_bytes(),
            maker.key().as_ref(),
        ],
        bump,
        payer = maker,
        space = OrderV1::LEN
    )]
    pub order_pda: Account<'info, OrderV1>,

    /// The escrow token account for the order
    #[account(
        init,
        token::mint = input_token_mint,
        token::authority = order_pda,
        token::token_program = input_token_program,
        seeds = [
            ESCROW_TOKEN_SEED.as_bytes(),
            order_pda.key().as_ref(),
            input_token_mint.key().as_ref(),
        ],
        bump,
        payer = maker,
    )]
    pub escrow_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The user token account for input token
    #[account(
        mut,
        token::mint = input_token_mint,
        token::authority = maker,
        token::token_program = input_token_program,
    )]
    pub input_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The mint of input token
    #[account(
        token::token_program = input_token_program,
    )]
    pub input_token_mint: Box<InterfaceAccount<'info, Mint>>,

    /// The mint of output token
    #[account(
        token::token_program = output_token_program,
    )]
    pub output_token_mint: Box<InterfaceAccount<'info, Mint>>,

    pub input_token_program: Interface<'info, TokenInterface>,
    pub output_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn place_order_handler(
    ctx: Context<PlaceOrder>,
    order_id: u64,
    making_amount: u64,
    expect_taking_amount: u64,
    min_return_amount: u64,
    deadline: u64,
    trade_fee: u64,
) -> Result<()> {
    msg!("PlaceOrder orderId: {}", order_id);
    let global_config = ctx.accounts.global_config.load()?;

    // Check if the deadline is valid: should >= 5 minutes
    let create_ts = solana_program::clock::Clock::get()?.unix_timestamp as u64;
    require_gte!(deadline, create_ts + MIN_DEADLINE, LimitOrderError::InvalidDeadline);

    // Check if the input token is the same as the output token
    require!(
        ctx.accounts.input_token_mint.key() != ctx.accounts.output_token_mint.key(),
        LimitOrderError::InputAndOutputTokenSame
    );

    // Check amount
    require!(making_amount > 0, LimitOrderError::InvalidMakingAmount);
    require!(expect_taking_amount > 0, LimitOrderError::InvalidExpectTakingAmount);
    require!(
        min_return_amount <= expect_taking_amount && min_return_amount > 0,
        LimitOrderError::InvalidMinReturnAmount
    );
    require!(trade_fee >= global_config.trade_fee, LimitOrderError::InvalidTradeFee);

    // Prepaid trade fee
    transfer_sol(
        ctx.accounts.maker.to_account_info(),
        ctx.accounts.order_pda.to_account_info(),
        trade_fee,
        None,
    )?;

    let before_balance = ctx.accounts.escrow_token_account.amount;
    // Transfer input token from user to escrow account
    transfer_token(
        ctx.accounts.maker.to_account_info(),
        ctx.accounts.input_token_account.to_account_info(),
        ctx.accounts.escrow_token_account.to_account_info(),
        ctx.accounts.input_token_mint.to_account_info(),
        ctx.accounts.input_token_program.to_account_info(),
        making_amount,
        ctx.accounts.input_token_mint.decimals,
        None,
    )?;

    // Calculate the actual making amount
    ctx.accounts.escrow_token_account.reload()?;
    let after_balance = ctx.accounts.escrow_token_account.amount;
    let actual_making_amount =
        after_balance.checked_sub(before_balance).ok_or(LimitOrderError::MathOverflow)?;
    require!(actual_making_amount > 0, LimitOrderError::ActualMakingAmountIsZero);

    let maker = ctx.accounts.maker.key();
    let input_token_mint = ctx.accounts.input_token_mint.key();
    let output_token_mint = ctx.accounts.output_token_mint.key();

    // Initialize the order PDA
    let order_pda = &mut ctx.accounts.order_pda;
    order_pda.order_id = order_id;
    order_pda.maker = maker;
    order_pda.making_amount = actual_making_amount;
    order_pda.expect_taking_amount = expect_taking_amount;
    order_pda.min_return_amount = min_return_amount;
    order_pda.create_ts = create_ts;
    order_pda.deadline = deadline;
    order_pda.escrow_token_account = ctx.accounts.escrow_token_account.key();
    order_pda.input_token_mint = input_token_mint;
    order_pda.output_token_mint = output_token_mint;
    order_pda.input_token_program = ctx.accounts.input_token_program.key();
    order_pda.output_token_program = ctx.accounts.output_token_program.key();
    order_pda.bump = ctx.bumps.order_pda;
    order_pda.padding = [0u8; 128];

    emit_cpi!(PlaceOrderEvent {
        order_id,
        maker,
        input_token_mint,
        output_token_mint,
        making_amount: actual_making_amount,
        expect_taking_amount,
        min_return_amount,
        create_ts,
        deadline,
        trade_fee,
    });
    Ok(())
}
