use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke;
use anchor_spl::token_2022::spl_token_2022::extension::transfer_fee::instruction::transfer_checked_with_fee;
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_interface::{Mint, TokenAccount};

use crate::errors::StablecoinError;
use crate::fees::decimals_and_epoch_fee;

#[derive(Accounts)]
pub struct TransferWithFee<'info> {
    pub owner: Signer<'info>,

    #[account(mint::token_program = token_program)]
    pub mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        token::mint = mint,
        token::authority = owner,
        token::token_program = token_program
    )]
    pub source: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = mint,
        token::token_program = token_program
    )]
    pub destination: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Program<'info, Token2022>,
}

impl<'info> TransferWithFee<'info> {
    pub fn transfer_with_fee(&mut self, amount: u64) -> Result<()> {
        require!(amount > 0, StablecoinError::InvalidAmount);

        let (decimals, fee) = decimals_and_epoch_fee(&self.mint.to_account_info(), amount)?;

        invoke(
            &transfer_checked_with_fee(
                &self.token_program.key(),
                &self.source.key(),
                &self.mint.key(),
                &self.destination.key(),
                &self.owner.key(),
                &[],
                amount,
                decimals,
                fee,
            )?,
            &[
                self.source.to_account_info(),
                self.mint.to_account_info(),
                self.destination.to_account_info(),
                self.owner.to_account_info(),
                self.token_program.to_account_info(),
            ],
        )?;

        Ok(())
    }
}
