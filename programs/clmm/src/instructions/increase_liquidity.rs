use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::states::*;

#[derive(Accounts)]
pub struct IncreaseLiquidity<'info> {
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
        constraint = position.pool == pool.key() @ ErrorCode::InvalidProgramExecutable,
        constraint = position.owner == payer.key() @ ErrorCode::InvalidProgramExecutable,
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