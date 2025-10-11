use crate::constants::*;
use crate::error::LimitOrderError;
use crate::processor::proxy_swap_processor::ProxySwapProcessor;
use crate::state::{config::*, event::*, order::*};
use crate::utils::*;
use crate::{SwapArgs, common_swap};
use anchor_lang::{prelude::*, solana_program::clock::Clock, solana_program::sysvar};
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[event_cpi]
#[derive(Accounts)]
#[instruction(order_id: u64)]
pub struct FillOrder<'info> {
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
        constraint = global_config.load()?.is_resolver(payer.key()) @ LimitOrderError::OnlyResolver,
    )]
    pub global_config: AccountLoader<'info, GlobalConfig>,

    /// CHECK: sa_authority
    #[account(
        seeds = [
            SEED_SA,
        ],
        bump = BUMP_SA,
    )]
    pub sa_authority: Option<UncheckedAccount<'info>>,

    #[account(mut)]
    pub input_token_sa: Option<UncheckedAccount<'info>>,

    #[account(mut)]
    pub output_token_sa: Option<UncheckedAccount<'info>>,

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
    pub order_pda: Box<Account<'info, OrderV1>>,

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

    /// Temp input token account
    #[account(mut)]
    pub temp_input_token_account: Option<Box<InterfaceAccount<'info, TokenAccount>>>,

    /// The user token account for output token
    #[account(
        mut,
        token::mint = output_token_mint,
        token::token_program = output_token_program,
    )]
    pub output_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The mint of input token
    #[account(mut)]
    pub input_token_mint: Box<InterfaceAccount<'info, Mint>>,

    /// The mint of output token
    #[account(
        constraint = output_token_mint.key() == order_pda.output_token_mint,
    )]
    pub output_token_mint: Box<InterfaceAccount<'info, Mint>>,

    pub input_token_program: Interface<'info, TokenInterface>,
    pub output_token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Option<Program<'info, AssociatedToken>>,
    pub system_program: Option<Program<'info, System>>,

    /// CHECK: Solana Instructions Sysvar
    #[account(address = sysvar::instructions::ID)]
    pub instructions_sysvar: UncheckedAccount<'info>,
}

