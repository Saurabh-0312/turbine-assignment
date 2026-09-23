use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_spl::token_2022::spl_token_2022::instruction::mint_to_checked;
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_interface::{Mint, TokenAccount};

use crate::errors::StablecoinError;
use crate::fees::decimals;
use crate::state::{Issuer, ISSUER_SEED};

#[derive(Accounts)]
pub struct Issue<'info> {
    pub admin: Signer<'info>,

    #[account(
        has_one = admin,
        seeds = [ISSUER_SEED, admin.key().as_ref()],
        bump = issuer.bump
    )]
    pub issuer: Account<'info, Issuer>,

    #[account(
        mut,
        mint::authority = issuer,
        mint::token_program = token_program
    )]
    pub mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        token::mint = mint,
        token::token_program = token_program
    )]
    pub destination: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Program<'info, Token2022>,
}

impl<'info> Issue<'info> {
    pub fn issue(&mut self, amount: u64) -> Result<()> {
        require!(amount > 0, StablecoinError::InvalidAmount);

        let decimals = decimals(&self.mint.to_account_info())?;
        let seeds = self.issuer.signer_seeds();

        invoke_signed(
            &mint_to_checked(
                &self.token_program.key(),
                &self.mint.key(),
                &self.destination.key(),
                &self.issuer.key(),
                &[],
                amount,
                decimals,
            )?,
            &[
                self.mint.to_account_info(),
                self.destination.to_account_info(),
                self.issuer.to_account_info(),
                self.token_program.to_account_info(),
            ],
            &[&seeds],
        )?;

        Ok(())
    }
}
