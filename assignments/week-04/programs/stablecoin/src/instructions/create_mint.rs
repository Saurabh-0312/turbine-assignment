use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::{invoke, invoke_signed};
use anchor_lang::system_program::{create_account, transfer, CreateAccount, Transfer};
use anchor_spl::token_2022::spl_token_2022::{
    extension::{
        confidential_transfer, confidential_transfer_fee, default_account_state,
        metadata_pointer, transfer_fee, BaseStateWithExtensions, ExtensionType,
        StateWithExtensions,
    },
    instruction::{initialize_mint2, initialize_mint_close_authority, initialize_permanent_delegate},
    solana_zk_sdk::encryption::pod::elgamal::PodElGamalPubkey,
    state::{AccountState, Mint},
};
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_2022_extensions::spl_pod::optional_keys::OptionalNonZeroPubkey;
use anchor_spl::token_2022_extensions::spl_token_metadata_interface::{
    instruction::initialize as initialize_token_metadata, state::TokenMetadata,
};

use crate::errors::StablecoinError;
use crate::state::{ConfidentialArgs, Issuer, MintArgs, ISSUER_SEED, MAX_FEE_BASIS_POINTS};

#[derive(Accounts)]
pub struct CreateMint<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        has_one = admin,
        seeds = [ISSUER_SEED, admin.key().as_ref()],
        bump = issuer.bump
    )]
    pub issuer: Account<'info, Issuer>,

    #[account(mut)]
    pub mint: Signer<'info>,

    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

impl<'info> CreateMint<'info> {
    pub fn create_mint(
        &mut self,
        args: &MintArgs,
        confidential: Option<&ConfidentialArgs>,
    ) -> Result<()> {
        require!(
            args.transfer_fee_basis_points <= MAX_FEE_BASIS_POINTS,
            StablecoinError::InvalidFee
        );

        let issuer = self.issuer.key();
        let mint = self.mint.key();
        let token_program = self.token_program.key();

        let mut extensions = vec![
            ExtensionType::TransferFeeConfig,
            ExtensionType::MetadataPointer,
            ExtensionType::DefaultAccountState,
            ExtensionType::MintCloseAuthority,
        ];

        let mut instructions = vec![
            transfer_fee::instruction::initialize_transfer_fee_config(
                &token_program,
                &mint,
                Some(&issuer),
                Some(&issuer),
                args.transfer_fee_basis_points,
                args.maximum_fee,
            )?,
            metadata_pointer::instruction::initialize(
                &token_program,
                &mint,
                Some(issuer),
                Some(mint),
            )?,
            default_account_state::instruction::initialize_default_account_state(
                &token_program,
                &mint,
                &AccountState::Frozen,
            )?,
            initialize_mint_close_authority(&token_program, &mint, Some(&issuer))?,
        ];

        if let Some(confidential) = confidential {
            extensions.extend([
                ExtensionType::PermanentDelegate,
                ExtensionType::ConfidentialTransferMint,
                ExtensionType::ConfidentialTransferFeeConfig,
            ]);

            instructions.extend([
                initialize_permanent_delegate(&token_program, &mint, &issuer)?,
                confidential_transfer::instruction::initialize_mint(
                    &token_program,
                    &mint,
                    Some(issuer),
                    false,
                    confidential
                        .auditor_elgamal_pubkey
                        .map(bytemuck::cast::<[u8; 32], PodElGamalPubkey>),
                )?,
                confidential_transfer_fee::instruction::initialize_confidential_transfer_fee_config(
                    &token_program,
                    &mint,
                    Some(issuer),
                    &bytemuck::cast(confidential.withdraw_withheld_authority_elgamal_pubkey),
                )?,
            ]);
        }

        instructions.push(initialize_mint2(
            &token_program,
            &mint,
            &issuer,
            Some(&issuer),
            args.decimals,
        )?);

        let rent = Rent::get()?;
        let space = ExtensionType::try_calculate_account_len::<Mint>(&extensions)?;

        create_account(
            CpiContext::new(
                self.system_program.to_account_info(),
                CreateAccount {
                    from: self.admin.to_account_info(),
                    to: self.mint.to_account_info(),
                },
            ),
            rent.minimum_balance(space),
            space as u64,
            &token_program,
        )?;

        let accounts = [
            self.mint.to_account_info(),
            self.token_program.to_account_info(),
        ];

        for instruction in &instructions {
            invoke(instruction, &accounts)?;
        }

        let metadata = TokenMetadata {
            update_authority: OptionalNonZeroPubkey::try_from(Some(issuer))?,
            mint,
            name: args.name.clone(),
            symbol: args.symbol.clone(),
            uri: args.uri.clone(),
            additional_metadata: Vec::new(),
        };

        let final_len = {
            let data = self.mint.try_borrow_data()?;
            StateWithExtensions::<Mint>::unpack(&data)?
                .try_get_new_account_len_for_variable_len_extension(&metadata)?
        };

        let shortfall = rent
            .minimum_balance(final_len)
            .saturating_sub(self.mint.lamports());

        if shortfall > 0 {
            transfer(
                CpiContext::new(
                    self.system_program.to_account_info(),
                    Transfer {
                        from: self.admin.to_account_info(),
                        to: self.mint.to_account_info(),
                    },
                ),
                shortfall,
            )?;
        }

        let seeds = self.issuer.signer_seeds();

        invoke_signed(
            &initialize_token_metadata(
                &token_program,
                &mint,
                &issuer,
                &mint,
                &issuer,
                metadata.name,
                metadata.symbol,
                metadata.uri,
            ),
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
