use anchor_lang::{accounts::sysvar, prelude::*};
use anchor_spl::token::{Mint, Token, TokenAccount};

declare_id!("4GhrgMYusqS5uuyzrrBvFv3FuVGp4RRp4XKDBctyW6oN");

#[program]
pub mod clmm {
    use anchor_spl::token::{self, Transfer};

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

    pub fn add_liquidity(
        ctx: Context<AddLiquidity>,
        owner: Pubkey,
        lower_tick: i32,
        upper_tick: i32,
        liquidity_amount:u128
    )-> Result<(u64,u64)>{

        let pool = &mut ctx.accounts.pool;
        let position = &mut ctx.accounts.position;

        require!(
            lower_tick < upper_tick &&
            lower_tick % (pool.tick_spacing as i32) == 0 &&
            upper_tick % (pool.tick_spacing as i32) == 0,
            ErrorCode::InvalidTickRange
        );

        require!(liquidity_amount > 0, ErrorCode::InsufficientInputAmount);

        //ideally should be able to add anywhere .
        //here were simplifying this to only add in the current tick range
        require!(
            pool.current_tick >= lower_tick && pool.current_tick < upper_tick,
            ErrorCode::MintRangeMustCoverCurrentPrice
        );

        let lower_tick_info = ctx.accounts.lower_tick_array.get_tick_info_mutable(lower_tick, pool.tick_spacing)?;
        let upper_tick_info = ctx.accounts.upper_tick_array.get_tick_info_mutable(upper_tick, pool.tick_spacing)?;

        lower_tick_info.update_liquidity(liquidity_amount as i128, true)?;
        upper_tick_info.update_liquidity(liquidity_amount as i128, false)?;

        let current_sqrt_price_x96 = pool.sqrt_price_x96;
        let lower_sqrt_price_x96 = get_sqrt_price_from_tick(lower_tick)?;
        let upper_sqrt_price_x96 = get_sqrt_price_from_tick(upper_tick)?;
 
        let(amount_0, amount_1) = get_amounts_for_liquidity(current_sqrt_price_x96, lower_sqrt_price_x96, upper_sqrt_price_x96, liquidity_amount)?;


        if position.liquidity == 0 && position.owner == Pubkey::default() { 
            position.owner = owner;
            position.pool = pool.key();
            position.tick_lower = lower_tick;
            position.tick_upper = upper_tick;
            position.liquidity = liquidity_amount;
            position.bump = ctx.bumps.position;

        } else { 
            require!(position.owner == owner, ErrorCode::InvalidPositionOwner);
            require!(position.tick_lower == lower_tick && position.tick_upper == upper_tick, ErrorCode::InvalidPositionRange);
            position.liquidity = position.liquidity.checked_add(liquidity_amount).ok_or(ErrorCode::ArithmeticOverflow)?;
        }

        pool.global_liquidity = pool.global_liquidity.checked_add(liquidity_amount).ok_or(ErrorCode::ArithmeticOverflow)?;

                if amount_0 > 0 {
            let cpi_accounts_0 = Transfer {
                from: ctx.accounts.user_token_0.to_account_info(),
                to: ctx.accounts.pool_token_0.to_account_info(),
                authority: ctx.accounts.payer.to_account_info(), // User is the authority
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            token::transfer(CpiContext::new(cpi_program, cpi_accounts_0), amount_0)?;
        }

        if amount_1 > 0 {
            let cpi_accounts_1 = Transfer {
                from: ctx.accounts.user_token_1.to_account_info(),
                to: ctx.accounts.pool_token_1.to_account_info(),
                authority: ctx.accounts.payer.to_account_info(), 
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            token::transfer(CpiContext::new(cpi_program, cpi_accounts_1), amount_1)?;
        }

        ctx.accounts.pool_token_0.reload()?;
        ctx.accounts.pool_token_1.reload()?;

        require!(
            ctx.accounts.pool_token_0.amount >= amount_0,
            ErrorCode::Token0TransferFailed
        );
        require!(
            ctx.accounts.pool_token_1.amount >= amount_1,
            ErrorCode::Token1TransferFailed
        );
        Ok((amount_0, amount_1))
    }


    pub fn swap(ctx:Context<Swap>, amount_in:u64,swap_token_0_for_1:bool, amount_out_minimum:u64)-> Result<u64>{

        let pool = &mut ctx.accounts.pool;

        require!(pool.global_liquidity > 0, ErrorCode::InsufficientPoolLiquidity);
        require!(amount_in > 0, ErrorCode::InsufficientInputAmount);

        let current_sqrt_price_x96 = pool.sqrt_price_x96;
        let global_liquidity = pool.global_liquidity;

        // let target_sqrt_price_x96 = if swap_token_0_for_1{
        //     get_sqrt_price_from_tick(pool.current_tick - 1)?
        // }else{
        //     get_sqrt_price_from_tick(pool.current_tick + 1)?
        // };

        // let current_tick_info = ctx.accounts.tick_array.get_tick_info_mutable(
        //     pool.current_tick,
        //     pool.tick_spacing,
        // )?;

        let (amount_in_used, amount_out_calculated, new_sqrt_price_x96) =  swap_segment(current_sqrt_price_x96, global_liquidity, amount_in, swap_token_0_for_1)?;
        
        require!(amount_out_calculated >= amount_out_minimum, ErrorCode::SlippageExceeded);

        let signer_seeds: &[&[&[u8]]] = &[&[
            b"pool",
            pool.token_mint_0.as_ref(),
            pool.token_mint_1.as_ref(),
            &pool.tick_spacing.to_le_bytes(),
            &[pool.bump],
        ]];

        if swap_token_0_for_1 {
            let cpi_accounts_in = Transfer {
                from: ctx.accounts.user_token_0.to_account_info(),
                to: ctx.accounts.pool_token_0.to_account_info(),
                authority: ctx.accounts.payer.to_account_info(),
            };
            token::transfer(CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts_in), amount_in_used)?;

            let cpi_accounts_out = Transfer {
                from: ctx.accounts.pool_token_1.to_account_info(),
                to: ctx.accounts.user_token_1.to_account_info(),
                authority: pool.to_account_info(),
            };
            token::transfer(CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), cpi_accounts_out, signer_seeds), amount_out_calculated)?;
        } else {

            let cpi_accounts_in = Transfer {
                from: ctx.accounts.user_token_1.to_account_info(),
                to: ctx.accounts.pool_token_1.to_account_info(),
                authority: ctx.accounts.payer.to_account_info(),
            };
            token::transfer(CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts_in), amount_in_used)?;

