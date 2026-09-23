use anchor_lang::prelude::*;
use anchor_lang::solana_program::{instruction::Instruction, program::invoke_signed};
use anchor_spl::token_2022::spl_token_2022::{
    extension::{confidential_transfer::instruction::approve_account, StateWithExtensions},
    instruction::{freeze_account, thaw_account},
    state::Account as TokenAccountState,
};
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_interface::{Mint, TokenAccount};

use crate::errors::StablecoinError;
use crate::state::{Issuer, ISSUER_SEED};

#[derive(Accounts)]
pub struct Compliance<'info> {
    pub admin: Signer<'info>,

    #[account(
        has_one = admin,
        seeds = [ISSUER_SEED, admin.key().as_ref()],
        bump = issuer.bump
    )]
    pub issuer: Account<'info, Issuer>,

    #[account(
        mint::freeze_authority = issuer,
        mint::token_program = token_program
    )]
    pub mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        token::mint = mint,
        token::token_program = token_program
    )]
    pub token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Program<'info, Token2022>,
}

impl<'info> Compliance<'info> {
    pub fn approve_kyc(&mut self) -> Result<()> {
        require!(self.is_frozen()?, StablecoinError::AlreadyVerified);

        self.invoke_as_issuer(thaw_account(
            &self.token_program.key(),
            &self.token_account.key(),
            &self.mint.key(),
            &self.issuer.key(),
            &[],
        )?)
    }

    pub fn sanction(&mut self) -> Result<()> {
        require!(!self.is_frozen()?, StablecoinError::AlreadyFrozen);

        self.invoke_as_issuer(freeze_account(
            &self.token_program.key(),
            &self.token_account.key(),
            &self.mint.key(),
            &self.issuer.key(),
            &[],
        )?)
    }

    pub fn approve_confidential_account(&mut self) -> Result<()> {
        self.invoke_as_issuer(approve_account(
            &self.token_program.key(),
            &self.token_account.key(),
            &self.mint.key(),
            &self.issuer.key(),
            &[],
        )?)
    }

    fn is_frozen(&self) -> Result<bool> {
        let info = self.token_account.to_account_info();
        let data = info.try_borrow_data()?;

        Ok(StateWithExtensions::<TokenAccountState>::unpack(&data)?
            .base
            .is_frozen())
    }

    fn invoke_as_issuer(&self, instruction: Instruction) -> Result<()> {
        let seeds = self.issuer.signer_seeds();

        invoke_signed(
            &instruction,
            &[
                self.token_account.to_account_info(),
                self.mint.to_account_info(),
                self.issuer.to_account_info(),
                self.token_program.to_account_info(),
            ],
            &[&seeds],
        )?;

        Ok(())
    }
}
