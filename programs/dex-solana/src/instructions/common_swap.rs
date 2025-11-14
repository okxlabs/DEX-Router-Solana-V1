use crate::adapters::*;
use crate::constants::*;
use crate::error::ErrorCode;
use crate::processor::*;
use crate::utils::*;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone, PartialEq, Eq, Debug, strum::Display)]
pub enum Dex {
    SplTokenSwap,
    StableSwap,
    Whirlpool,
    MeteoraDynamicpool,
    RaydiumSwap,
    RaydiumStableSwap,
    RaydiumClmmSwap,
    AldrinExchangeV1,
    AldrinExchangeV2,
    LifinityV1,
    LifinityV2,
    RaydiumClmmSwapV2,
    FluxBeam,
    MeteoraDlmm,
    RaydiumCpmmSwap,
    OpenBookV2,
    WhirlpoolV2,
    Phoenix,
    ObricV2,
    SanctumAddLiq,
    SanctumRemoveLiq,
    SanctumNonWsolSwap,
    SanctumWsolSwap,
    PumpfunBuy,
    PumpfunSell,
    StabbleSwap,
    SanctumRouter,
    MeteoraVaultDeposit,
    MeteoraVaultWithdraw,
    Saros,
    MeteoraLst,
    Solfi,
    QualiaSwap,
    Zerofi,
    PumpfunammBuy,
    PumpfunammSell,
    Virtuals,
    VertigoBuy,
    VertigoSell,
    PerpetualsAddLiq,
    PerpetualsRemoveLiq,
    PerpetualsSwap,
    RaydiumLaunchpad,
    LetsBonkFun,
    Woofi,
    MeteoraDbc,
    MeteoraDlmmSwap2,
    MeteoraDAMMV2,
    Gavel,
    BoopfunBuy,
    BoopfunSell,
    MeteoraDbc2,
    GooseFX,
    Dooar,
    Numeraire,
    SaberDecimalWrapperDeposit,
    SaberDecimalWrapperWithdraw,
    SarosDlmm,
    OneDexSwap,
    Manifest,
    ByrealClmm,
    PancakeSwapV3Swap,
    PancakeSwapV3SwapV2,
    Tessera,
    #[strum(to_string = "SolRfq")]
    SolRfq {
        rfq_id: u64,
        expected_maker_amount: u64,
        expected_taker_amount: u64,
        maker_send_amount: u64,
        taker_send_amount: u64,
        expiry: u64,
        maker_use_native_sol: bool,
        taker_use_native_sol: bool,
    },
    PumpfunBuy2,
    PumpfunammBuy2,
    Humidifi,
    HeavenBuy,
    HeavenSell,
    SolfiV2,
    PumpfunBuy3,
    PumpfunSell3,
    PumpfunammBuy3,
    PumpfunammSell3,
    Goonfi,
    MoonitBuy,
    MoonitSell,
    RaydiumSwapV2,
    Whalestreet,
    #[strum(to_string = "SugarMoneyBuy")]
    SugarMoneyBuy {
        bonding_curve_bump: u8,
        bonding_curve_sol_associated_account_bump: u8,
    },
    #[strum(to_string = "SugarMoneySell")]
    SugarMoneySell {
        bonding_curve_bump: u8,
        bonding_curve_sol_associated_account_bump: u8,
    },
    MeteoraDAMMV2Swap2,
    AlphaQ,
    FutarchyAmm,
    PumpfunSell2,
}

#[derive(Debug)]
pub struct HopAccounts {
    pub last_to_account: Pubkey,
    pub from_account: Pubkey,
    pub to_account: Pubkey,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct Route {
    pub dexes: Vec<Dex>,
    pub weights: Vec<u8>,
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone)]
pub struct SwapArgs {
    pub amount_in: u64,
    pub expect_amount_out: u64,
    pub min_return: u64,
    pub amounts: Vec<u64>,       // 1st level split amount
    pub routes: Vec<Vec<Route>>, // 2nd level split route
}

#[event]
pub struct SwapEvent {
    pub dex: Dex,
    pub amount_in: u64,
    pub amount_out: u64,
}

