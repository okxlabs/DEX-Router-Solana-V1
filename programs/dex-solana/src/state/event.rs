use anchor_lang::prelude::*;

#[event]
pub struct InitGlobalConfigEvent {
    pub admin: Pubkey,
    pub trade_fee: u64,
}

#[event]
pub struct SetAdminEvent {
    pub admin: Pubkey,
}

#[event]
pub struct AddResolverEvent {
    pub resolver: Pubkey,
}

#[event]
pub struct RemoveResolverEvent {
    pub resolver: Pubkey,
}

#[event]
pub struct SetTradeFeeEvent {
    pub trade_fee: u64,
}

#[event]
pub struct PauseTradingEvent {
    pub paused: bool,
}

#[event]
pub struct SetFeeMultiplierEvent {
    pub fee_multiplier: u8,
}

// ******************** Limit Order V1 ******************** //

#[event]
pub struct PlaceOrderEvent {
    pub order_id: u64,
    pub maker: Pubkey,
    pub input_token_mint: Pubkey,
    pub output_token_mint: Pubkey,
    pub making_amount: u64,
    pub expect_taking_amount: u64,
    pub min_return_amount: u64,
    pub create_ts: u64,
    pub deadline: u64,
    pub trade_fee: u64,
}

#[event]
pub struct UpdateOrderEvent {
    pub order_id: u64,
    pub maker: Pubkey,
    pub expect_taking_amount: u64,
    pub min_return_amount: u64,
    pub deadline: u64,
    pub update_ts: u64,
    pub increase_fee: u64,
}

#[event]
pub struct RefundEvent {
    pub order_id: u64,
    pub maker: Pubkey,
    pub input_token_mint: Pubkey,
    pub amount: u64,
}

#[event]
pub struct CancelOrderEvent {
    pub order_id: u64,
    pub payer: Pubkey,
    pub maker: Pubkey,
    pub update_ts: u64,
}

#[event]
pub struct FillOrderEvent {
    pub order_id: u64,
    pub payer: Pubkey,
    pub maker: Pubkey,
    pub input_token_mint: Pubkey,
    pub output_token_mint: Pubkey,
    pub making_amount: u64,
    pub taking_amount: u64,
    pub update_ts: u64,
}
