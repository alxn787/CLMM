use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::states::*;
use crate::utils::ErrorCode;

#[derive(Accounts)]
#[instruction(lower_tick: i32, upper_tick: i32, liquidity_amount: u128)]
pub struct AddLiquidity<'info> {
    #[account(
        mut,
        has_one = token_mint_0,
        has_one = token_mint_1,
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        mut,
        constraint = lower_tick_array.key() == Pubkey::find_program_address(
            &[
                b"tick_array".as_ref(),
                pool.key().as_ref(),
                &TickArray::get_starting_tick_index(lower_tick, pool.tick_spacing).to_le_bytes()
            ],
            &crate::ID
        ).0 @ ErrorCode::InvalidTickArrayAccount
    )]
    pub lower_tick_array: Account<'info, TickArray>,

    #[account(
        mut,
        constraint = upper_tick_array.key() == Pubkey::find_program_address(
            &[
                b"tick_array".as_ref(),
                pool.key().as_ref(),
                &TickArray::get_starting_tick_index(upper_tick, pool.tick_spacing).to_le_bytes()
            ],
            &crate::ID
        ).0 @ ErrorCode::InvalidTickArrayAccount
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