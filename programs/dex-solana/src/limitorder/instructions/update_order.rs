use crate::constants::*;
use crate::error::LimitOrderError;
use crate::state::{config::*, event::*, order::*};
use crate::utils::transfer_sol;
use anchor_lang::{prelude::*, solana_program};

#[event_cpi]
#[derive(Accounts)]
#[instruction(order_id: u64)]
pub struct UpdateOrder<'info> {
    /// CHECK: The order maker
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
        mut,
        seeds = [
            ORDER_V1_SEED.as_bytes(),
            &order_id.to_le_bytes(),
            maker.key().as_ref(),
        ],
        bump = order_pda.bump,
    )]
    pub order_pda: Account<'info, OrderV1>,

    /// System program
    pub system_program: Program<'info, System>,
}

pub fn update_order_handler<'a>(
    ctx: Context<UpdateOrder>,
    order_id: u64,
    expect_taking_amount: u64,
    min_return_amount: u64,
    deadline: u64,
    increase_fee: u64,
) -> Result<()> {
    msg!("UpdateOrder orderId: {}", order_id);

    // Check if the order is expired
    let order = &mut ctx.accounts.order_pda;
    let update_ts = solana_program::clock::Clock::get()?.unix_timestamp as u64;
    require_gte!(order.deadline, update_ts, LimitOrderError::OrderExpired);

    if expect_taking_amount == 0 && min_return_amount == 0 && deadline == 0 {
        return Err(LimitOrderError::InvalidUpdateParameter.into());
    }

    // Update expect_taking_amount
    if expect_taking_amount > 0 {
        order.expect_taking_amount = expect_taking_amount;
    }

    // Update min_return_amount
    if min_return_amount > 0 {
        order.min_return_amount = min_return_amount;
    }

    // Check if min_return_amount <= expect_taking_amount
    require!(
        order.min_return_amount <= order.expect_taking_amount,
        LimitOrderError::InvalidMinReturnAmount
    );

    // Update deadline: should >= 5 minutes
    if deadline > 0 {
        require_gte!(deadline, update_ts + MIN_DEADLINE, LimitOrderError::InvalidDeadline);
        order.deadline = deadline;
    }

    // Increase trade fee
    if increase_fee > 0 {
        transfer_sol(
            ctx.accounts.maker.to_account_info(),
            order.to_account_info(),
            increase_fee,
            None,
        )?;
    }

    emit_cpi!(UpdateOrderEvent {
        order_id: order.order_id,
        maker: order.maker,
        expect_taking_amount: order.expect_taking_amount,
        min_return_amount: order.min_return_amount,
        deadline: order.deadline,
        update_ts,
        increase_fee,
    });
    Ok(())
}
