use anchor_lang::prelude::*;
use anchor_spl::token_2022::spl_token_2022::{
    extension::{transfer_fee::TransferFeeConfig, BaseStateWithExtensions, StateWithExtensions},
    state::Mint,
};

use crate::errors::StablecoinError;

pub fn decimals_and_epoch_fee(mint: &AccountInfo, amount: u64) -> Result<(u8, u64)> {
    let data = mint.try_borrow_data()?;
    let state = StateWithExtensions::<Mint>::unpack(&data)?;

    let fee = state
        .get_extension::<TransferFeeConfig>()?
        .calculate_epoch_fee(Clock::get()?.epoch, amount)
        .ok_or(StablecoinError::FeeOverflow)?;

    Ok((state.base.decimals, fee))
}

pub fn decimals(mint: &AccountInfo) -> Result<u8> {
    let data = mint.try_borrow_data()?;

    Ok(StateWithExtensions::<Mint>::unpack(&data)?.base.decimals)
}