impl std::fmt::Debug for SwapEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SwapEvent")
            .field("dex", &format_args!("{}", &self.dex))
            .field("amount_in", &self.amount_in)
            .field("amount_out", &self.amount_out)
            .finish()
    }
}

pub fn common_swap<'info, T: CommonSwapProcessor<'info>>(
    swap_processor: &T,
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
    fee_rate: Option<u32>,
    fee_direction: Option<bool>,
    fee_token_account: Option<&InterfaceAccount<'info, TokenAccount>>,
) -> Result<u64> {
    log_swap_basic_info(
        order_id,
        &source_mint.key(),
        &destination_mint.key(),
        &source_token_account.owner,
        &destination_token_account.owner,
    );

    let before_source_balance = source_token_account.amount;
    let before_destination_balance = destination_token_account.amount;
    let min_return = args.min_return;

    log_swap_balance_before(
        before_source_balance,
        before_destination_balance,
        args.amount_in,
        args.expect_amount_out,
        min_return,
    );

    // Verify sa_authority is valid
    if sa_authority.is_some() {
        require!(
            sa_authority.as_ref().unwrap().key() == authority_pda::ID,
            ErrorCode::InvalidSaAuthority
        );
    }

    // get swap accounts
    let (mut source_account, mut destination_account) = swap_processor.get_swap_accounts(
        payer,
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
    )?;

    // before swap hook
    let real_amount_in = swap_processor.before_swap(
        owner,
        source_token_account,
        source_mint,
        source_token_sa,
        source_token_program,
        args.amount_in,
        owner_seeds,
        fee_rate,
        fee_direction,
        fee_token_account,
    )?;

    // Common swap
    let amount_out = execute_swap(
        &mut source_account,
        &mut destination_account,
        remaining_accounts,
        args,
        real_amount_in,
        order_id,
        source_token_sa.is_some(),
        owner_seeds,
        Some(payer),
    )?;

    // after swap hook
    swap_processor.after_swap(
        sa_authority,
        destination_token_account,
        destination_mint,
        destination_token_sa,
        destination_token_program,
        amount_out,
        Some(SA_AUTHORITY_SEED),
        fee_rate,
        fee_direction,
        fee_token_account,
    )?;

    // source token account has been closed in pumpfun buy
    let after_source_balance = if source_token_account.get_lamports() != 0 {
        source_token_account.reload()?;
        source_token_account.amount
    } else {
        0
    };
    let source_token_change = before_source_balance
        .checked_sub(after_source_balance)
        .ok_or(ErrorCode::CalculationError)?;

    destination_token_account.reload()?;
    let after_destination_balance = destination_token_account.amount;
    let destination_token_change = after_destination_balance
        .checked_sub(before_destination_balance)
        .ok_or(ErrorCode::CalculationError)?;

    log_swap_end(
        after_source_balance,
        after_destination_balance,
        source_token_change,
        destination_token_change,
    );

    // Check min return
    require!(destination_token_change >= min_return, ErrorCode::MinReturnNotReached);
    Ok(destination_token_change)
}

