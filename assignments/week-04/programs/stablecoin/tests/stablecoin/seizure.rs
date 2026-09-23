use anchor_spl::token_2022::spl_token_2022::{error::TokenError, state::AccountState};
use stablecoin::errors::StablecoinError;

use crate::harness::*;

#[tokio::test]
async fn seizure_moves_the_public_balance_out_of_a_sanctioned_wallet() {
    let mut harness = Harness::new().await;
    let keys = ConfidentialKeys::new();
    let mint = harness.create_confidential_mint(&keys).await;
    let alice = harness.onboard(&mint).await;
    let treasury = harness.onboard(&mint).await;
    harness.issue(&mint, &alice.account, 1_000 * UNIT).await.unwrap();

    harness.sanction(&mint, &alice.account).await.unwrap();
    assert_eq!(harness.state(&alice.account).await, AccountState::Frozen);
    assert_eq!(
        error_code(
            harness
                .transfer(&mint, &alice, &treasury.account, 10 * UNIT)
                .await
        ),
        token_error(TokenError::AccountFrozen)
    );

    let amount = 400 * UNIT;
    let fee = harness.expected_fee(&mint, amount).await;
    harness
        .seize(&mint, &alice.account, &treasury.account, amount)
        .await
        .unwrap();

    assert_eq!(harness.balance(&alice.account).await, 600 * UNIT);
    assert_eq!(harness.balance(&treasury.account).await, amount - fee);
    assert_eq!(harness.withheld(&treasury.account).await, fee);
    assert_eq!(harness.state(&alice.account).await, AccountState::Frozen);
}

#[tokio::test]
async fn seizure_cannot_reach_a_confidential_balance() {
    let mut harness = Harness::new().await;
    let keys = ConfidentialKeys::new();
    let mint = harness.create_confidential_mint(&keys).await;
    let alice = harness.onboard_confidential(&mint).await;
    let bob = harness.onboard_confidential(&mint).await;
    let treasury = harness.onboard(&mint).await;
    harness.issue(&mint, &alice.account, 1_000 * UNIT).await.unwrap();

    harness.deposit(&mint, &alice, 600 * UNIT).await.unwrap();
    harness.apply_pending(&alice).await.unwrap();
    assert_eq!(harness.balance(&alice.account).await, 400 * UNIT);
    assert_eq!(harness.available(&alice).await, 600 * UNIT);

    harness.sanction(&mint, &alice.account).await.unwrap();

    assert_eq!(
        error_code(
            harness
                .seize(&mint, &alice.account, &treasury.account, 600 * UNIT)
                .await
        ),
        program_error(StablecoinError::InsufficientPublicBalance)
    );

    harness
        .seize(&mint, &alice.account, &treasury.account, 400 * UNIT)
        .await
        .unwrap();
    assert_eq!(harness.balance(&alice.account).await, 0);
    assert_eq!(harness.available(&alice).await, 600 * UNIT);

    assert_eq!(
        error_code(
            harness
                .confidential_transfer(&mint, &alice, &bob, 100 * UNIT)
                .await
        ),
        token_error(TokenError::AccountFrozen)
    );
}

#[tokio::test]
async fn seizure_needs_a_permanent_delegate_on_the_mint() {
    let mut harness = Harness::new().await;
    let mint = harness.create_mint().await;
    let alice = harness.onboard(&mint).await;
    let treasury = harness.onboard(&mint).await;
    harness.issue(&mint, &alice.account, 100 * UNIT).await.unwrap();

    assert_eq!(
        error_code(
            harness
                .seize(&mint, &alice.account, &treasury.account, 10 * UNIT)
                .await
        ),
        program_error(StablecoinError::NotSeizable)
    );
    assert_eq!(harness.balance(&alice.account).await, 100 * UNIT);
}
