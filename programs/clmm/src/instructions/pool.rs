use anchor_lang:: prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::states::*;
use crate::utils::ErrorCode;
use crate::utils::math::*;

#[derive(Accounts)]
#[instruction(tick_spacing: i32)]
pub struct InitializePool<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = Pool::INIT_SPACE,
        seeds = [
            b"pool".as_ref(),
            token_mint_0.key().as_ref(),
            token_mint_1.key().as_ref(),
            &tick_spacing.to_le_bytes()
        ],
        bump,
    )]
    pub pool: Account<'info, Pool>,
    pub token_mint_0: Account<'info, Mint>,
    pub token_mint_1: Account<'info, Mint>,

    #[account(
        init,
        payer = payer,
        token::mint = token_mint_0,
        token::authority = pool,
    )]
    pub token_vault_0: Account<'info, TokenAccount>,
    
    #[account(
        init,
        payer = payer,
        token::mint = token_mint_1,
        token::authority = pool,
    )]
    pub token_vault_1: Account<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn init_pool(ctx:Context<InitializePool>, tick_spacing: i32, initial_sqrt_price: u128) -> Result<()> {
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