            let cpi_accounts_out = Transfer {
                from: ctx.accounts.pool_token_0.to_account_info(),
                to: ctx.accounts.user_token_0.to_account_info(),
                authority: pool.to_account_info(), 
            };
            token::transfer(CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), cpi_accounts_out, signer_seeds), amount_out_calculated)?;
        }

        pool.sqrt_price_x96 = new_sqrt_price_x96;
        pool.current_tick = get_tick_at_sqrt_price(new_sqrt_price_x96)?;

        Ok(amount_out_calculated)
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
#[instruction(lower_tick: i32, upper_tick: i32, liquidity_amount: u128, tick_spacing: i32)]
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
            &TickArray::get_starting_tick_index(lower_tick, tick_spacing ).to_le_bytes()
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
        &TickArray::get_starting_tick_index(upper_tick,tick_spacing ).to_le_bytes()
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

    #[account(
        mut,
        seeds = [
            b"tick_array",
            pool.key().as_ref(),
            &TickArray::get_starting_tick_index(pool.current_tick,pool.tick_spacing ).to_le_bytes()
        ],
        bump,
    )]
    pub tick_array: Account<'info, TickArray>,

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

pub const TICKS_PER_ARRAY: usize = 30;

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
    pub fn get_tick_info_mutable(&mut self, tick: i32, tick_spacing: i32) -> Result<&mut TickInfo> {
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
    let base_sqrt_price = 1u128 << 96;
    let adjustment_factor = 1_000_000_000 / 1000;
    // This is a simplification; real math is logarithmic.
    let adjusted_price = base_sqrt_price.checked_add_signed((tick as i128) * (adjustment_factor as i128))
        .ok_or(ErrorCode::ArithmeticOverflow)?;
    Ok(adjusted_price)
}

pub fn get_tick_at_sqrt_price(sqrt_price_x96: u128) -> Result<i32> {
    let base_sqrt_price = 1u128 << 96;
    let adjustment_factor = 1_000_000_000 / 1000;

    let diff = sqrt_price_x96 as i128 - base_sqrt_price as i128;
    let tick = diff.checked_div(adjustment_factor as i128).ok_or(ErrorCode::ArithmeticOverflow)? as i32;
    Ok(tick)
}

pub fn get_amounts_for_liquidity(
    current_sqrt_price_x96: u128,
    lower_sqrt_price_x96: u128,
    upper_sqrt_price_x96: u128,
    liquidity: u128,
) -> Result<(u64, u64)> {
    let mut amount0 = 0u64;
    let mut amount1 = 0u64;

    //simplified logic with approximation 
    if current_sqrt_price_x96 >= lower_sqrt_price_x96 && current_sqrt_price_x96 < upper_sqrt_price_x96 {
        amount0 = (liquidity / 2) as u64; 
        amount1 = (liquidity / 2) as u64;
    } else if current_sqrt_price_x96 < lower_sqrt_price_x96 {
        amount0 = liquidity as u64; 
    } else {
        amount1 = liquidity as u64; 
    }
    Ok((amount0, amount1))
}

pub fn swap_segment(
    current_sqrt_price_x96: u128,
    global_liquidity: u128,
    amount_remaining_in: u64,
    swap_token_0_for_1: bool,
) -> Result<(u64, u64, u128)> {
    if global_liquidity == 0 {
        return Err(ErrorCode::InsufficientPoolLiquidity.into());
    }

    let amount_in_used = amount_remaining_in;
    let mut amount_out_calculated = 0u64;
    let mut new_sqrt_price = current_sqrt_price_x96;

    amount_out_calculated = amount_in_used.checked_sub(amount_in_used / 1000).ok_or(ErrorCode::ArithmeticOverflow)?; // 0.1% slippage 
    if swap_token_0_for_1 {
        new_sqrt_price = current_sqrt_price_x96.checked_sub(1_000_000_000).ok_or(ErrorCode::ArithmeticOverflow)?;
        if new_sqrt_price < 1 { new_sqrt_price = 1; }
        new_sqrt_price = current_sqrt_price_x96.checked_add(1_000_000_000).ok_or(ErrorCode::ArithmeticOverflow)?;
    }

    Ok((amount_in_used, amount_out_calculated, new_sqrt_price))
}