use anchor_lang::prelude::*;

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
    #[msg("TickArray account not found or invalid")]
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
