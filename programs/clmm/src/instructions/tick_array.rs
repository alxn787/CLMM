use anchor_lang::prelude::*;
use crate::states::*;

#[derive(Accounts)]
#[instruction(starting_tick: i32)]
pub struct CreateTickArray<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account()]
    pub pool: Account<'info, Pool>,

    #[account(
        init,
        payer = payer,
        space = TickArray::INIT_SPACE,
        seeds = [
            b"tick_array",
            pool.key().as_ref(),
            &starting_tick.to_le_bytes()
        ],
        bump
    )]
    pub tick_array: Account<'info, TickArray>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}
