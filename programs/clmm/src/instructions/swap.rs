use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::states::*;
use crate::utils::ErrorCode;

#[derive(Accounts)]
#[instruction(amount_in: u64, swap_token_0_for_1: bool, amount_out_minimum: u64)]
pub struct Swap<'info> {
    #[account(mut)]
    pub pool: Account<'info, Pool>,
    #[account(mut)]
    pub user_token_0: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_token_1: Account<'info, TokenAccount>,
    #[account(mut)]
    pub pool_token_0: Account<'info, TokenAccount>,
    #[account(mut)]
    pub pool_token_1: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = tick_array.key() == Pubkey::find_program_address(
            &[
                b"tick_array".as_ref(),
                pool.key().as_ref(),
                &TickArray::get_starting_tick_index(pool.current_tick, pool.tick_spacing).to_le_bytes()
            ],
            &crate::ID
        ).0 @ ErrorCode::InvalidTickArrayAccount
    )]
    pub tick_array: Account<'info, TickArray>,

    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}
