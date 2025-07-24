use anchor_lang::prelude::*;
#[account]
pub struct Position {
    pub liquidity: u128,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub owner: Pubkey,
    pub pool: Pubkey,
    pub bump: u8,
}

impl Position {
    pub const SPACE: usize = 8 + // discriminator
        16 + // liquidity
        4 +  // tick_lower
        4 +  // tick_upper
        32 + // owner
        32 + // pool
        1;   // bump
}