use anchor_lang::prelude::*;

pub mod errors;
pub mod instructions;
pub mod plugins;
pub mod state;

pub use instructions::*;
pub use state::*;

declare_id!("6YezEsD1WUPGhJJpXrrWtkgbn68s71dgFqv3vwPzBJVB");

#[program]
pub mod nft_staking {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, args: InitializeArgs) -> Result<()> {
        ctx.accounts.initialize(args, &ctx.bumps)
    }

    pub fn mint_asset(ctx: Context<MintAsset>, name: String, uri: String) -> Result<()> {
        ctx.accounts.mint_asset(name, uri)
    }

    pub fn stake(ctx: Context<Stake>) -> Result<()> {
        ctx.accounts.stake()
    }

    pub fn claim_rewards(ctx: Context<Position>) -> Result<()> {
        ctx.accounts.claim_rewards()
    }

    pub fn unstake(ctx: Context<Position>) -> Result<()> {
        ctx.accounts.unstake()
    }

    pub fn burn_staked_nft(ctx: Context<Position>) -> Result<()> {
        ctx.accounts.burn_staked_nft()
    }
}
