use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer};
use anchor_spl::token::{Token, TokenAccount};
use crate::states::*;
use crate::utils::ErrorCode;
use crate::utils::math::*;

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