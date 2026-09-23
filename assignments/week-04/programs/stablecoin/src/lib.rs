use anchor_lang::prelude::*;

pub mod errors;
pub mod fees;
pub mod instructions;
pub mod state;

pub use instructions::*;
pub use state::*;

declare_id!("ZsyntkiEGwSPtXwAUe72QhuPBhyi91jA4244YKpuNc4");

#[program]
pub mod stablecoin {
    use super::*;

    pub fn initialize_issuer(ctx: Context<InitializeIssuer>) -> Result<()> {
        ctx.accounts.initialize_issuer(ctx.bumps.issuer)
    }

    pub fn create_mint(ctx: Context<CreateMint>, args: MintArgs) -> Result<()> {
        ctx.accounts.create_mint(&args, None)
    }

    pub fn create_confidential_mint(
        ctx: Context<CreateMint>,
        args: MintArgs,
        confidential: ConfidentialArgs,
    ) -> Result<()> {
        ctx.accounts.create_mint(&args, Some(&confidential))
    }

    pub fn issue(ctx: Context<Issue>, amount: u64) -> Result<()> {
        ctx.accounts.issue(amount)
    }

    pub fn approve_kyc(ctx: Context<Compliance>) -> Result<()> {
        ctx.accounts.approve_kyc()
    }

    pub fn sanction(ctx: Context<Compliance>) -> Result<()> {
        ctx.accounts.sanction()
    }

    pub fn approve_confidential_account(ctx: Context<Compliance>) -> Result<()> {
        ctx.accounts.approve_confidential_account()
    }

    pub fn transfer_with_fee(ctx: Context<TransferWithFee>, amount: u64) -> Result<()> {
        ctx.accounts.transfer_with_fee(amount)
    }

    pub fn update_transfer_fee(
        ctx: Context<UpdateTransferFee>,
        transfer_fee_basis_points: u16,
        maximum_fee: u64,
    ) -> Result<()> {
        ctx.accounts
            .update_transfer_fee(transfer_fee_basis_points, maximum_fee)
    }

    pub fn seize(ctx: Context<Seize>, amount: u64) -> Result<()> {
        ctx.accounts.seize(amount)
    }
}