pub fn common_swap_v3<'info, T: PlatformFeeV3Processor<'info>>(
    swap_processor: &T,
    payer: &AccountInfo<'info>,
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
    // COMMISSION
    commission_rate: u32,
    commission_direction: bool,
    commission_account: &Option<AccountInfo<'info>>,
    // PLATFORM FEE
    platform_fee_rate: Option<u16>,
    platform_fee_account: &Option<AccountInfo<'info>>,
    // TRIM
    trim_rate: Option<u8>,
    charge_rate: Option<u16>,
    trim_account: Option<&AccountInfo<'info>>,
    charge_account: Option<&AccountInfo<'info>>,
    acc_close_flag: bool,
) -> Result<u64> {
    log_swap_basic_info(
        order_id,
        &source_mint.key(),
        &destination_mint.key(),
        &source_token_account.owner,
        &destination_token_account.owner,
    );

    let before_source_balance = source_token_account.amount;
    let before_destination_balance = destination_token_account.amount;
    let min_return = args.min_return;

    log_swap_balance_before(
        before_source_balance,
        before_destination_balance,
        args.amount_in,
        args.expect_amount_out,
        min_return,
    );

    // Verify sa_authority is valid
    if sa_authority.is_some() {
        require!(
            sa_authority.as_ref().unwrap().key() == authority_pda::ID,
            ErrorCode::InvalidSaAuthority
        );
    }

    // get swap accounts
    let (mut source_account, mut destination_account) = swap_processor.get_swap_accounts(
        payer,
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
    )?;

    // before swap hook
    let real_amount_in = swap_processor.before_swap(
        payer,
        sa_authority,
        source_token_account,
        source_mint,
        source_token_sa,
        source_token_program,
        args.amount_in,
        commission_rate,
        commission_direction,
        commission_account,
        platform_fee_rate,
        platform_fee_account,
    )?;

    // Common swap
    let expected_amount_out = args.expect_amount_out;
    let amount_out = execute_swap(
        &mut source_account,
        &mut destination_account,
        remaining_accounts,
        args,
        real_amount_in,
        order_id,
        source_token_sa.is_some(),
        None,
        Some(payer),
    )?;

    // after swap hook
    let actual_amount_out = swap_processor.after_swap(
        payer,
        sa_authority,
        destination_token_account,
        destination_mint,
        destination_token_sa,
        destination_token_program,
        expected_amount_out,
        amount_out,
        commission_rate,
        commission_direction,
        commission_account,
        platform_fee_rate,
        platform_fee_account,
        trim_rate,
        charge_rate,
        trim_account,
        charge_account,
        acc_close_flag,
    )?;

    // source token account has been closed in pumpfun buy
    let after_source_balance = if source_token_account.get_lamports() != 0 {
        source_token_account.reload()?;
        source_token_account.amount
    } else {
        0
    };
    let source_token_change = before_source_balance
        .checked_sub(after_source_balance)
        .ok_or(ErrorCode::CalculationError)?;

    // destination token account has been closed in swap_tob_processor
    let (after_destination_balance, destination_token_change) =
        if destination_token_account.get_lamports() != 0 {
            destination_token_account.reload()?;
            let after_destination_balance = destination_token_account.amount;
            (
                after_destination_balance,
                after_destination_balance
                    .checked_sub(before_destination_balance)
                    .ok_or(ErrorCode::CalculationError)?,
            )
        } else {
            (actual_amount_out, actual_amount_out)
        };

    log_swap_end(
        after_source_balance,
        after_destination_balance,
        source_token_change,
        destination_token_change,
    );

    // Check min return
    require!(destination_token_change >= min_return, ErrorCode::MinReturnNotReached);
    Ok(destination_token_change)
}

