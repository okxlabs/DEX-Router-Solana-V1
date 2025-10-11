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

#[cfg(feature = "staging")]
declare_id!("preZmu827KVPCoQ4LYwSoec13x6seQrKA3QpjgDtx1R");

#[cfg(not(feature = "staging"))]
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

    pub fn swap_tob_v3_enhanced<'a>(
        ctx: Context<'_, '_, 'a, 'a, CommissionProxySwapAccountsV3<'a>>,
        args: SwapArgs,
        commission_info: u32,
        trim_rate: u8,
        charge_rate: u16,
        platform_fee_rate: u16,
        order_id: u64,
    ) -> Result<()> {
        instructions::swap_tob_enhanced_handler(
            ctx,
            args,
            commission_info,
            order_id,
            trim_rate,
            charge_rate,
            Some(platform_fee_rate),
        )
    }

    pub fn wrap_unwrap_v3<'a>(
        ctx: Context<'_, '_, 'a, 'a, PlatformFeeWrapUnwrapAccounts<'a>>,
        args: PlatformFeeWrapUnwrapArgs,
    ) -> Result<()> {
        instructions::platform_fee_wrap_unwrap_handler_v3(ctx, args)
    }

    pub fn create_token_account<'a>(
        ctx: Context<'_, '_, 'a, 'a, CreateTokenAccountAccounts<'a>>,
        bump: u8,
    ) -> Result<()> {
        instructions::create_token_account_handler(ctx, bump)
    }

    pub fn create_token_account_with_seed<'a>(
        ctx: Context<'_, '_, 'a, 'a, CreateTokenAccountWithSeedAccounts<'a>>,
        bump: u8,
        seed: u32,
    ) -> Result<()> {
        instructions::create_token_account_with_seed_handler(ctx, bump, seed)
    }

    // ******************** Claim ******************** //
    pub fn claim<'a>(ctx: Context<'_, '_, 'a, 'a, ClaimAccounts<'a>>) -> Result<()> {
        instructions::claim_handler(ctx)
    }
}
