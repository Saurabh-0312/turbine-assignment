use anchor_lang::prelude::*;

use crate::errors::EscrowError;
use crate::state::Escrow;

#[derive(Accounts)]
pub struct Update<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
        mut,
        has_one = maker,
        seeds = [b"escrow", maker.key().as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump
    )]
    pub escrow: Account<'info, Escrow>,
}

impl<'info> Update<'info> {
    pub fn update(&mut self, receive: u64, expiry: i64) -> Result<()> {
        require!(receive > 0, EscrowError::InvalidAmount);
        require!(
            expiry > Clock::get()?.unix_timestamp,
            EscrowError::InvalidExpiry
        );

        self.escrow.receive = receive;
        self.escrow.expiry = expiry;

        Ok(())
    }
}