fn execute_swap<'info>(
    source_account: &mut InterfaceAccount<'info, TokenAccount>,
    destination_account: &mut InterfaceAccount<'info, TokenAccount>,
    remaining_accounts: &'info [AccountInfo<'info>],
    args: SwapArgs,
    real_amount_in: u64,
    order_id: u64,
    proxy_from: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
    payer: Option<&AccountInfo<'info>>,
) -> Result<u64> {
    destination_account.reload()?;
    let before_destination_balance = destination_account.amount;

    // Check SwapArgs
    let SwapArgs { amount_in: _, min_return, expect_amount_out, amounts, routes } = &args;
    require!(real_amount_in > 0, ErrorCode::AmountInMustBeGreaterThanZero);
    require!(*min_return > 0, ErrorCode::MinReturnMustBeGreaterThanZero);
    require!(*expect_amount_out >= *min_return, ErrorCode::InvalidExpectAmountOut);
    require!(amounts.len() == routes.len(), ErrorCode::AmountsAndRoutesMustHaveTheSameLength);

    let total_amounts: u64 = amounts
        .iter()
        .try_fold(0u64, |acc, &x| acc.checked_add(x).ok_or(ErrorCode::CalculationError))?;
    require!(total_amounts == real_amount_in, ErrorCode::TotalAmountsMustBeEqualToAmountIn);

    // Swap by Routes
    let mut offset: usize = 0;
    // Level 1 split handling
    for (i, hops) in routes.iter().enumerate() {
        require!(hops.len() <= MAX_HOPS, ErrorCode::TooManyHops);
        let mut amount_in = amounts[i];

        // Multi-hop handling
        let mut last_to_account = ZERO_ADDRESS;
        for (hop, route) in hops.iter().enumerate() {
            let dexes = &route.dexes;
            let weights = &route.weights;
            require!(dexes.len() == weights.len(), ErrorCode::DexesAndWeightsMustHaveTheSameLength);
            let total_weight: u8 = weights
                .iter()
                .try_fold(0u8, |acc, &x| acc.checked_add(x).ok_or(ErrorCode::CalculationError))?;
            require!(total_weight == TOTAL_WEIGHT, ErrorCode::WeightsMustSumTo100);

            // Level 2 split handling
            let mut hop_accounts = HopAccounts {
                last_to_account,
                from_account: ZERO_ADDRESS,
                to_account: ZERO_ADDRESS,
            };
            let mut amount_out: u64 = 0;
            let mut acc_fork_in: u64 = 0;
            for (index, dex) in dexes.iter().enumerate() {
                // Calculate 2 level split amount
                let fork_amount_in = if index == dexes.len() - 1 {
                    // The last dex, use the remaining amount_in for trading to prevent accumulation
                    amount_in.checked_sub(acc_fork_in).ok_or(ErrorCode::CalculationError)?
                } else {
                    let temp_amount = amount_in
                        .checked_mul(weights[index] as u64)
                        .ok_or(ErrorCode::CalculationError)?
                        .checked_div(TOTAL_WEIGHT as u64)
                        .ok_or(ErrorCode::CalculationError)?;
                    acc_fork_in =
                        acc_fork_in.checked_add(temp_amount).ok_or(ErrorCode::CalculationError)?;
                    temp_amount
                };

                // Execute swap
                let fork_amount_out = distribute_swap(
                    dex,
                    remaining_accounts,
                    fork_amount_in,
                    &mut offset,
                    &mut hop_accounts,
                    hop,
                    proxy_from,
                    order_id,
                    owner_seeds,
                    payer,
                )?;

                // Emit SwapEvent
                let event =
                    SwapEvent { dex: *dex, amount_in: fork_amount_in, amount_out: fork_amount_out };
                emit!(event);
                msg!("{:?}", event);
                hop_accounts.from_account.log();
                hop_accounts.to_account.log();

                amount_out =
                    amount_out.checked_add(fork_amount_out).ok_or(ErrorCode::CalculationError)?;
            }

            if hop == 0 {
                // CHECK: Verify the first hop's from_token must be consistent with ctx.accounts.source_token_account
                require!(
                    source_account.key() == hop_accounts.from_account,
                    ErrorCode::InvalidSourceTokenAccount
                );
            }
            if hop == hops.len() - 1 {
                // CHECK: Verify the last hop's to_account must be consistent with ctx.accounts.destination_token_account
                require!(
                    destination_account.key() == hop_accounts.to_account,
                    ErrorCode::InvalidDestinationTokenAccount
                );
            }
            amount_in = amount_out;
            last_to_account = hop_accounts.to_account;
        }
    }

    destination_account.reload()?;
    let after_destination_balance = destination_account.amount;
    let amount_out = after_destination_balance
        .checked_sub(before_destination_balance)
        .ok_or(ErrorCode::CalculationError)?;
    Ok(amount_out)
}

