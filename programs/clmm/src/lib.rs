use anchor_lang::prelude::*;

declare_id!("4GhrgMYusqS5uuyzrrBvFv3FuVGp4RRp4XKDBctyW6oN");

#[program]
pub mod clmm {
    use super::*;

    pub fn initialize(ctx: Context<InitializePool>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializePool {

}

#[account]
#[derive(InitSpace)]
pub struct Pool {
    pub liquidity: u128,
    pub sqrt_price_x96 : u128,
    pub current_tick :i32,
    pub tick_spacing : i32
}

#[account]
#[derive(InitSpace)]
pub struct Tick {
    pub liquidity_gross: u128,
    pub initialized: bool,
}

#[account]
#[derive(InitSpace)]
pub struct Position {
    pub liquidity: u128,
    pub tick_lower: i32,
    pub tick_upper: i32,
}
