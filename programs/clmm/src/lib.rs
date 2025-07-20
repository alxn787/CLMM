pub mod states;
pub mod utils;
pub mod instructions;

use crate::instructions::*;
use crate::utils::ErrorCode;
use anchor_lang::prelude::*;

declare_id!("4GhrgMYusqS5uuyzrrBvFv3FuVGp4RRp4XKDBctyW6oN");

#[program]
pub mod clmm {
    use anchor_spl::token::{self, Transfer};
    use super::*;

    pub fn initialize_pool(
        ctx: Context<InitializePool>,
        tick_spacing: i32,
        initial_sqrt_price: u128,
    ) -> Result<()> {
        let pool = &mut ctx.accounts.pool;
        require!(tick_spacing > 0, ErrorCode::InvalidTickSpacing);
        require!(
            ctx.accounts.token_mint_0.key() != ctx.accounts.token_mint_1.key(),
            ErrorCode::InvalidTokenPair
        );

        pool.token_mint_0 = ctx.accounts.token_mint_0.key();
        pool.token_mint_1 = ctx.accounts.token_mint_1.key();
        pool.token_vault_0 = ctx.accounts.token_vault_0.key();
        pool.token_vault_1 = ctx.accounts.token_vault_1.key();
        pool.global_liquidity = 0;
        pool.sqrt_price_x96 = initial_sqrt_price;
        pool.current_tick = get_tick_at_sqrt_price(initial_sqrt_price)?;
        pool.tick_spacing = tick_spacing;
        pool.bump = ctx.bumps.pool;
        Ok(())
    }

    pub fn create_tick_array(ctx: Context<CreateTickArray>, starting_tick: i32) -> Result<()> {
        let tick_array = &mut ctx.accounts.tick_array;
        tick_array.pool = ctx.accounts.pool.key();
        tick_array.starting_tick = starting_tick;
        tick_array.bump = ctx.bumps.tick_array;
        Ok(())
    }

    pub fn add_liquidity(
        ctx: Context<AddLiquidity>,
        owner: Pubkey,
        lower_tick: i32,
        upper_tick: i32,
        liquidity_amount: u128,
    ) -> Result<(u64, u64)> {
        let pool = &mut ctx.accounts.pool;
        let position = &mut ctx.accounts.position;

        require!(
            lower_tick < upper_tick
                && lower_tick % pool.tick_spacing == 0
                && upper_tick % pool.tick_spacing == 0,
            ErrorCode::InvalidTickRange
        );
        require!(liquidity_amount > 0, ErrorCode::InsufficientInputAmount);
        require!(
            pool.current_tick >= lower_tick && pool.current_tick < upper_tick,
            ErrorCode::MintRangeMustCoverCurrentPrice
        );

        let lower_tick_array = &mut ctx.accounts.lower_tick_array;
        let upper_tick_array = &mut ctx.accounts.upper_tick_array;

        let lower_tick_info =
            lower_tick_array.get_tick_info_mutable(lower_tick, pool.tick_spacing)?;
        let upper_tick_info =
            upper_tick_array.get_tick_info_mutable(upper_tick, pool.tick_spacing)?;

        lower_tick_info.update_liquidity(liquidity_amount as i128, true)?;
        upper_tick_info.update_liquidity(liquidity_amount as i128, false)?;

        let (amount_0, amount_1) = get_amounts_for_liquidity(
            pool.sqrt_price_x96,
            get_sqrt_price_from_tick(lower_tick)?,
            get_sqrt_price_from_tick(upper_tick)?,
            liquidity_amount,
        )?;

        if position.liquidity == 0 && position.owner == Pubkey::default() {
            position.owner = owner;
            position.pool = pool.key();
            position.tick_lower = lower_tick;
            position.tick_upper = upper_tick;
            position.liquidity = liquidity_amount;
            position.bump = ctx.bumps.position;
        } else {
            require!(position.owner == owner, ErrorCode::InvalidPositionOwner);
            require!(
                position.tick_lower == lower_tick && position.tick_upper == upper_tick,
                ErrorCode::InvalidPositionRange
            );
            position.liquidity = position
                .liquidity
                .checked_add(liquidity_amount)
                .ok_or(ErrorCode::ArithmeticOverflow)?;
        }

        pool.global_liquidity = pool
            .global_liquidity
            .checked_add(liquidity_amount)
            .ok_or(ErrorCode::ArithmeticOverflow)?;

        if amount_0 > 0 {
            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.user_token_0.to_account_info(),
                        to: ctx.accounts.pool_token_0.to_account_info(),
                        authority: ctx.accounts.payer.to_account_info(),
                    },
                ),
                amount_0,
            )?;
        }

        if amount_1 > 0 {
            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.user_token_1.to_account_info(),
                        to: ctx.accounts.pool_token_1.to_account_info(),
                        authority: ctx.accounts.payer.to_account_info(),
                    },
                ),
                amount_1,
            )?;
        }

        Ok((amount_0, amount_1))
    }

    pub fn swap(
        ctx: Context<Swap>,
        amount_in: u64,
        swap_token_0_for_1: bool,
        amount_out_minimum: u64,
    ) -> Result<u64> {
        let pool = &mut ctx.accounts.pool;

        require!(pool.global_liquidity > 0, ErrorCode::InsufficientPoolLiquidity);
        require!(amount_in > 0, ErrorCode::InsufficientInputAmount);

        let (amount_in_used, amount_out_calculated, new_sqrt_price_x96) = swap_segment(
            pool.sqrt_price_x96,
            pool.global_liquidity,
            amount_in,
            swap_token_0_for_1,
        )?;

        require!(
            amount_out_calculated >= amount_out_minimum,
            ErrorCode::SlippageExceeded
        );

        let signer_seeds: &[&[&[u8]]] = &[&[
            b"pool",
            pool.token_mint_0.as_ref(),
            pool.token_mint_1.as_ref(),
            &pool.tick_spacing.to_le_bytes(),
            &[pool.bump],
        ]];

        if swap_token_0_for_1 {
            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.user_token_0.to_account_info(),
                        to: ctx.accounts.pool_token_0.to_account_info(),
                        authority: ctx.accounts.payer.to_account_info(),
                    },
                ),
                amount_in_used,
            )?;

            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.pool_token_1.to_account_info(),
                        to: ctx.accounts.user_token_1.to_account_info(),
                        authority: pool.to_account_info(),
                    },
                    signer_seeds,
                ),
                amount_out_calculated,
            )?;
        } else {
            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.user_token_1.to_account_info(),
                        to: ctx.accounts.pool_token_1.to_account_info(),
                        authority: ctx.accounts.payer.to_account_info(),
                    },
                ),
                amount_in_used,
            )?;

            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.pool_token_0.to_account_info(),
                        to: ctx.accounts.user_token_0.to_account_info(),
                        authority: pool.to_account_info(),
                    },
                    signer_seeds,
                ),
                amount_out_calculated,
            )?;
        }

        pool.sqrt_price_x96 = new_sqrt_price_x96;
        pool.current_tick = get_tick_at_sqrt_price(new_sqrt_price_x96)?;

        Ok(amount_out_calculated)
    }
}







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