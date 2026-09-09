use anchor_lang::prelude::*;

pub mod errors;
pub mod instructions;
pub mod state;

pub use instructions::*;
pub use state::*;

declare_id!("FnzppdD95wjWgbQJ6TfW5zmvngVCtfrJEEeG4zULfjfR");

#[program]
pub mod escrow {
    use super::*;

    pub fn make(
        ctx: Context<Make>,
        seed: u64,
        deposit: u64,
        receive: u64,
        expiry: i64,
    ) -> Result<()> {
        ctx.accounts.make(seed, deposit, receive, expiry, &ctx.bumps)
    }

    pub fn take(ctx: Context<Take>) -> Result<()> {
        ctx.accounts.take()
    }

    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        ctx.accounts.refund()
    }

    pub fn update(ctx: Context<Update>, receive: u64, expiry: i64) -> Result<()> {
        ctx.accounts.update(receive, expiry)
    }
}
