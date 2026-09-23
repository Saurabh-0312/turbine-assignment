use anchor_lang::prelude::*;

pub const ISSUER_SEED: &[u8] = b"issuer";

pub const MAX_FEE_BASIS_POINTS: u16 = 10_000;

#[account]
#[derive(InitSpace)]
pub struct Issuer {
    pub admin: Pubkey,
    pub bump: u8,
}

impl Issuer {
    pub fn signer_seeds(&self) -> [&[u8]; 3] {
        [
            ISSUER_SEED,
            self.admin.as_ref(),
            core::slice::from_ref(&self.bump),
        ]
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct MintArgs {
    pub decimals: u8,
    pub transfer_fee_basis_points: u16,
    pub maximum_fee: u64,
    pub name: String,
    pub symbol: String,
    pub uri: String,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct ConfidentialArgs {
    pub auditor_elgamal_pubkey: Option<[u8; 32]>,
    pub withdraw_withheld_authority_elgamal_pubkey: [u8; 32],
}
