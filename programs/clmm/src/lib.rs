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
    pub tick_spacing : i32,
    pub bump : u8,
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

#[account]
#[derive(InitSpace)]
pub struct TickInfo {
    pub initialized : bool,
    pub liquidity_total : u128
}

impl TickInfo {
    pub fn add_liquidity(&mut self , liquidity: u128){
        let init_liquidity = self.liquidity_total;

        if init_liquidity == 0 {
            self.initialized = true;
        }

        let final_liquidity = init_liquidity.checked_add(liquidity).expect("liq overflow");
        self.liquidity_total = final_liquidity;
    }
}

#[error_code]
pub enum ErrorCode {
    #[msg("Arithmetic Overflow")]
    ArithmeticOverflow
}