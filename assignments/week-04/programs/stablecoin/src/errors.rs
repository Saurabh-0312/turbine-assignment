use anchor_lang::prelude::*;

#[error_code]
pub enum StablecoinError {
    #[msg("Amount must be greater than zero")]
    InvalidAmount,
    #[msg("Transfer fee cannot exceed 10000 basis points")]
    InvalidFee,
    #[msg("Transfer fee calculation overflowed")]
    FeeOverflow,
    #[msg("Account has already cleared KYC")]
    AlreadyVerified,
    #[msg("Account is already frozen")]
    AlreadyFrozen,
    #[msg("This mint has no seizure authority")]
    NotSeizable,
    #[msg("Only the public balance of an account can be seized")]
    InsufficientPublicBalance,
}
