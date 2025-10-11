use crate::processor::common_processor::CommonSwapProcessor;
use crate::{SwapArgs, common_swap};
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

pub trait CommonCommissionProcessorV3<'info> {
    fn commission_process_v3(
        &self,
        _amount_in: u64,
        _amount_out: u64,
        _expected_amount_out: u64,
        _payer: &AccountInfo<'info>,
        _source_token_account: &InterfaceAccount<'info, TokenAccount>,
        _destination_token_account: &InterfaceAccount<'info, TokenAccount>,
        _source_mint: &InterfaceAccount<'info, Mint>,
        _destination_mint: &InterfaceAccount<'info, Mint>,
        _source_token_program: &Option<Interface<'info, TokenInterface>>,
        _destination_token_program: &Option<Interface<'info, TokenInterface>>,
        // SA
        _sa_authority: &mut Option<UncheckedAccount<'info>>,
        _source_token_sa: &mut Option<UncheckedAccount<'info>>,
        _destination_token_sa: &mut Option<UncheckedAccount<'info>>,
        // COMMISSION
        _commission_rate: u32,
        _commission_direction: bool,
        _commission_account: Option<&AccountInfo<'info>>,
        // TRIM
        _trim_rate: Option<u8>,
        _trim_account: Option<&AccountInfo<'info>>,
        // PLATFORM FEE
        _platform_fee_rate: Option<u16>,
        _platform_fee_account: Option<&AccountInfo<'info>>,
        // TOB
        _tob: bool,
    ) -> Result<()> {
        Ok(())
    }
}

pub fn common_commission_token_swap_v3<
    'info,
    T: CommonSwapProcessor<'info>,
    U: CommonCommissionProcessorV3<'info>,
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
    sa_authority: &mut Option<UncheckedAccount<'info>>,
    source_token_sa: &mut Option<UncheckedAccount<'info>>,
    destination_token_sa: &mut Option<UncheckedAccount<'info>>,
    source_token_program: &Option<Interface<'info, TokenInterface>>,
    destination_token_program: &Option<Interface<'info, TokenInterface>>,
    associated_token_program: &Option<Program<'info, AssociatedToken>>,
    system_program: &Option<Program<'info, System>>,
    remaining_accounts: &'info [AccountInfo<'info>],
    args: SwapArgs,
    order_id: u64,
    // COMMISSION
    commission_rate: u32,
    commission_direction: bool,
    commission_account: Option<&AccountInfo<'info>>,
    // TRIM
    trim_rate: Option<u8>,
    trim_account: Option<&AccountInfo<'info>>,
    // PLATFORM FEE
    platform_fee_rate: Option<u16>,
    platform_fee_account: Option<&AccountInfo<'info>>,
    // TOB
    tob: bool,
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
    commission_processor.commission_process_v3(
        amount_in,
        amount_out,
        expect_amount_out,
        payer,
        source_token_account,
        destination_token_account,
        source_mint,
        destination_mint,
        source_token_program,
        destination_token_program,
        // SA
        sa_authority,
        source_token_sa,
        destination_token_sa,
        // COMMISSION
        commission_rate,
        commission_direction,
        commission_account,
        // TRIM
        trim_rate,
        trim_account,
        // PLATFORM FEE
        platform_fee_rate,
        platform_fee_account,
        // TOB
        tob,
    )?;
    Ok(amount_out)
}