fn distribute_swap<'a>(
    dex: &Dex,
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_from: bool,
    order_id: u64,
    owner_seeds: Option<&[&[&[u8]]]>,
    payer: Option<&AccountInfo<'a>>,
) -> Result<u64> {
    let swap_function = match dex {
        Dex::SplTokenSwap => spl_token_swap::swap,
        Dex::StableSwap => stable_swap::swap,
        Dex::Whirlpool => whirlpool::swap,
        Dex::MeteoraDynamicpool => meteora::swap,
        Dex::RaydiumSwap => raydium::swap,
        Dex::RaydiumStableSwap => raydium::swap_stable,
        Dex::RaydiumClmmSwap => raydium::swap_clmm,
        Dex::RaydiumClmmSwapV2 => raydium::swap_clmm_v2,
        Dex::AldrinExchangeV1 => aldrin::swap_v1,
        Dex::AldrinExchangeV2 => aldrin::swap_v2,
        Dex::LifinityV1 => lifinity::swap_v1,
        Dex::LifinityV2 => lifinity::swap_v2,
        Dex::FluxBeam => fluxbeam::swap,
        Dex::MeteoraDlmm => meteora::dlmm_swap,
        Dex::RaydiumCpmmSwap => raydium::swap_cpmm,
        Dex::OpenBookV2 => openbookv2::place_take_order,
        Dex::WhirlpoolV2 => whirlpool::swap_v2,
        Dex::Phoenix => phoenix::swap,
        Dex::ObricV2 => obric_v2::swap,
        Dex::SanctumAddLiq => sanctum::add_liquidity_handler,
        Dex::SanctumRemoveLiq => sanctum::remove_liquidity_handler,
        Dex::SanctumNonWsolSwap => sanctum::swap_without_wsol_handler,
        Dex::SanctumWsolSwap => sanctum::swap_with_wsol_handler,
        Dex::PumpfunBuy => pumpfun::buy,
        Dex::PumpfunSell => pumpfun::sell,
        Dex::Saros => saros::swap,
        Dex::StabbleSwap => stabble::swap,
        Dex::SanctumRouter => {
            return sanctum_router::sanctum_router_handler(
                remaining_accounts,
                amount_in,
                offset,
                hop_accounts,
                hop,
                proxy_from,
                order_id,
                owner_seeds,
            );
        }
        Dex::MeteoraVaultDeposit => meteora::deposit,
        Dex::MeteoraVaultWithdraw => meteora::withdraw,
        Dex::MeteoraLst => meteora::swap_lst,
        Dex::Solfi => solfi::swap,
        Dex::QualiaSwap => qualia::swap,
        Dex::Zerofi => zerofi::swap,
        Dex::PumpfunammBuy => pumpfunamm::buy,
        Dex::PumpfunammSell => pumpfunamm::sell,
        Dex::Virtuals => virtuals::swap,
        Dex::VertigoBuy => vertigo::buy,
        Dex::VertigoSell => vertigo::sell,
        Dex::PerpetualsAddLiq => {
            return perpetuals::liquidity_handler(
                remaining_accounts,
                amount_in,
                offset,
                hop_accounts,
                hop,
                proxy_from,
                true,
                owner_seeds,
            );
        }
        Dex::PerpetualsRemoveLiq => {
            return perpetuals::liquidity_handler(
                remaining_accounts,
                amount_in,
                offset,
                hop_accounts,
                hop,
                proxy_from,
                false,
                owner_seeds,
            );
        }
        Dex::PerpetualsSwap => perpetuals::perpetuals_swap_handler,
        Dex::RaydiumLaunchpad => {
            return raydium_launchpad::launchpad_handler(
                remaining_accounts,
                amount_in,
                offset,
                hop_accounts,
                hop,
                proxy_from,
                owner_seeds,
                "RaydiumLaunchpad",
            );
        }
        Dex::LetsBonkFun => {
            return raydium_launchpad::launchpad_handler(
                remaining_accounts,
                amount_in,
                offset,
                hop_accounts,
                hop,
                proxy_from,
                owner_seeds,
                "LetsBonkFun",
            );
        }
        Dex::Woofi => woofi::swap,
        Dex::MeteoraDbc => meteora_dbc::swap,
        Dex::MeteoraDlmmSwap2 => meteora::dlmm_swap2,
        Dex::MeteoraDAMMV2 => meteora::damm_v2_swap,
        Dex::Gavel => gavel::swap,
        Dex::BoopfunBuy => {
            return boopfun::buy(
                remaining_accounts,
                amount_in,
                offset,
                hop_accounts,
                hop,
                proxy_from,
                owner_seeds,
                payer,
            );
        }
        Dex::BoopfunSell => boopfun::sell,
        Dex::MeteoraDbc2 => meteora_dbc::swap2,
        Dex::GooseFX => goosefx::swap,
        Dex::Dooar => dooar::swap,
        Dex::Numeraire => numeraire::swap,
        Dex::SaberDecimalWrapperDeposit => saber_decimal_wrapper::deposit,
        Dex::SaberDecimalWrapperWithdraw => saber_decimal_wrapper::withdraw,
        Dex::SarosDlmm => saros::dlmm_swap,
        Dex::OneDexSwap => one_dex::swap,
        Dex::Manifest => manifest::swap,
        Dex::ByrealClmm => byreal_clmm::swap_v2,
        Dex::PancakeSwapV3Swap => pancake_swap_v3::swap,
        Dex::PancakeSwapV3SwapV2 => pancake_swap_v3::swap_v2,
        Dex::Tessera => tessera::swap,
        Dex::SolRfq {
            rfq_id,
            expected_maker_amount,
            expected_taker_amount,
            maker_send_amount,
            taker_send_amount,
            expiry,
            maker_use_native_sol,
            taker_use_native_sol,
        } => {
            return sol_rfq::fill_order(
                remaining_accounts,
                amount_in,
                offset,
                hop_accounts,
                hop,
                proxy_from,
                owner_seeds,
                *rfq_id,
                *expected_maker_amount,
                *expected_taker_amount,
                *maker_send_amount,
                *taker_send_amount,
                *expiry,
                *maker_use_native_sol,
                *taker_use_native_sol,
            );
        }
        Dex::PumpfunBuy2 => {
            return pumpfun::buy2(
                remaining_accounts,
                amount_in,
                offset,
                hop_accounts,
                hop,
                proxy_from,
                owner_seeds,
                payer,
            );
        }
        Dex::PumpfunammBuy2 => pumpfunamm::buy2,
        Dex::Humidifi => humidifi::swap,
        Dex::HeavenBuy => heaven::buy,
        Dex::HeavenSell => heaven::sell,
        Dex::SolfiV2 => solfi::swap_v2,
        Dex::PumpfunBuy3 => {
            return pumpfun::buy3(
                remaining_accounts,
                amount_in,
                offset,
                hop_accounts,
                hop,
                proxy_from,
                owner_seeds,
                payer,
            );
        }
        Dex::PumpfunSell3 => {
            return pumpfun::sell3(
                remaining_accounts,
                amount_in,
                offset,
                hop_accounts,
                hop,
                proxy_from,
                owner_seeds,
                payer,
            );
        }
        Dex::PumpfunammBuy3 => pumpfunamm::buy3,
        Dex::PumpfunammSell3 => {
            return pumpfunamm::sell3(
                remaining_accounts,
                amount_in,
                offset,
                hop_accounts,
                hop,
                proxy_from,
                owner_seeds,
                payer,
            );
        }
        Dex::Goonfi => goonfi::swap,
        Dex::MoonitBuy => {
            return moonit::buy(
                remaining_accounts,
                amount_in,
                offset,
                hop_accounts,
                hop,
                proxy_from,
                owner_seeds,
                payer,
            );
        }
        Dex::MoonitSell => moonit::sell,
        Dex::RaydiumSwapV2 => raydium::swap_v2,
        Dex::Whalestreet => whalestreet::swap,
        Dex::SugarMoneyBuy { bonding_curve_bump, bonding_curve_sol_associated_account_bump } => {
            return sugar_money::buy(
                remaining_accounts,
                amount_in,
                offset,
                hop_accounts,
                hop,
                proxy_from,
                owner_seeds,
                payer,
                *bonding_curve_bump,
                *bonding_curve_sol_associated_account_bump,
            );
        }
        Dex::SugarMoneySell { bonding_curve_bump, bonding_curve_sol_associated_account_bump } => {
            return sugar_money::sell(
                remaining_accounts,
                amount_in,
                offset,
                hop_accounts,
                hop,
                proxy_from,
                owner_seeds,
                *bonding_curve_bump,
                *bonding_curve_sol_associated_account_bump,
            );
        }
        Dex::MeteoraDAMMV2Swap2 => meteora::damm_v2_swap2,
        Dex::AlphaQ => alphaq::swap,
        Dex::FutarchyAmm => futarchy::swap,
        Dex::PumpfunSell2 => {
            return pumpfun::sell2(
                remaining_accounts,
                amount_in,
                offset,
                hop_accounts,
                hop,
                proxy_from,
                owner_seeds,
                payer,
            );
        }
    };
    swap_function(remaining_accounts, amount_in, offset, hop_accounts, hop, proxy_from, owner_seeds)
}
