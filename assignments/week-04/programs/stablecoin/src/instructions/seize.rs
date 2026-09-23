use anchor_lang::prelude::*;
use anchor_lang::solana_program::{instruction::Instruction, program::invoke_signed};
use anchor_spl::token_2022::spl_token_2022::{
    extension::{
        permanent_delegate::PermanentDelegate,
        transfer_fee::instruction::transfer_checked_with_fee, BaseStateWithExtensions,
        StateWithExtensions,
    },
    instruction::{freeze_account, thaw_account},
    state::{Account as TokenAccountState, Mint as MintState},
};
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_interface::{Mint, TokenAccount};

use crate::errors::StablecoinError;
use crate::fees::decimals_and_epoch_fee;
use crate::state::{Issuer, ISSUER_SEED};

#[derive(Accounts)]
pub struct Seize<'info> {
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
    pub source: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = mint,
        token::token_program = token_program
    )]
    pub treasury: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Program<'info, Token2022>,
}

impl<'info> Seize<'info> {
    pub fn seize(&mut self, amount: u64) -> Result<()> {
        require!(amount > 0, StablecoinError::InvalidAmount);
        require!(self.issuer_is_permanent_delegate()?, StablecoinError::NotSeizable);

        let (public_balance, frozen) = {
            let info = self.source.to_account_info();
            let data = info.try_borrow_data()?;
            let account = StateWithExtensions::<TokenAccountState>::unpack(&data)?;

            (account.base.amount, account.base.is_frozen())
        };

        require!(
            amount <= public_balance,
            StablecoinError::InsufficientPublicBalance
        );

        let (decimals, fee) = decimals_and_epoch_fee(&self.mint.to_account_info(), amount)?;

        if frozen {
            self.invoke_as_issuer(thaw_account(
                &self.token_program.key(),
                &self.source.key(),
                &self.mint.key(),
                &self.issuer.key(),
                &[],
            )?)?;
        }

        self.invoke_as_issuer(transfer_checked_with_fee(
            &self.token_program.key(),
            &self.source.key(),
            &self.mint.key(),
            &self.treasury.key(),
            &self.issuer.key(),
            &[],
            amount,
            decimals,
            fee,
        )?)?;

        if frozen {
            self.invoke_as_issuer(freeze_account(
                &self.token_program.key(),
                &self.source.key(),
                &self.mint.key(),
                &self.issuer.key(),
                &[],
            )?)?;
        }

        Ok(())
    }

    fn issuer_is_permanent_delegate(&self) -> Result<bool> {
        let info = self.mint.to_account_info();
        let data = info.try_borrow_data()?;
        let mint = StateWithExtensions::<MintState>::unpack(&data)?;

        Ok(match mint.get_extension::<PermanentDelegate>() {
            Ok(extension) => Option::<Pubkey>::from(extension.delegate) == Some(self.issuer.key()),
            Err(_) => false,
        })
    }

    fn invoke_as_issuer(&self, instruction: Instruction) -> Result<()> {
        let seeds = self.issuer.signer_seeds();

        invoke_signed(
            &instruction,
            &[
                self.source.to_account_info(),
                self.mint.to_account_info(),
                self.treasury.to_account_info(),
                self.issuer.to_account_info(),
                self.token_program.to_account_info(),
            ],
            &[&seeds],
        )?;

        Ok(())
    }
}
