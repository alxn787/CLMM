use anchor_lang::{accounts::sysvar, prelude::*};
use anchor_spl::token::{Mint, Token, TokenAccount};

declare_id!("4GhrgMYusqS5uuyzrrBvFv3FuVGp4RRp4XKDBctyW6oN");

#[program]
pub mod clmm {
    use super::*;

    pub fn initializePool(
        ctx: Context<InitializePool>,
        tick_spacing: i32,
        initial_sqrt_price: u128,
        ) -> Result<()> {
        let pool = &mut ctx.accounts.pool;
        require!(tick_spacing > 100 , ErrorCode::InvalidTickSpacing);
        require!(ctx.accounts.token_mint_0.key() != ctx.accounts.token_mint_1.key(), ErrorCode::InvalidTokenPair);

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

#[derive(Accounts)]
#[instruction(amount_in: u64, swap_token_0_for_1:bool, amount_out_minimum: u64)]
pub struct Swap <'info> {

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

    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(lower_tick:i32, upper_tick:i32, liquidity_amount: u128)]
pub struct Burn<'info> {

    #[account(mut)]
    pub pool: Account<'info, Pool>,

    #[account(
        mut,
        close = owner,
        seeds = [
            b"position",
            owner.key().as_ref(),
            pool.key().as_ref(),
            &lower_tick.to_le_bytes(),
            &upper_tick.to_le_bytes(),
        ],
        bump = position.bump,
    )]
    pub position: Account<'info, Position>,

    #[account(mut)]
    pub tick_array_lower: Account<'info, TickArray>,
    
    #[account(mut)]
    pub tick_array_upper: Account<'info, TickArray>,

    #[account(mut)]
    pub pool_token_0: Account<'info, TokenAccount>,

    #[account(mut)]
    pub pool_token_1: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user_token_0: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user_token_1: Account<'info, TokenAccount>,

    #[account(mut)]
    pub owner: Signer<'info>,
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
    pub owner: Pubkey,
    pub pool: Pubkey,
    pub bump: u8,
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
    ArithmeticOverflow,
    #[msg("Invalid Tick Range")]
    InvalidTickRange,
    #[msg("Insufficient Input Amount")]
    InsufficientInputAmount,
    #[msg("Slippage Exceeded")]
    SlippageExceeded,
    #[msg("Insufficient Liquidity in Position")]
    InsufficientLiquidity,
    #[msg("Invalid Tick Spacing")]
    InvalidTickSpacing,
    #[msg("Invalid Initial Price")]
    InvalidPrice,
    #[msg("Invalid Position Owner")]
    InvalidPositionOwner,
    #[msg("Invalid Position Range")]
    InvalidPositionRange,
    #[msg("Could not find next initialized tick")]
    TickNotFound,
    #[msg("Token 0 transfer failed")]
    Token0TransferFailed,
    #[msg("Token 1 transfer failed")]
    Token1TransferFailed,
    #[msg("Invalid PDA bump")]
    InvalidBump,
    #[msg("TickArray account not found or invalid in remaining accounts")]
    InvalidTickArrayAccount,
    #[msg("Invalid Token Pair (Mints cannot be identical)")]
    InvalidTokenPair,
    #[msg("Minted liquidity must cover the current price tick interval")]
    MintRangeMustCoverCurrentPrice,
    #[msg("Burned liquidity must cover the current price tick interval")]
    BurnRangeMustCoverCurrentPrice,
    #[msg("Insufficient pool liquidity to fulfill swap")]
    InsufficientPoolLiquidity,
}

pub fn get_sqrt_price_from_tick(tick: i32) -> Result<u128> {
    // Represents sqrt(1) in Q64.96 format, i.e., price of 1.
    let base_sqrt_price = 1u128 << 96;
    // Example small adjustment per tick. This is NOT mathematically derived.
    let adjustment_factor = 1_000_000_000 / 1000;

    // Apply a linear adjustment based on the tick index.
    // This is a simplification; real math is logarithmic.
    let adjusted_price = base_sqrt_price.checked_add_signed((tick as i128) * (adjustment_factor as i128))
        .ok_or(ErrorCode::ArithmeticOverflow)?;
    Ok(adjusted_price)
}

/// Placeholder: Converts a sqrt_price_x96 back to its nearest tick index.
///
/// This is the inverse of `get_sqrt_price_from_tick`.
pub fn get_tick_at_sqrt_price(sqrt_price_x96: u128) -> Result<i32> {
    let base_sqrt_price = 1u128 << 96;
    let adjustment_factor = 1_000_000_000 / 1000;

    // Calculate the difference from the base price.
    let diff = sqrt_price_x96 as i128 - base_sqrt_price as i128;
    // Reverse the linear adjustment to get the tick.
    let tick = diff.checked_div(adjustment_factor as i128).ok_or(ErrorCode::ArithmeticOverflow)? as i32;
    Ok(tick)
}