use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer};
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::states::*;
use crate::utils::ErrorCode;
use crate::utils::math::*;

#[derive(Accounts)]
#[instruction(lower_tick: i32, upper_tick: i32, liquidity_amount: u128, tick_array_lower_start_index: u32, tick_array_upper_start_index: u32)]
pub struct OpenPosition<'info> {
    #[account(
        mut,
        has_one = token_mint_0,
        has_one = token_mint_1,
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        init_if_needed,
        payer = payer,   
        space = TickArray::INIT_SPACE,  
        seeds = [
            b"tick_array",
            pool.key().as_ref(),
            &tick_array_lower_start_index.to_le_bytes()
        ],
        bump
    )]

    pub lower_tick_array: Account<'info, TickArray>,

    #[account(
    init_if_needed,
    payer = payer,   
    space = TickArray::INIT_SPACE,  
    seeds = [
        b"tick_array",
        pool.key().as_ref(),
        &tick_array_upper_start_index.to_le_bytes()
    ],
    bump
    )]
    pub upper_tick_array: Account<'info, TickArray>,

    #[account(
        init_if_needed,
        payer = payer,
        space = Position::INIT_SPACE,
        seeds = [
            b"position",
            payer.key().as_ref(),
            pool.key().as_ref(),
            &lower_tick.to_le_bytes(),
            &upper_tick.to_le_bytes(),
        ],
        bump,
    )]
    pub position: Account<'info, Position>,

    #[account(mut)]
    pub user_token_0: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_token_1: Account<'info, TokenAccount>,
    #[account(mut)]
    pub pool_token_0: Account<'info, TokenAccount>,
    #[account(mut)]
    pub pool_token_1: Account<'info, TokenAccount>,
    #[account(mut)]
    pub payer: Signer<'info>,
    
    pub token_mint_0: Account<'info, Mint>,
    pub token_mint_1: Account<'info, Mint>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

 pub fn open_position(
        ctx: Context<OpenPosition>,
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
