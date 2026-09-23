use anchor_lang::prelude::*;

use crate::state::{Issuer, ISSUER_SEED};

#[derive(Accounts)]
pub struct InitializeIssuer<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + Issuer::INIT_SPACE,
        seeds = [ISSUER_SEED, admin.key().as_ref()],
        bump
    )]
    pub issuer: Account<'info, Issuer>,

    pub system_program: Program<'info, System>,
}

impl<'info> InitializeIssuer<'info> {
    pub fn initialize_issuer(&mut self, bump: u8) -> Result<()> {
        self.issuer.set_inner(Issuer {
            admin: self.admin.key(),
            bump,
        });

        Ok(())
    }
}
