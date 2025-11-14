use anchor_lang::prelude::*;

pub fn log_swap_basic_info(
    order_id: u64,
    source_mint: &Pubkey,
    destination_mint: &Pubkey,
    source_owner: &Pubkey,
    destination_owner: &Pubkey,
) {
    if order_id > 0 {
        msg!("order_id: {}", order_id);
    }
    source_mint.log();
    destination_mint.log();
    source_owner.log();
    destination_owner.log();
}

pub fn log_swap_balance_before(
    before_source_balance: u64,
    before_destination_balance: u64,
    amount_in: u64,
    expect_amount_out: u64,
    min_return: u64,
) {
    msg!(
        "before_source_balance: {}, before_destination_balance: {}, amount_in: {}, expect_amount_out: {}, min_return: {}",
        before_source_balance,
        before_destination_balance,
        amount_in,
        expect_amount_out,
        min_return
    );
}

pub fn log_swap_end(
    after_source_balance: u64,
    after_destination_balance: u64,
    source_token_change: u64,
    destination_token_change: u64,
) {
    msg!(
        "after_source_balance: {}, after_destination_balance: {}, source_token_change: {}, destination_token_change: {}",
        after_source_balance,
        after_destination_balance,
        source_token_change,
        destination_token_change
    );
}

pub fn log_commission_info(commission_direction: bool, commission_amount: u64, adjust_amount: u64) {
    msg!(
        "commission_direction: {:?}, commission_amount: {:?}, commission_adjust_amount: {:?}",
        commission_direction,
        commission_amount,
        adjust_amount
    );
}

pub fn log_platform_fee_info(amount: u64, adjust_amount: u64, fee_account: &Pubkey) {
    msg!("platform_fee_amount: {:?}, platform_fee_adjust_amount: {:?}", amount, adjust_amount);
    fee_account.log();
}

pub fn log_trim_fee_info(amount: u64, adjust_amount: u64, fee_account: &Pubkey) {
    msg!("trim_fee_amount: {:?}, trim_fee_adjust_amount: {:?}", amount, adjust_amount);
    fee_account.log();
}

pub fn log_charge_fee_info(amount: u64, adjust_amount: u64, fee_account: &Pubkey) {
    msg!("charge_fee_amount: {:?}, charge_fee_adjust_amount: {:?}", amount, adjust_amount);
    fee_account.log();
}

pub fn log_rate_info(commission_rate: u32, platform_fee_rate: u32, trim_rate: Option<u8>) {
    if let Some(trim_rate) = trim_rate {
        msg!(
            "commission_rate: {:?}, platform_fee_rate: {:?}, trim_rate: {:?}",
            commission_rate,
            platform_fee_rate,
            trim_rate
        );
    } else {
        msg!("commission_rate: {:?}, platform_fee_rate: {:?}", commission_rate, platform_fee_rate);
    }
}

pub fn log_rate_info_v3(
    commission_rate: u32,
    platform_fee_rate: Option<u16>,
    trim_rate: Option<u8>,
    commission_direction: bool,
    acc_close_flag: bool,
) {
    let platform_fee_rate_val = platform_fee_rate.unwrap_or(0);
    let trim_rate_val = trim_rate.unwrap_or(0);
    msg!(
        "commission_rate: {:?}, platform_fee_rate: {:?}, trim_rate: {:?}, commission_direction: {:?}, acc_close_flag: {:?}",
        commission_rate,
        platform_fee_rate_val,
        trim_rate_val,
        commission_direction,
        acc_close_flag
    );
}

pub fn log_rate_info_v3_enhanced(
    commission_rate: u32,
    platform_fee_rate: Option<u16>,
    trim_rate: u8,
    charge_rate: u16,
    commission_direction: bool,
    acc_close_flag: bool,
) {
    let platform_fee_rate_val = platform_fee_rate.unwrap_or(0);
    msg!(
        "commission_rate: {:?}, platform_fee_rate: {:?}, trim_rate: {:?}, charge_rate: {:?}, commission_direction: {:?}, acc_close_flag: {:?}",
        commission_rate,
        platform_fee_rate_val,
        trim_rate,
        charge_rate,
        commission_direction,
        acc_close_flag
    );
}

pub fn log_claim_info_before(source_balance: u64, destination_balance: u64, amount: u64) {
    msg!(
        "before_source_balance: {:?}, before_destination_balance: {:?}, amount: {:?}",
        source_balance,
        destination_balance,
        amount
    );
}

pub fn log_claim_info_after(
    source_balance: u64,
    destination_balance: u64,
    source_token_change: u64,
    destination_token_change: u64,
) {
    msg!(
        "after_source_balance: {:?}, after_destination_balance: {:?}, source_token_change: {:?}, destination_token_change: {:?}",
        source_balance,
        destination_balance,
        source_token_change,
        destination_token_change
    );
}

pub fn log_sa_lamports_info(
    before_sa_lamports: u64,
    after_sa_lamports: u64,
    diff_sa_lamports: u64,
) {
    msg!(
        "before_sa_lamports: {:?}, after_sa_lamports: {:?}, diff_sa_lamports: {:?}",
        before_sa_lamports,
        after_sa_lamports,
        diff_sa_lamports
    );
}
