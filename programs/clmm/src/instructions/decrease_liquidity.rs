use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer};
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::states::*;
use crate::utils::ErrorCode;
use crate::utils::math::*;

#[derive(Accounts)]
pub struct DecreaseLiquidity<'info> {
    #[account(
        mut,
        has_one = token_mint_0,
        has_one = token_mint_1,
    )]
    pub pool: Account<'info, Pool>,

    #[account()]
    pub lower_tick_array: Account<'info, TickArray>,

    #[account()]
    pub upper_tick_array: Account<'info, TickArray>,

    #[account(
        constraint = position.pool == pool.key() @ ErrorCode::InvalidPositionRange,
        constraint = position.owner == payer.key() @ ErrorCode::InvalidPositionOwner,
    )]
    pub position: Account<'info, Position>,

    #[account(
        mut,
        token::mint = token_mint_0
    )]
    pub user_token_0: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = token_mint_1
    )]
    pub user_token_1: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = token_mint_0
    )]
    pub pool_token_0: Account<'info, TokenAccount>,

    #[account(
        mut, 
        token::mint = token_mint_1
    )]
    pub pool_token_1: Account<'info, TokenAccount>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_mint_0: Account<'info, Mint>,
    pub token_mint_1: Account<'info, Mint>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn decrease_liquidity(
    ctx:Context<DecreaseLiquidity>,
    liquidity_amount: u128,
    lower_tick: i32,
    upper_tick: i32,
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

    position.liquidity = position.liquidity.checked_sub(liquidity_amount).ok_or(ErrorCode::ArithmeticOverflow)?;

    let (amount_0, amount_1) = get_amounts_for_liquidity(
        pool.sqrt_price_x96,
        get_sqrt_price_from_tick(lower_tick)?,
        get_sqrt_price_from_tick(upper_tick)?,
        liquidity_amount,
    )?;


    pool.global_liquidity = pool
        .global_liquidity
        .checked_sub(liquidity_amount)
        .ok_or(ErrorCode::ArithmeticOverflow)?;

    if amount_0 > 0 {
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"pool".as_ref(),
            pool.token_mint_0.as_ref(),
            pool.token_mint_1.as_ref(),
            &pool.tick_spacing.to_le_bytes(),
            &[pool.bump],
        ]];

        let ctx = CpiContext:: new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.pool_token_0.to_account_info(),
                to: ctx.accounts.user_token_0.to_account_info(),
                authority: pool.to_account_info()
            },
            signer_seeds,
        );

        token::transfer(ctx, amount_0)?;
    }

    if amount_1 > 0 {
        let signer_seeds: &[&[&[u8]]]  = &[&[
            b"pool".as_ref(),
            pool.token_mint_0.as_ref(),
            pool.token_mint_1.as_ref(),
            &pool.tick_spacing.to_le_bytes(),
            &[pool.bump],
        ]];

        let context = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(), 
            Transfer {
                from:ctx.accounts.pool_token_1.to_account_info(),
                to: ctx.accounts.user_token_1.to_account_info(),
                authority: pool.to_account_info(),
            }, 
            signer_seeds,
        );
        token::transfer(context, amount_1)?;
    }

    Ok((amount_0, amount_1))
}   
