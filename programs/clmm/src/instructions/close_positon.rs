use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer};
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::states::*;
use crate::utils::ErrorCode;
use crate::utils::math::*;

#[derive(Accounts)]
#[instruction(lower_tick: i32, upper_tick: i32, tick_array_lower_start_index: i32, tick_array_upper_start_index: i32)]
pub struct ClosePosition<'info> {
    #[account(mut)]
    pub pool: Account<'info, Pool>,

    #[account(
        mut,
        seeds = [
            b"tick_array",
            pool.key().as_ref(),
            &tick_array_lower_start_index.to_le_bytes()
        ],
        bump
    )]
    pub lower_tick_array: Account<'info, TickArray>,

    #[account(
        mut,
        seeds = [
            b"tick_array",
            pool.key().as_ref(),
            &tick_array_upper_start_index.to_le_bytes()
        ],
        bump
    )]
    pub upper_tick_array: Account<'info, TickArray>,

    #[account(
        mut,
        close = owner,
        seeds = [
            b"position",
            owner.key().as_ref(),
            pool.key().as_ref(),
            &lower_tick.to_le_bytes(),
            &upper_tick.to_le_bytes(),
        ],
        bump = position.bump,
        has_one = owner,
        has_one = pool,
    )]
    pub position: Account<'info, Position>,

    #[account(mut)]
    pub user_token_0: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_token_1: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = pool_token_0.mint == pool.token_mint_0
    )]
    pub pool_token_0: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = pool_token_1.mint == pool.token_mint_1
    )]
    pub pool_token_1: Account<'info, TokenAccount>,

    #[account(mut)]
    pub owner: Signer<'info>,
    
    pub token_program: Program<'info, Token>,
}


pub fn close_position(
    ctx: Context<ClosePosition>,
    lower_tick: i32,
    upper_tick: i32,
    _tick_array_lower_start_index: i32,
    _tick_array_upper_start_index: i32
) -> Result<(u64, u64)> {
    let pool = &mut ctx.accounts.pool;
    let position = &ctx.accounts.position;

    let liquidity_to_remove = position.liquidity;
    require!(liquidity_to_remove > 0, ErrorCode::NoLiquidityToRemove);

    let (amount_0, amount_1) = get_amounts_for_liquidity(
        pool.sqrt_price_x96,
        get_sqrt_price_from_tick(lower_tick)?,
        get_sqrt_price_from_tick(upper_tick)?,
        liquidity_to_remove,
    )?;

    let lower_tick_array = &mut ctx.accounts.lower_tick_array;
    let upper_tick_array = &mut ctx.accounts.upper_tick_array;

    let lower_tick_info =
        lower_tick_array.get_tick_info_mutable(lower_tick, pool.tick_spacing)?;
    lower_tick_info.update_liquidity(-(liquidity_to_remove as i128), true)?;

    let upper_tick_info =
        upper_tick_array.get_tick_info_mutable(upper_tick, pool.tick_spacing)?;
    upper_tick_info.update_liquidity(-(liquidity_to_remove as i128), false)?;

    pool.global_liquidity = pool
        .global_liquidity
        .checked_sub(liquidity_to_remove)
        .ok_or(ErrorCode::ArithmeticOverflow)?;

    let pool_seeds = &[
        b"pool".as_ref(),
        pool.token_mint_0.as_ref(),
        pool.token_mint_1.as_ref(),
        &[pool.bump],
    ];
    let signer_seeds = &[&pool_seeds[..]];

    if amount_0 > 0 {
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
            amount_0,
        )?;
    }

    if amount_1 > 0 {
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
            amount_1,
        )?;
    }

    Ok((amount_0, amount_1))
}