use anchor_lang::prelude::*;

#[error_code]
pub enum EscrowError {
    #[msg("Amount must be greater than zero")]
    InvalidAmount,
    #[msg("Expiry must be in the future")]
    InvalidExpiry,
    #[msg("This escrow has expired")]
    EscrowExpired,
    #[msg("Both mints must be different")]
    IdenticalMints,
}
