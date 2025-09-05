use anchor_lang::prelude::*;
pub mod adapters;
pub mod allocator;
pub mod constants;
pub mod error;
pub mod global_config;
pub mod instructions;
pub mod limitorder;
pub mod processor;
pub mod state;
pub mod utils;

pub use constants::*;
pub use global_config::instructions as global_config_instructions;
pub use global_config::instructions::*;
pub use instructions::*;
pub use limitorder::instructions as limitorder_instructions;
pub use limitorder::instructions::*;
pub use processor::*;

declare_id!("6m2CDdhRgxpH4WjvdzxAYbGxwdGUz5MziiL5jek2kBma");

#[program]
pub mod dex_solana {
    use super::*;

    pub fn swap<'a>(
        ctx: Context<'_, '_, 'a, 'a, SwapAccounts<'a>>,
        data: SwapArgs,
        order_id: u64,
    ) -> Result<()> {
        instructions::swap_handler(ctx, data, order_id)
    }

    pub fn from_swap_log<'a>(
        ctx: Context<'_, '_, 'a, 'a, FromSwapAccounts<'a>>,
        args: SwapArgs,
        bridge_to_args: BridgeToArgs,
        offset: u8,
        len: u8,
    ) -> Result<()> {
        instructions::from_swap_log_handler(ctx, args, bridge_to_args, offset, len)
    }

    // ******************** Commission Swap ******************** //
    pub fn commission_spl_swap<'a>(
        ctx: Context<'_, '_, 'a, 'a, CommissionSPLAccounts<'a>>,
        data: CommissionSwapArgs,
        order_id: u64,
    ) -> Result<()> {
        instructions::commission_spl_swap_handler(ctx, data, order_id)
    }

    pub fn commission_sol_swap<'a>(
        ctx: Context<'_, '_, 'a, 'a, CommissionSOLAccounts<'a>>,
        data: CommissionSwapArgs,
        order_id: u64,
    ) -> Result<()> {
        instructions::commission_sol_swap_handler(ctx, data, order_id)
    }

    pub fn commission_wrap_unwrap<'a>(
        ctx: Context<'_, '_, 'a, 'a, CommissionWrapUnwrapAccounts<'a>>,
        data: CommissionWrapUnwrapArgs,
        order_id: u64,
    ) -> Result<()> {
        instructions::commission_wrap_unwrap_handler(ctx, data, order_id)
    }

    pub fn commission_sol_from_swap<'a>(
        ctx: Context<'_, '_, 'a, 'a, CommissionSOLFromSwapAccounts<'a>>,
        args: SwapArgs,
        commission_rate: u16,
        bridge_to_args: BridgeToArgs,
        offset: u8,
        len: u8,
    ) -> Result<()> {
        instructions::commission_sol_from_swap_handler(
            ctx,
            args,
            commission_rate,
            bridge_to_args,
            offset,
            len,
        )
    }

    pub fn commission_spl_from_swap<'a>(
        ctx: Context<'_, '_, 'a, 'a, CommissionSPLFromSwapAccounts<'a>>,
        args: SwapArgs,
        commission_rate: u16,
        bridge_to_args: BridgeToArgs,
        offset: u8,
        len: u8,
    ) -> Result<()> {
        instructions::commission_spl_from_swap_handler(
            ctx,
            args,
            commission_rate,
            bridge_to_args,
            offset,
            len,
        )
    }

    // ******************** Proxy Swap ******************** //
    pub fn proxy_swap<'a>(
        ctx: Context<'_, '_, 'a, 'a, ProxySwapAccounts<'a>>,
        data: SwapArgs,
        order_id: u64,
    ) -> Result<()> {
        instructions::proxy_swap_handler(ctx, data, order_id)
    }

    pub fn commission_sol_proxy_swap<'a>(
        ctx: Context<'_, '_, 'a, 'a, CommissionSOLProxySwapAccounts<'a>>,
        data: SwapArgs,
        commission_rate: u16,
        commission_direction: bool,
        order_id: u64,
    ) -> Result<()> {
        instructions::commission_sol_proxy_swap_handler(
            ctx,
            data,
            commission_rate,
            commission_direction,
            order_id,
        )
    }

    pub fn commission_spl_proxy_swap<'a>(
        ctx: Context<'_, '_, 'a, 'a, CommissionSPLProxySwapAccounts<'a>>,
        data: SwapArgs,
        commission_rate: u16,
        commission_direction: bool,
        order_id: u64,
    ) -> Result<()> {
        instructions::commission_spl_proxy_swap_handler(
            ctx,
            data,
            commission_rate,
            commission_direction,
            order_id,
        )
    }

    // ******************** Platform Fee Swap ******************** //
    pub fn platform_fee_sol_proxy_swap_v2<'a>(
        ctx: Context<'_, '_, 'a, 'a, CommissionSOLProxySwapAccounts<'a>>,
        args: SwapArgs,
        commission_info: u32,
        platform_fee_rate: u32,
        trim_rate: u8,
        order_id: u64,
    ) -> Result<()> {
        instructions::platform_fee_sol_proxy_swap_handler_v2(
            ctx,
            args,
            commission_info,
            order_id,
            platform_fee_rate,
            trim_rate,
        )
    }

    pub fn platform_fee_spl_proxy_swap_v2<'a>(
        ctx: Context<'_, '_, 'a, 'a, CommissionSPLProxySwapAccounts<'a>>,
        args: SwapArgs,
        commission_info: u32,
        platform_fee_rate: u32,
        trim_rate: u8,
        order_id: u64,
    ) -> Result<()> {
        instructions::platform_fee_spl_proxy_swap_handler_v2(
            ctx,
            args,
            commission_info,
            order_id,
            platform_fee_rate,
            trim_rate,
        )
    }

    pub fn platform_fee_sol_wrap_unwrap_v2<'a>(
        ctx: Context<'_, '_, 'a, 'a, PlatformFeeWrapUnwrapAccountsV2<'a>>,
        args: PlatformFeeWrapUnwrapArgsV2,
        order_id: u64,
    ) -> Result<()> {
        instructions::platform_fee_wrap_unwrap_handler_v2(ctx, args, order_id)
    }

    // ******************** Swap V3 ******************** //
    pub fn swap_v3<'a>(
        ctx: Context<'_, '_, 'a, 'a, CommissionProxySwapAccountsV3<'a>>,
        args: SwapArgs,
        commission_info: u32,
        platform_fee_rate: u16,
        order_id: u64,
    ) -> Result<()> {
        instructions::swap_toc_handler(
            ctx,
            args,
            commission_info,
            order_id,
            Some(platform_fee_rate),
        )
    }

    pub fn swap_tob_v3<'a>(
        ctx: Context<'_, '_, 'a, 'a, CommissionProxySwapAccountsV3<'a>>,
        args: SwapArgs,
        commission_info: u32,
        trim_rate: u8,
        platform_fee_rate: u16,
        order_id: u64,
    ) -> Result<()> {
        instructions::swap_tob_handler(
            ctx,
            args,
            commission_info,
            order_id,
            Some(trim_rate),
            Some(platform_fee_rate),
        )
    }

    /// Swap ToB with optional specified receiver
    /// - For normal token swaps: sol_receiver should be None
    /// - For swap to SOL with custom receiver: sol_receiver should be Some and acc_close_flag must be true
    pub fn swap_tob_v3_with_receiver<'a>(
        ctx: Context<'_, '_, 'a, 'a, CommissionProxySwapAccountsV3WithReceiver<'a>>,
        args: SwapArgs,
        commission_info: u32,
        trim_rate: u8,
        platform_fee_rate: u16,
        order_id: u64,
    ) -> Result<()> {
        instructions::swap_tob_specified_receiver_handler(
            ctx,
            args,
            commission_info,
            order_id,
            Some(trim_rate),
            Some(platform_fee_rate),
        )
    }

    pub fn wrap_unwrap_v3<'a>(
        ctx: Context<'_, '_, 'a, 'a, PlatformFeeWrapUnwrapAccounts<'a>>,
        args: PlatformFeeWrapUnwrapArgs,
    ) -> Result<()> {
        instructions::platform_fee_wrap_unwrap_handler_v3(ctx, args)
    }

    // ******************** Claim ******************** //
    pub fn claim<'a>(
        ctx: Context<'_, '_, 'a, 'a, ClaimAccounts<'a>>,
    ) -> Result<()> {
        instructions::claim_handler(ctx)
    }

    // ******************** Global Config ******************** //
    pub fn init_global_config(ctx: Context<InitGlobalConfig>, trade_fee: u64) -> Result<()> {
        global_config_instructions::init_global_config_handler(ctx, trade_fee)
    }

    pub fn set_admin(ctx: Context<UpdateGlobalConfig>, admin: Pubkey) -> Result<()> {
        global_config_instructions::set_admin_handler(ctx, admin)
    }

    pub fn add_resolver(ctx: Context<UpdateGlobalConfig>, resolver: Pubkey) -> Result<()> {
        global_config_instructions::add_resolver_handler(ctx, resolver)
    }

    pub fn remove_resolver(ctx: Context<UpdateGlobalConfig>, resolver: Pubkey) -> Result<()> {
        global_config_instructions::remove_resolver_handler(ctx, resolver)
    }

    pub fn set_trade_fee(ctx: Context<UpdateGlobalConfig>, trade_fee: u64) -> Result<()> {
        global_config_instructions::set_trade_fee_handler(ctx, trade_fee)
    }

    pub fn pause(ctx: Context<UpdateGlobalConfig>) -> Result<()> {
        global_config_instructions::pause_trading_handler(ctx)
    }

    pub fn unpause(ctx: Context<UpdateGlobalConfig>) -> Result<()> {
        global_config_instructions::unpause_trading_handler(ctx)
    }

    pub fn set_fee_multiplier(ctx: Context<UpdateGlobalConfig>, fee_multiplier: u8) -> Result<()> {
        global_config_instructions::set_fee_multiplier_handler(ctx, fee_multiplier)
    }

    // ******************** Limit Order ******************** //
    pub fn place_order(
        ctx: Context<PlaceOrder>,
        order_id: u64,
        making_amount: u64,
        expect_taking_amount: u64,
        min_return_amount: u64,
        deadline: u64,
        trade_fee: u64,
    ) -> Result<()> {
        limitorder_instructions::place_order_handler(
            ctx,
            order_id,
            making_amount,
            expect_taking_amount,
            min_return_amount,
            deadline,
            trade_fee,
        )
    }

    pub fn update_order(
        ctx: Context<UpdateOrder>,
        order_id: u64,
        expect_taking_amount: u64,
        min_return_amount: u64,
        deadline: u64,
        increase_fee: u64,
    ) -> Result<()> {
        limitorder_instructions::update_order_handler(
            ctx,
            order_id,
            expect_taking_amount,
            min_return_amount,
            deadline,
            increase_fee,
        )
    }

    pub fn cancel_order(ctx: Context<CancelOrder>, order_id: u64, tips: u64) -> Result<()> {
        limitorder_instructions::cancel_order_handler(ctx, order_id, tips)
    }

    pub fn fill_order_by_resolver<'a>(
        ctx: Context<'_, '_, 'a, 'a, FillOrder<'a>>,
        order_id: u64,
        tips: u64,
        args: SwapArgs,
    ) -> Result<()> {
        limitorder_instructions::fill_order_by_resolver_handler(ctx, order_id, tips, args)
    }

    pub fn commission_fill_order<'a>(
        ctx: Context<'_, '_, 'a, 'a, CommissionFillOrder<'a>>,
        order_id: u64,
        tips: u64,
        args: SwapArgs,
        commission_info: u32,
    ) -> Result<()> {
        limitorder_instructions::commission_fill_order_handler(
            ctx,
            order_id,
            tips,
            args,
            commission_info,
        )
    }
}