pub fn fill_order_by_resolver_handler<'a>(
    ctx: Context<'_, '_, 'a, 'a, FillOrder<'a>>,
    order_id: u64,
    tips: u64,
    args: SwapArgs,
) -> Result<()> {
    msg!("FillOrder orderId: {}", order_id);
    let order = &ctx.accounts.order_pda;

    // Check order deadline
    let current_ts = Clock::get()?.unix_timestamp as u64;
    require_gte!(order.deadline, current_ts, LimitOrderError::OrderExpired);

    // update on 2025-05-23: fix tax token issue start
    // Check amount_in
    // require!(
    //     args.amount_in == ctx.accounts.escrow_token_account.amount,
    //     LimitOrderError::InvalidAmountIn
    // );
    let escrow_token_amount = ctx.accounts.escrow_token_account.amount;
    msg!("FillOrder amount_in: {}, escrow_token_amount: {}", args.amount_in, escrow_token_amount);
    // update on 2025-05-23: fix tax token issue end

    let payer = ctx.accounts.payer.key();
    let maker = ctx.accounts.maker.key();
    let input_token_mint = ctx.accounts.input_token_mint.key();
    let output_token_mint = ctx.accounts.output_token_mint.key();
    let order_pda_seeds: &[&[&[u8]]] = &[&[
        ORDER_V1_SEED.as_bytes(),
        &order.order_id.to_le_bytes(),
        maker.as_ref(),
        &[order.bump],
    ]];

    let is_wsol_input = input_token_mint == wsol_program::ID;
    let is_wsol_output = output_token_mint == wsol_program::ID;

    // Set source token account
    let mut source_token_account = &mut ctx.accounts.escrow_token_account;
    if is_wsol_input {
        // The pump.fun adapter can only close a temporary wsol account of a system account
        if let Some(temp_input_token_account) = &mut ctx.accounts.temp_input_token_account {
            require!(
                temp_input_token_account.owner == payer,
                LimitOrderError::InvalidInputTokenOwner
            );

            // Transfer token to temp_input_token_account
            transfer_token(
                ctx.accounts.order_pda.to_account_info(),
                ctx.accounts.escrow_token_account.to_account_info(),
                temp_input_token_account.to_account_info(),
                ctx.accounts.input_token_mint.to_account_info(),
                ctx.accounts.input_token_program.to_account_info(),
                args.amount_in,
                ctx.accounts.input_token_mint.decimals,
                Some(order_pda_seeds),
            )?;
            source_token_account = temp_input_token_account;
        }
    }

    // Check output token owner
    let output_token_account = &mut ctx.accounts.output_token_account;
    if is_wsol_output {
        // Owner is payer, support toToken is sol, The following instruction will close output_token_account for user and recover the rent through tips.
        // Owner is maker, support toToken is wsol
        require!(
            output_token_account.owner == payer || output_token_account.owner == maker,
            LimitOrderError::InvalidOutputTokenOwner
        );
    } else {
        // Owner is maker, support other toToken
        require!(output_token_account.owner == maker, LimitOrderError::InvalidOutputTokenOwner);
    }

    // Reset swap args
    let mut _args = args.clone();
    _args.amount_in = escrow_token_amount;
    _args.expect_amount_out = order.expect_taking_amount;
    _args.min_return = order.min_return_amount;

    // Swap
    let actual_taking_amount = common_swap(
        &ProxySwapProcessor,
        &ctx.accounts.payer.to_account_info(),
        &ctx.accounts.order_pda.to_account_info(),
        Some(order_pda_seeds),
        source_token_account,
        output_token_account,
        &ctx.accounts.input_token_mint,
        &ctx.accounts.output_token_mint,
        &ctx.accounts.sa_authority,
        &mut ctx.accounts.input_token_sa,
        &mut ctx.accounts.output_token_sa,
        &Some(ctx.accounts.input_token_program.clone()),
        &Some(ctx.accounts.output_token_program.clone()),
        &ctx.accounts.associated_token_program,
        &ctx.accounts.system_program,
        ctx.remaining_accounts,
        _args,
        order_id,
        None,
        None,
        None,
    )?;

    if is_wsol_output && output_token_account.owner == payer {
        // Output token is sol, close the output_token_account and transfer sol to maker
        handle_sol_output(
            output_token_account,
            &ctx.accounts.payer.to_account_info(),
            &ctx.accounts.maker.to_account_info(),
            &ctx.accounts.output_token_program,
            actual_taking_amount,
        )?;
    }

    // Harvest the transfer fee if it exists
    if get_transfer_fee(&ctx.accounts.input_token_mint.to_account_info(), args.amount_in)? > 0 {
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

    // Collect fees
    collect_fees(
        tips,
        ctx.accounts.global_config.load()?.fee_multiplier,
        ctx.accounts.order_pda.to_account_info(),
        ctx.accounts.payer.to_account_info(),
        &ctx.accounts.instructions_sysvar.to_account_info(),
        ORDER_MIN_RENT,
    )?;

    // Emit event
    emit_cpi!(FillOrderEvent {
        order_id: order.order_id,
        payer,
        maker,
        input_token_mint,
        output_token_mint,
        making_amount: order.making_amount,
        taking_amount: actual_taking_amount,
        update_ts: current_ts,
    });
    Ok(())
}

fn handle_sol_output<'info>(
    output_token_account: &InterfaceAccount<'info, TokenAccount>,
    payer: &AccountInfo<'info>,
    maker: &AccountInfo<'info>,
    output_token_program: &Interface<'info, TokenInterface>,
    amount: u64,
) -> Result<()> {
    // Close the temp wsol account
    close_token_account(
        output_token_account.to_account_info(),
        payer.to_account_info(),
        payer.to_account_info(),
        output_token_program.to_account_info(),
        None,
    )?;
    // Transfer sol to maker
    transfer_sol(payer.to_account_info(), maker.to_account_info(), amount, None)?;
    msg!("Transfer sol to maker: {}", amount);
    Ok(())
}
