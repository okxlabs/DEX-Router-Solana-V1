use anchor_lang::prelude::*;

#[account]
#[derive(Debug)]
pub struct OrderV1 {
    /// Bump to identify PDA.
    pub bump: u8,

    /// The order id.
    pub order_id: u64,

    /// The maker of the order.
    pub maker: Pubkey,

    /// The makeing amount of the order.
    pub making_amount: u64,

    /// The expect taking amount of the order.
    pub expect_taking_amount: u64,

    /// The min return amount of the order.
    pub min_return_amount: u64,

    /// The escrow token account of the order.
    pub escrow_token_account: Pubkey,

    /// Input token mint.
    pub input_token_mint: Pubkey,

    /// Output token mint.
    pub output_token_mint: Pubkey,

    /// Input token program.
    pub input_token_program: Pubkey,

    /// Output token program.
    pub output_token_program: Pubkey,

    /// The create timestamp of the order.
    pub create_ts: u64,

    /// The deadline of the order.
    pub deadline: u64,

    /// padding
    pub padding: [u8; 128],
}

impl Default for OrderV1 {
    fn default() -> Self {
        OrderV1 {
            bump: 0,
            order_id: 0,
            maker: Pubkey::default(),
            making_amount: 0,
            expect_taking_amount: 0,
            min_return_amount: 0,
            escrow_token_account: Pubkey::default(),
            input_token_mint: Pubkey::default(),
            output_token_mint: Pubkey::default(),
            input_token_program: Pubkey::default(),
            output_token_program: Pubkey::default(),
            create_ts: 0,
            deadline: 0,
            padding: [0u8; 128],
        }
    }
}

impl OrderV1 {
    pub const LEN: usize = 8 + std::mem::size_of::<OrderV1>();
}
