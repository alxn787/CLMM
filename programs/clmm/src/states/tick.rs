use anchor_lang::prelude::*;
use crate::utils::ErrorCode;

#[account]
pub struct TickInfo {
    pub initialized: bool,
    pub liquidity_gross: u128,
    pub liquidity_net: i128,
}

impl TickInfo {

    pub const SPACE: usize = 
        8 + // discriminator
        16 + // liquidity_gross
        16 + // liquidity_net
        1;   // initialized

    pub fn update_liquidity(&mut self, liquidity_delta: i128, is_lower: bool) -> Result<()> {
        if !self.initialized {
            self.initialized = true;
        }
        self.liquidity_gross = self
            .liquidity_gross
            .checked_add(liquidity_delta.unsigned_abs())
            .ok_or(ErrorCode::ArithmeticOverflow)?;
        if is_lower {
            self.liquidity_net = self
                .liquidity_net
                .checked_add(liquidity_delta)
                .ok_or(ErrorCode::ArithmeticOverflow)?;
        } else {
            self.liquidity_net = self
                .liquidity_net
                .checked_sub(liquidity_delta)
                .ok_or(ErrorCode::ArithmeticOverflow)?;
        }
        Ok(())
    }
}

pub const TICKS_PER_ARRAY: usize = 30;

#[account]
pub struct TickArray {
    pub pool: Pubkey,
    pub starting_tick: i32,
    pub ticks: [TickInfo; TICKS_PER_ARRAY],
    pub bump: u8,
}

impl TickArray {

    pub const SPACE: usize = 8 + // discriminator
        32 + // pool
        4 +  // starting_tick
        TICKS_PER_ARRAY * 48 + // ticks
        1;   // bump

    pub fn get_starting_tick_index(tick: i32, tick_spacing: i32) -> i32 {
        let ticks_per_array_i32 = TICKS_PER_ARRAY as i32;
        let array_idx = tick
            .checked_div(tick_spacing)
            .expect("Div by zero: tick_spacing")
            .checked_div(ticks_per_array_i32)
            .expect("Div by zero: TICKS_PER_ARRAY");
        array_idx
            .checked_mul(ticks_per_array_i32)
            .expect("Mul overflow")
            .checked_mul(tick_spacing)
            .expect("Mul overflow")
    }
    pub fn get_tick_info_mutable(&mut self, tick: i32, tick_spacing: i32) -> Result<&mut TickInfo> {
        let ticks_per_array_i32 = TICKS_PER_ARRAY as i32;
        let offset = (tick
            .checked_div(tick_spacing)
            .ok_or(ErrorCode::ArithmeticOverflow)?)
        .checked_sub(
            self.starting_tick
                .checked_div(tick_spacing)
                .ok_or(ErrorCode::ArithmeticOverflow)?,
        )
        .ok_or(ErrorCode::ArithmeticOverflow)?
        .checked_rem(ticks_per_array_i32)
        .ok_or(ErrorCode::ArithmeticOverflow)? as usize;
        Ok(&mut self.ticks[offset])
    }
}
