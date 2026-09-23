use anchor_spl::token_2022::spl_token_2022::{
    error::TokenError,
    extension::{
        default_account_state::DefaultAccountState, transfer_fee::TransferFeeConfig,
        BaseStateWithExtensions, StateWithExtensions,
    },
    state::{AccountState, Mint},
};
use stablecoin::errors::StablecoinError;

use crate::harness::*;

#[tokio::test]
async fn new_accounts_start_frozen_until_kyc_clears() {
    let mut harness = Harness::new().await;
    let mint = harness.create_mint().await;
    let alice = harness.holder(&mint).await;

    assert_eq!(harness.state(&alice.account).await, AccountState::Frozen);
    assert_eq!(
        error_code(harness.issue(&mint, &alice.account, 100 * UNIT).await),
        token_error(TokenError::AccountFrozen)
    );

    harness.approve_kyc(&mint, &alice.account).await.unwrap();
    assert_eq!(harness.state(&alice.account).await, AccountState::Initialized);

    harness.issue(&mint, &alice.account, 100 * UNIT).await.unwrap();
    assert_eq!(harness.balance(&alice.account).await, 100 * UNIT);
}

#[tokio::test]
async fn kyc_thaws_one_account_without_touching_the_mint_default() {
    let mut harness = Harness::new().await;
    let mint = harness.create_mint().await;
    let alice = harness.holder(&mint).await;
    let bob = harness.holder(&mint).await;

    harness.approve_kyc(&mint, &alice.account).await.unwrap();

    assert_eq!(harness.state(&alice.account).await, AccountState::Initialized);
    assert_eq!(harness.state(&bob.account).await, AccountState::Frozen);

    let data = harness.data(&mint).await;
    let state = StateWithExtensions::<Mint>::unpack(&data).unwrap();
    assert_eq!(
        state.get_extension::<DefaultAccountState>().unwrap().state,
        AccountState::Frozen as u8
    );

    assert_eq!(
        error_code(harness.approve_kyc(&mint, &alice.account).await),
        program_error(StablecoinError::AlreadyVerified)
    );
}

#[tokio::test]
async fn transfer_withholds_the_fee_for_the_current_epoch() {
    let mut harness = Harness::new().await;
    let mint = harness.create_mint().await;
    let alice = harness.onboard(&mint).await;
    let bob = harness.onboard(&mint).await;
    harness.issue(&mint, &alice.account, 1_000 * UNIT).await.unwrap();

    let amount = 200 * UNIT;
    let fee = harness.expected_fee(&mint, amount).await;
    assert_eq!(fee, 2 * UNIT);

    harness
        .transfer(&mint, &alice, &bob.account, amount)
        .await
        .unwrap();

    assert_eq!(harness.balance(&alice.account).await, 800 * UNIT);
    assert_eq!(harness.balance(&bob.account).await, amount - fee);
    assert_eq!(harness.withheld(&bob.account).await, fee);
}

#[tokio::test]
async fn transfer_fee_stops_at_the_maximum() {
    let mut harness = Harness::new().await;
    let mint = harness.create_mint().await;
    let alice = harness.onboard(&mint).await;
    let bob = harness.onboard(&mint).await;
    harness.issue(&mint, &alice.account, 10_000 * UNIT).await.unwrap();

    let amount = 2_000 * UNIT;
    assert_eq!(harness.expected_fee(&mint, amount).await, MAXIMUM_FEE);

    harness
        .transfer(&mint, &alice, &bob.account, amount)
        .await
        .unwrap();

    assert_eq!(harness.balance(&bob.account).await, amount - MAXIMUM_FEE);
    assert_eq!(harness.withheld(&bob.account).await, MAXIMUM_FEE);
}

#[tokio::test]
async fn transfer_prices_each_epoch_from_the_live_fee_schedule() {
    let mut harness = Harness::new().await;
    let mint = harness.create_mint().await;
    let alice = harness.onboard(&mint).await;
    let bob = harness.onboard(&mint).await;
    harness.issue(&mint, &alice.account, 1_000 * UNIT).await.unwrap();

    let start = harness.epoch().await;
    harness.update_fee(&mint, 250, MAXIMUM_FEE).await.unwrap();

    let data = harness.data(&mint).await;
    let state = StateWithExtensions::<Mint>::unpack(&data).unwrap();
    let schedule = state.get_extension::<TransferFeeConfig>().unwrap();
    assert_eq!(
        u16::from(schedule.older_transfer_fee.transfer_fee_basis_points),
        FEE_BASIS_POINTS
    );
    assert_eq!(
        u16::from(schedule.newer_transfer_fee.transfer_fee_basis_points),
        250
    );
    assert_eq!(u64::from(schedule.newer_transfer_fee.epoch), start + 2);

    harness
        .transfer(&mint, &alice, &bob.account, 100 * UNIT)
        .await
        .unwrap();
    assert_eq!(harness.withheld(&bob.account).await, UNIT);

    harness.ctx.warp_to_epoch(start + 2).unwrap();

    harness
        .transfer(&mint, &alice, &bob.account, 100 * UNIT)
        .await
        .unwrap();
    assert_eq!(harness.withheld(&bob.account).await, UNIT + 2_500_000);
}

#[tokio::test]
async fn transfer_to_an_account_that_has_not_cleared_kyc_fails() {
    let mut harness = Harness::new().await;
    let mint = harness.create_mint().await;
    let alice = harness.onboard(&mint).await;
    let bob = harness.holder(&mint).await;
    harness.issue(&mint, &alice.account, 100 * UNIT).await.unwrap();

    assert_eq!(
        error_code(harness.transfer(&mint, &alice, &bob.account, 10 * UNIT).await),
        token_error(TokenError::AccountFrozen)
    );
    assert_eq!(harness.balance(&alice.account).await, 100 * UNIT);
}
