pub mod states;
pub mod utils;
pub mod instructions;

use crate::instructions::*;
use anchor_lang::prelude::*;

declare_id!("4GhrgMYusqS5uuyzrrBvFv3FuVGp4RRp4XKDBctyW6oN");

#[program]
pub mod clmm {
    use super::*;

    pub fn initialize_pool(
        ctx: Context<InitializePool>,
        tick_spacing: i32,
        initial_sqrt_price: u128,
    ) -> Result<()> {
        instructions::pool::init_pool(ctx, tick_spacing, initial_sqrt_price)
    }


    pub fn open_position(
        ctx: Context<OpenPosition>, 
        owner: Pubkey,
        lower_tick: i32, 
        upper_tick: i32, 
        liquidity_amount: u128,
        _tick_array_lower_start_index: i32,
        _tick_array_upper_start_index: i32

    ) -> Result<(u64, u64)> {
       instructions::open_position::open_position(ctx, owner, lower_tick, upper_tick, liquidity_amount, _tick_array_lower_start_index, _tick_array_upper_start_index)
    }

    pub fn increase_liquidity(
        ctx: Context<IncreaseLiquidity>,
        lower_tick: i32,
        upper_tick: i32,
        liquidity_amount: u128
    ) -> Result<(u64,u64)>{
        instructions::increase_liquidity::increase_liquidity(ctx, liquidity_amount, lower_tick, upper_tick)
    }
    pub fn decrease_liquiduty(
        ctx: Context<DecreaseLiquidity>,
        lower_tick: i32,
        upper_tick: i32,
        liquidity_amount: u128
    ) -> Result<(u64,u64)>{
        instructions::decrease_liquidity::decrease_liquidity(ctx, liquidity_amount, lower_tick, upper_tick)
    }

    pub fn swap(ctx: Context<Swap>, amount_in: u64, swap_token_0_for_1: bool, amount_out_minimum: u64) -> Result<u64> {
        instructions::swap::swap(ctx, amount_in, swap_token_0_for_1, amount_out_minimum)    
    }


}

