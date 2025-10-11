use crate::instructions::common_swap::Route;
use crate::processor::common_processor::CommonSwapProcessor;
use crate::{SwapArgs, common_swap};
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(AnchorDeserialize, AnchorSerialize, Clone)]
pub struct CommissionSwapArgs {
    pub amount_in: u64,
    pub expect_amount_out: u64,
    pub min_return: u64,
    pub amounts: Vec<u64>,          // 1st level split amount
    pub routes: Vec<Vec<Route>>,    // 2nd level split route
    pub commission_rate: u16,       // Commission rate
    pub commission_direction: bool, // Commission direction: true-fromToken, false-toToken
}

pub trait CommonCommissionProcessor<'info> {
    fn commission_sol_process(
        &self,
        _amount_in: u64,
        _amount_out: u64,
        _expected_amount_out: u64,
        _commission_rate: u16,
        _commission_direction: bool,
        _payer: &AccountInfo<'info>,
        _commission_account: &AccountInfo<'info>,
        _trim_account: Option<&AccountInfo<'info>>,
        _source_mint: &InterfaceAccount<'info, Mint>,
        _destination_mint: &InterfaceAccount<'info, Mint>,
        _destination_token_account: &mut InterfaceAccount<'info, TokenAccount>,
        _destination_token_program: &Option<Interface<'info, TokenInterface>>,
        _source_token_sa: &mut Option<UncheckedAccount<'info>>,
        _destination_token_sa: &mut Option<UncheckedAccount<'info>>,
        _platform_fee_rate: Option<u16>,
    ) -> Result<()> {
        Ok(())
    }

    fn commission_token_process(
        &self,
        _amount_in: u64,
        _amount_out: u64,
        _expected_amount_out: u64,
        _commission_rate: u16,
        _commission_direction: bool,
        _payer: &AccountInfo<'info>,
        _commission_token_account: &InterfaceAccount<'info, TokenAccount>,
        _trim_token_account: Option<&AccountInfo<'info>>,
        _source_token_account: &InterfaceAccount<'info, TokenAccount>,
        _destination_token_account: &InterfaceAccount<'info, TokenAccount>,
        _source_mint: &InterfaceAccount<'info, Mint>,
        _destination_mint: &InterfaceAccount<'info, Mint>,
        _commission_token_program: AccountInfo<'info>,
        _trim_token_program: Option<AccountInfo<'info>>,
        _source_token_sa: &mut Option<UncheckedAccount<'info>>,
        _destination_token_sa: &mut Option<UncheckedAccount<'info>>,
        _platform_fee_rate: Option<u16>,
    ) -> Result<()> {
        Ok(())
    }
}

pub fn common_commission_sol_swap<
    'info,
    T: CommonSwapProcessor<'info>,
    U: CommonCommissionProcessor<'info>,
>(
    swap_processor: &T,
    commission_processor: &U,
    payer: &AccountInfo<'info>,
    owner: &AccountInfo<'info>,
    owner_seeds: Option<&[&[&[u8]]]>,
    source_token_account: &mut InterfaceAccount<'info, TokenAccount>,
    destination_token_account: &mut InterfaceAccount<'info, TokenAccount>,
    source_mint: &InterfaceAccount<'info, Mint>,
    destination_mint: &InterfaceAccount<'info, Mint>,
    sa_authority: &Option<UncheckedAccount<'info>>,
    source_token_sa: &mut Option<UncheckedAccount<'info>>,
    destination_token_sa: &mut Option<UncheckedAccount<'info>>,
    source_token_program: &Option<Interface<'info, TokenInterface>>,
    destination_token_program: &Option<Interface<'info, TokenInterface>>,
    associated_token_program: &Option<Program<'info, AssociatedToken>>,
    system_program: &Option<Program<'info, System>>,
    remaining_accounts: &'info [AccountInfo<'info>],
    args: SwapArgs,
    order_id: u64,
    commission_rate: u16,
    commission_direction: bool,
    commission_account: &AccountInfo<'info>,
    trim_account: Option<&AccountInfo<'info>>,
    platform_fee_rate: Option<u16>,
) -> Result<u64> {
    let amount_in = args.amount_in;
    let expect_amount_out = args.expect_amount_out;

    // 1. Swap
    let amount_out = common_swap(
        swap_processor,
        payer,
        owner,
        owner_seeds,
        source_token_account,
        destination_token_account,
        source_mint,
        destination_mint,
        sa_authority,
        source_token_sa,
        destination_token_sa,
        source_token_program,
        destination_token_program,
        associated_token_program,
        system_program,
        remaining_accounts,
        args,
        order_id,
        None,
        None,
        None,
    )?;

    // 2. Commission SOL
    commission_processor.commission_sol_process(
        amount_in,
        amount_out,
        expect_amount_out,
        commission_rate,
        commission_direction,
        payer,
        commission_account,
        trim_account,
        source_mint,
        destination_mint,
        destination_token_account,
        destination_token_program,
        source_token_sa,
        destination_token_sa,
        platform_fee_rate,
    )?;
    Ok(amount_out)
}

pub fn common_commission_token_swap<
    'info,
    T: CommonSwapProcessor<'info>,
    U: CommonCommissionProcessor<'info>,
>(
    swap_processor: &T,
    commission_processor: &U,
    payer: &AccountInfo<'info>,
    owner: &AccountInfo<'info>,
    owner_seeds: Option<&[&[&[u8]]]>,
    source_token_account: &mut InterfaceAccount<'info, TokenAccount>,
    destination_token_account: &mut InterfaceAccount<'info, TokenAccount>,
    source_mint: &InterfaceAccount<'info, Mint>,
    destination_mint: &InterfaceAccount<'info, Mint>,
    sa_authority: &Option<UncheckedAccount<'info>>,
    source_token_sa: &mut Option<UncheckedAccount<'info>>,
    destination_token_sa: &mut Option<UncheckedAccount<'info>>,
    source_token_program: &Option<Interface<'info, TokenInterface>>,
    destination_token_program: &Option<Interface<'info, TokenInterface>>,
    associated_token_program: &Option<Program<'info, AssociatedToken>>,
    system_program: &Option<Program<'info, System>>,
    remaining_accounts: &'info [AccountInfo<'info>],
    args: SwapArgs,
    order_id: u64,
    commission_rate: u16,
    commission_direction: bool,
    commission_token_account: &InterfaceAccount<'info, TokenAccount>,
    trim_token_account: Option<&AccountInfo<'info>>,
    commission_token_program: AccountInfo<'info>,
    trim_token_program: Option<AccountInfo<'info>>,
    platform_fee_rate: Option<u16>,
) -> Result<u64> {
    let amount_in = args.amount_in;
    let expect_amount_out = args.expect_amount_out;

    // 1. Swap
    let amount_out = common_swap(
        swap_processor,
        payer,
        owner,
        owner_seeds,
        source_token_account,
        destination_token_account,
        source_mint,
        destination_mint,
        sa_authority,
        source_token_sa,
        destination_token_sa,
        source_token_program,
        destination_token_program,
        associated_token_program,
        system_program,
        remaining_accounts,
        args,
        order_id,
        None,
        None,
        None,
    )?;

    // 2. Commission token
    commission_processor.commission_token_process(
        amount_in,
        amount_out,
        expect_amount_out,
        commission_rate,
        commission_direction,
        payer,
        commission_token_account,
        trim_token_account,
        source_token_account,
        destination_token_account,
        source_mint,
        destination_mint,
        commission_token_program,
        trim_token_program,
        source_token_sa,
        destination_token_sa,
        platform_fee_rate,
    )?;
    Ok(amount_out)
}
