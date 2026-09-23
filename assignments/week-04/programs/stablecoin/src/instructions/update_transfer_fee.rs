use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_spl::token_2022::spl_token_2022::extension::transfer_fee::instruction::set_transfer_fee;
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_interface::Mint;

use crate::errors::StablecoinError;
use crate::state::{Issuer, ISSUER_SEED, MAX_FEE_BASIS_POINTS};

#[derive(Accounts)]
pub struct UpdateTransferFee<'info> {
    pub admin: Signer<'info>,

    #[account(
        has_one = admin,
        seeds = [ISSUER_SEED, admin.key().as_ref()],
        bump = issuer.bump
    )]
    pub issuer: Account<'info, Issuer>,

    #[account(mut, mint::token_program = token_program)]
    pub mint: Box<InterfaceAccount<'info, Mint>>,

    pub token_program: Program<'info, Token2022>,
}

impl<'info> UpdateTransferFee<'info> {
    pub fn update_transfer_fee(
        &mut self,
        transfer_fee_basis_points: u16,
        maximum_fee: u64,
    ) -> Result<()> {
        require!(
            transfer_fee_basis_points <= MAX_FEE_BASIS_POINTS,
            StablecoinError::InvalidFee
        );

        let seeds = self.issuer.signer_seeds();

        invoke_signed(
            &set_transfer_fee(
                &self.token_program.key(),
                &self.mint.key(),
                &self.issuer.key(),
                &[],
                transfer_fee_basis_points,
                maximum_fee,
            )?,
            &[
                self.mint.to_account_info(),
                self.issuer.to_account_info(),
                self.token_program.to_account_info(),
            ],
            &[&seeds],
        )?;

        Ok(())
    }
}
