use anchor_lang::prelude::*;

#[error_code]
pub enum HypeBondError {
    #[msg("The program has already been initialized.")]
    AlreadyInitialized,
    
    #[msg("The program has not been initialized.")]
    NotInitialized,
    
    #[msg("The provided authority is not authorized.")]
    NotAuthorized,
    
    #[msg("Mathematical overflow occurred.")]
    MathOverflow,
    
    #[msg("The trade size is invalid.")]
    InvalidTradeSize,

    #[msg("Too much SOL required")]
    TooMuchSolRequired,

    #[msg("Bonfing curve complete")]
    BondingCurveComplete,

    #[msg("SOL received too little")]
    TooLittleSolReceived,

    #[msg("Invalid percentage allocations")]
    InvalidPercentages,

    #[msg("Invalid milestone count")]
    InvalidMilestoneCount,
}