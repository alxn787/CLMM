use anchor_lang::{accounts::sysvar, prelude::*};
use anchor_spl::token::{Mint, Token, TokenAccount};

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
    pub rent : Sysvar<'info,Rent>
}

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
        init_if_needed,
        payer = payer,
        space = TickArray::INIT_SPACE,
        seeds = [
            b"tick_array",
            pool.key().as_ref(),
            &TickArray::get_starting_tick_index(lower_tick, pool.tick_spacing ).to_le_bytes()
        ],
        bump,
    )]
    pub lower_tick_array: Account<'info, TickArray>,

    #[account(
    init_if_needed,
    payer = payer,
    space = TickArray::INIT_SPACE,
    seeds = [
        b"tick_array",
        pool.key().as_ref(),
        &TickArray::get_starting_tick_index(upper_tick, pool.tick_spacing ).to_le_bytes()
    ],
    bump,
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
    pub fn get_starting_tick_index(tick: i32, tick_spacing: i32) -> i32 {
        let tick_spacing_i32 = tick_spacing as i32;
        let array_idx = tick.checked_div(tick_spacing_i32).expect("Div by zero")
                            .checked_div(TICKS_PER_ARRAY as i32).expect("Div by zero");
                            array_idx.checked_mul(TICKS_PER_ARRAY as i32).expect("Mul overflow")
                            .checked_mul(tick_spacing_i32).expect("Mul overflow")
    }
    pub fn get_tick_info_mutable(&mut self, tick: i32, tick_spacing: u16) -> Result<&mut TickInfo> {
        let tick_spacing_i32 = tick_spacing as i32;
        let offset = (tick.checked_div(tick_spacing_i32).ok_or(ErrorCode::ArithmeticOverflow)?)
            .checked_sub(self.starting_tick.checked_div(tick_spacing_i32).ok_or(ErrorCode::ArithmeticOverflow)?)
            .ok_or(ErrorCode::ArithmeticOverflow)?
            .checked_rem(TICKS_PER_ARRAY as i32).ok_or(ErrorCode::ArithmeticOverflow)? as usize;
        Ok(&mut self.ticks[offset])
    }
}

#[error_code]
pub enum ErrorCode {
    #[msg("Arithmetic Overflow")]
    ArithmeticOverflow
}