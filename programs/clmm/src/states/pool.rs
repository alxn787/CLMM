use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Pool {
    pub token_mint_0: Pubkey,
    pub token_mint_1: Pubkey,
    pub token_vault_0: Pubkey,
    pub token_vault_1: Pubkey,
    pub global_liquidity: u128,
    pub sqrt_price_x96: u128,
    pub current_tick: i32,
    pub tick_spacing: i32,
    pub bump: u8,
}

impl Pool {
    pub const SPACE: usize = 8 + // discriminator
        32 + // token_mint_0
        32 + // token_mint_1
        32 + // token_vault_0
        32 + // token_vault_1
        16 + // global_liquidity
        16 + // sqrt_price_x96
        4 +  // current_tick
        4 +  // tick_spacing
        1;   // bump
}