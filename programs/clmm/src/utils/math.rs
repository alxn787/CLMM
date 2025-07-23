use crate::utils::ErrorCode;
use anchor_lang::prelude::*;

pub fn get_sqrt_price_from_tick(tick: i32) -> Result<u128> {
    // This is a simplification; real math is logarithmic.
    let base_sqrt_price = 1u128 << 96;
    let adjustment_factor = 1_000_000_000 / 1000;
    let adjusted_price = base_sqrt_price
        .checked_add_signed((tick as i128) * (adjustment_factor as i128))
        .ok_or(ErrorCode::ArithmeticOverflow)?;
    Ok(adjusted_price)
}

pub fn get_tick_at_sqrt_price(sqrt_price_x96: u128) -> Result<i32> {
    let base_sqrt_price = 1u128 << 96;
    let adjustment_factor = 1_000_000_000 / 1000;

    let diff = sqrt_price_x96 as i128 - base_sqrt_price as i128;
    let tick = diff
        .checked_div(adjustment_factor as i128)
        .ok_or(ErrorCode::ArithmeticOverflow)? as i32;
    Ok(tick)
}

pub fn get_amounts_for_liquidity(
    current_sqrt_price_x96: u128,
    lower_sqrt_price_x96: u128,
    upper_sqrt_price_x96: u128,
    liquidity: u128,
) -> Result<(u64, u64)> {
    let amount0: u64;
    let amount1: u64;

    if current_sqrt_price_x96 >= lower_sqrt_price_x96 && current_sqrt_price_x96 < upper_sqrt_price_x96 {
        amount0 = (liquidity / 2) as u64;
        amount1 = (liquidity / 2) as u64;
    } else if current_sqrt_price_x96 < lower_sqrt_price_x96 {
        amount0 = liquidity as u64;
        amount1 = 0;
    } else {
        amount0 = 0;
        amount1 = liquidity as u64;
    }
    Ok((amount0, amount1))
}

pub fn swap_segment(
    current_sqrt_price_x96: u128,
    global_liquidity: u128,
    amount_remaining_in: u64,
    swap_token_0_for_1: bool,
) -> Result<(u64, u64, u128)> {
    if global_liquidity == 0 {
        return Err(ErrorCode::InsufficientPoolLiquidity.into());
    }

    let amount_in_used = amount_remaining_in;
    // This is a simplified calculation and does not represent a real AMM curve.
    let amount_out_calculated = amount_in_used
        .checked_sub(amount_in_used / 1000)
        .ok_or(ErrorCode::ArithmeticOverflow)?; // Simple 0.1% fee

    let new_sqrt_price = if swap_token_0_for_1 {
        current_sqrt_price_x96
            .checked_sub(1_000_000_000)
            .ok_or(ErrorCode::ArithmeticOverflow)?
    } else {
        // Swapping token 1 for 0, price of token 0 in terms of 1 increases.
        current_sqrt_price_x96
            .checked_add(1_000_000_000)
            .ok_or(ErrorCode::ArithmeticOverflow)?
    };

    Ok((amount_in_used, amount_out_calculated, new_sqrt_price))
}