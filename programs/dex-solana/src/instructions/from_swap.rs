use crate::BRIDGE_TO_LOG_SELECTOR;
use crate::processor::swap_processor::SwapProcessor;
use crate::{SwapArgs, common_swap, error::ErrorCode};
use anchor_lang::{
    prelude::*,
    solana_program::{instruction::Instruction, program::invoke},
};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::Token,
    token_2022::Token2022,
    token_interface::{Mint, TokenAccount},
};

#[derive(Accounts)]
pub struct FromSwapAccounts<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        token::mint = source_mint,
        token::authority = payer,
    )]
    pub source_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = destination_mint,
    )]
    pub destination_token_account: InterfaceAccount<'info, TokenAccount>,

    pub source_mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub destination_mint: InterfaceAccount<'info, Mint>,

    /// CHECK: bridge_program
    #[account(address = crate::okx_bridge_program::id())]
    pub bridge_program: AccountInfo<'info>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub token_2022_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct BridgeToArgs {
    pub adaptor_id: AdaptorID, // bridge adaptor id
    pub to: Vec<u8>,           // recipient address on target chain
    pub order_id: u64,         // order id for okx
    pub to_chain_id: u64,      // target chain id
    pub amount: u64,           // amount to bridge
    pub swap_type: SwapType,   // swap type
    pub data: Vec<u8>,         // data for bridge
    pub ext_data: Vec<u8>,     // ext data for extension feature
}

#[derive(AnchorDeserialize, AnchorSerialize)]
pub enum SwapType {
    BRIDGE,
    SWAPANDBRIDGE,
}

#[derive(AnchorDeserialize, AnchorSerialize)]
pub enum AdaptorID {
    /* 00 */ Bridge0,
    /* 01 */ Bridge1,
    /* 02 */ Bridge2,
    /* 03 */ Bridge3,
    /* 04 */ Bridge4,
    /* 05 */ Bridge5,
    /* 06 */ Bridge6,
    /* 07 */ Bridge7,
    /* 08 */ Bridge8,
    /* 09 */ Bridge9,
    /* 10 */ Bridge10,
    /* 11 */ Bridge11,
    /* 12 */ Bridge12,
    /* 13 */ Bridge13,
    /* 14 */ Bridge14,
    /* 15 */ Bridge15,
    /* 16 */ Bridge16,
    /* 17 */ Bridge17,
    /* 18 */ Cctp,
    /* 19 */ Bridge19,
    /* 20 */ Bridge20,
    /* 21 */ Wormhole,
    /* 22 */ Meson,
    /* 23 */ Bridge23,
    /* 24 */ Bridge24,
    /* 25 */ Bridge25,
    /* 26 */ Bridge26,
    /* 27 */ Bridge27,
    /* 28 */ Bridge28,
    /* 29 */ Bridge29,
    /* 30 */ Bridge30,
    /* 31 */ Bridge31,
    /* 32 */ Bridge32,
    /* 33 */ Bridge33,
    /* 34 */ Debridgedln,
    /* 35 */ Bridge35,
    /* 36 */ Bridge36,
    /* 37 */ Bridge37,
    /* 38 */ Bridge38,
    /* 39 */ Bridge39,
    /* 40 */ Bridge40,
    /* 41 */ Allbridge,
    /* 42 */ Bridge42,
    /* 43 */ Bridge43,
    /* 44 */ Bridge44,
    /* 45 */ Bridge45,
    /* 46 */ Bridge46,
    /* 47 */ MayanSwift,
    /* 48 */ Bridge48,
    /* 49 */ Bridge49,
}

pub fn from_swap_log_handler<'a>(
    ctx: Context<'_, '_, 'a, 'a, FromSwapAccounts<'a>>,
    args: SwapArgs,
    bridge_to_args: BridgeToArgs,
    offset: u8,
    len: u8,
) -> Result<()> {
    // 1.Smart swap
    let amount_out = common_swap(
        &SwapProcessor,
        &ctx.accounts.payer,
        &ctx.accounts.payer,
        None,
        &mut ctx.accounts.source_token_account,
        &mut ctx.accounts.destination_token_account,
        &ctx.accounts.source_mint,
        &ctx.accounts.destination_mint,
        &None,
        &mut None,
        &mut None,
        &None,
        &None,
        &None,
        &None,
        ctx.remaining_accounts,
        args,
        0,
        None,
        None,
        None,
    )?;

    msg!("Swap amount_out: {}", amount_out);

    // 2. CPI bridge_to_log
    cpi_bridge_to_log(
        bridge_to_args,
        amount_out,
        offset,
        len,
        &ctx.accounts.bridge_program,
        &ctx.accounts.payer,
        &ctx.accounts.destination_token_account,
        &ctx.accounts.destination_mint,
        &ctx.accounts.associated_token_program,
        &ctx.accounts.token_program,
        &ctx.accounts.token_2022_program,
        &ctx.accounts.system_program,
        &ctx.remaining_accounts,
    )?;

    Ok(())
}

pub fn cpi_bridge_to_log<'info>(
    bridge_to_args: BridgeToArgs,
    amount_out: u64,
    offset: u8,
    len: u8,
    bridge_program: &AccountInfo<'info>,
    payer: &Signer<'info>,
    destination_token_account: &InterfaceAccount<'info, TokenAccount>,
    destination_mint: &InterfaceAccount<'info, Mint>,
    associated_token_program: &Program<'info, AssociatedToken>,
    token_program: &AccountInfo<'info>,
    token_2022_program: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    remaining_accounts: &[AccountInfo<'info>],
) -> Result<()> {
    let offset = offset as usize;
    let len = len as usize;
    require!(remaining_accounts.len() >= offset + len, ErrorCode::InvalidAccountsLength);
    // get bridgeTo remaining accounts
    let bridge_remaining_accounts = Vec::from(&remaining_accounts[offset..offset + len]);

    // reset amount
    let mut bridge_to_args = bridge_to_args;
    bridge_to_args.amount = amount_out;

    let serialized_args = bridge_to_args.try_to_vec()?;
    let mut data = Vec::with_capacity(BRIDGE_TO_LOG_SELECTOR.len() + serialized_args.len());
    data.extend_from_slice(BRIDGE_TO_LOG_SELECTOR);
    data.extend_from_slice(&serialized_args);

    let mut accounts = vec![
        AccountMeta::new(payer.key(), true),
        AccountMeta::new(destination_token_account.key(), false),
        AccountMeta::new(destination_mint.key(), false),
        AccountMeta::new_readonly(associated_token_program.key(), false),
        AccountMeta::new_readonly(token_program.key(), false),
        AccountMeta::new_readonly(token_2022_program.key(), false),
        AccountMeta::new_readonly(system_program.key(), false),
    ];
    accounts.extend(bridge_remaining_accounts.to_account_metas(None));

    let mut accounts_infos = vec![
        payer.to_account_info(),
        destination_token_account.to_account_info(),
        destination_mint.to_account_info(),
        associated_token_program.to_account_info(),
        token_program.to_account_info(),
        token_2022_program.to_account_info(),
        system_program.to_account_info(),
    ];
    accounts_infos.extend(bridge_remaining_accounts.to_account_infos());

    let ix = Instruction { program_id: bridge_program.key(), accounts, data };
    invoke(&ix, &accounts_infos)?;

    Ok(())
}
