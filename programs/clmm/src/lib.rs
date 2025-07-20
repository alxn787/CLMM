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
    pub token_mint_0: Pubkey,
    pub token_mint_1: Pubkey,
    pub token_vault_0: Pubkey,
    pub token_vault_1: Pubkey,
    pub global_liquidity: u128,
    pub sqrt_price_x96 : u128,
    pub current_tick :i32,
    pub tick_spacing : i32,
    pub bump : u8,
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
    pub liquidity_gross : u128,
    pub liquidity_net : i128,
}

impl TickInfo {
    pub fn update_liquidity(&mut self , liquidity_delta: i128, is_lower: bool) -> Result<()> {
       
       if self.liquidity_gross == 0 {
            self.initialized = true;
        }

        self.liquidity_gross = self.liquidity_gross.checked_add(liquidity_delta.unsigned_abs() as u128)
            .ok_or(ErrorCode::ArithmeticOverflow)?;

        if is_lower {
            self.liquidity_net = self.liquidity_net.checked_add(liquidity_delta).ok_or(ErrorCode::ArithmeticOverflow)?;
        } else {
            self.liquidity_net = self.liquidity_net.checked_sub(liquidity_delta).ok_or(ErrorCode::ArithmeticOverflow)?;
        }
        Ok(())
    }
}

pub const TICKS_PER_ARRAY: usize = 100;

#[account]
#[derive(InitSpace)]
pub struct TickArray {
    pub pool : Pubkey,
    pub starting_tick: i32,
    pub ticks: [TickInfo; TICKS_PER_ARRAY],
    pub bump : u8,
}

impl TickArray {

}

#[error_code]
pub enum ErrorCode {
    #[msg("Arithmetic Overflow")]
    ArithmeticOverflow
}