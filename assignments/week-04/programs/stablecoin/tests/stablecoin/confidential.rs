use anchor_lang::prelude::Pubkey;
use anchor_spl::token_2022::spl_token_2022::{
    self,
    error::TokenError,
    extension::{
        confidential_transfer::{self, ConfidentialTransferMint},
        confidential_transfer_fee::ConfidentialTransferFeeConfig,
        default_account_state, metadata_pointer,
        permanent_delegate::PermanentDelegate,
        transfer_fee, BaseStateWithExtensions, ExtensionType, StateWithExtensions,
    },
    instruction::{initialize_mint2, initialize_mint_close_authority, initialize_permanent_delegate},
    solana_zk_sdk::encryption::{
        auth_encryption::AeKey, elgamal::ElGamalKeypair, pod::elgamal::PodElGamalPubkey,
    },
    state::{AccountState, Mint},
};
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_system_interface::instruction::create_account;

use crate::harness::*;

#[tokio::test]
async fn carrying_the_fee_into_a_confidential_mint_needs_the_confidential_fee_extension() {
    let mut harness = Harness::new().await;
    let mint = Keypair::new();
    let authority = harness.admin.pubkey();
    let token_program = spl_token_2022::ID;

    let extensions = [
        ExtensionType::TransferFeeConfig,
        ExtensionType::MetadataPointer,
        ExtensionType::DefaultAccountState,
        ExtensionType::MintCloseAuthority,
        ExtensionType::PermanentDelegate,
        ExtensionType::ConfidentialTransferMint,
    ];
    let space = ExtensionType::try_calculate_account_len::<Mint>(&extensions).unwrap();
    let lamports = harness
        .ctx
        .banks_client
        .get_rent()
        .await
        .unwrap()
        .minimum_balance(space);

    let instructions = [
        create_account(&authority, &mint.pubkey(), lamports, space as u64, &token_program),
        transfer_fee::instruction::initialize_transfer_fee_config(
            &token_program,
            &mint.pubkey(),
            Some(&authority),
            Some(&authority),
            FEE_BASIS_POINTS,
            MAXIMUM_FEE,
        )
        .unwrap(),
        metadata_pointer::instruction::initialize(
            &token_program,
            &mint.pubkey(),
            Some(authority),
            Some(mint.pubkey()),
        )
        .unwrap(),
        default_account_state::instruction::initialize_default_account_state(
            &token_program,
            &mint.pubkey(),
            &AccountState::Frozen,
        )
        .unwrap(),
        initialize_mint_close_authority(&token_program, &mint.pubkey(), Some(&authority)).unwrap(),
        initialize_permanent_delegate(&token_program, &mint.pubkey(), &authority).unwrap(),
        confidential_transfer::instruction::initialize_mint(
            &token_program,
            &mint.pubkey(),
            Some(authority),
            false,
            None,
        )
        .unwrap(),
        initialize_mint2(
            &token_program,
            &mint.pubkey(),
            &authority,
            Some(&authority),
            DECIMALS,
        )
        .unwrap(),
    ];

    assert_eq!(
        error_code(harness.send(&instructions, &[&mint]).await),
        token_error(TokenError::InvalidExtensionCombination)
    );
}

#[tokio::test]
async fn reissued_mint_carries_every_extension_forward() {
    let mut harness = Harness::new().await;
    let keys = ConfidentialKeys::new();
    let mint = harness.create_confidential_mint(&keys).await;

    let data = harness.data(&mint).await;
    let state = StateWithExtensions::<Mint>::unpack(&data).unwrap();

    let expected = [
        ExtensionType::TransferFeeConfig,
        ExtensionType::MetadataPointer,
        ExtensionType::DefaultAccountState,
        ExtensionType::MintCloseAuthority,
        ExtensionType::TokenMetadata,
        ExtensionType::PermanentDelegate,
        ExtensionType::ConfidentialTransferMint,
        ExtensionType::ConfidentialTransferFeeConfig,
    ];
    let extensions = state.get_extension_types().unwrap();
    assert_eq!(extensions.len(), expected.len());
    for extension in expected {
        assert!(extensions.contains(&extension), "missing {extension:?}");
    }

    let delegate = state.get_extension::<PermanentDelegate>().unwrap();
    assert_eq!(Option::<Pubkey>::from(delegate.delegate), Some(harness.issuer));

    let confidential = state.get_extension::<ConfidentialTransferMint>().unwrap();
    assert_eq!(
        Option::<Pubkey>::from(confidential.authority),
        Some(harness.issuer)
    );
    assert!(!bool::from(confidential.auto_approve_new_accounts));
    assert_eq!(
        Option::<PodElGamalPubkey>::from(confidential.auditor_elgamal_pubkey),
        Some(PodElGamalPubkey::from(*keys.auditor.pubkey()))
    );

    let confidential_fee = state
        .get_extension::<ConfidentialTransferFeeConfig>()
        .unwrap();
    assert_eq!(
        Option::<Pubkey>::from(confidential_fee.authority),
        Some(harness.issuer)
    );
    assert_eq!(
        confidential_fee.withdraw_withheld_authority_elgamal_pubkey,
        PodElGamalPubkey::from(*keys.withheld.pubkey())
    );
}

#[tokio::test]
async fn only_the_owner_can_configure_a_confidential_account() {
    let mut harness = Harness::new().await;
    let keys = ConfidentialKeys::new();
    let mint = harness.create_confidential_mint(&keys).await;
    let alice = harness.onboard(&mint).await;
    harness.reallocate(&alice).await.unwrap();

    let mallory = Keypair::new();
    assert_eq!(
        error_code(
            harness
                .configure_as(
                    &mint,
                    &alice.account,
                    &mallory,
                    &ElGamalKeypair::new_rand(),
                    &AeKey::new_rand(),
                )
                .await
        ),
        token_error(TokenError::OwnerMismatch)
    );

    harness
        .configure_as(&mint, &alice.account, &alice.owner, &alice.elgamal, &alice.aes)
        .await
        .unwrap();
    assert!(!harness.approved(&alice.account).await);

    harness
        .approve_confidential(&mint, &alice.account)
        .await
        .unwrap();
    assert!(harness.approved(&alice.account).await);
}

#[tokio::test]
async fn an_unapproved_account_cannot_take_a_confidential_deposit() {
    let mut harness = Harness::new().await;
    let keys = ConfidentialKeys::new();
    let mint = harness.create_confidential_mint(&keys).await;
    let alice = harness.onboard(&mint).await;
    harness.configure(&mint, &alice).await.unwrap();
    harness.issue(&mint, &alice.account, 100 * UNIT).await.unwrap();

    assert_eq!(
        error_code(harness.deposit(&mint, &alice, 100 * UNIT).await),
        token_error(TokenError::ConfidentialTransferAccountNotApproved)
    );

    harness
        .approve_confidential(&mint, &alice.account)
        .await
        .unwrap();
    harness.deposit(&mint, &alice, 100 * UNIT).await.unwrap();

    assert_eq!(harness.balance(&alice.account).await, 0);
    assert_eq!(harness.pending_credits(&alice.account).await, 1);
}

#[tokio::test]
async fn pending_funds_must_be_applied_before_they_can_be_withdrawn() {
    let mut harness = Harness::new().await;
    let keys = ConfidentialKeys::new();
    let mint = harness.create_confidential_mint(&keys).await;
    let alice = harness.onboard_confidential(&mint).await;
    harness.issue(&mint, &alice.account, 500 * UNIT).await.unwrap();
    harness.deposit(&mint, &alice, 500 * UNIT).await.unwrap();

    assert_eq!(harness.available(&alice).await, 0);
    assert!(!harness.can_prove_withdraw(&alice, 100 * UNIT).await);

    harness.apply_pending(&alice).await.unwrap();
    assert_eq!(harness.available(&alice).await, 500 * UNIT);
    assert!(harness.can_prove_withdraw(&alice, 100 * UNIT).await);

    harness.withdraw(&mint, &alice, 100 * UNIT).await.unwrap();
    assert_eq!(harness.balance(&alice.account).await, 100 * UNIT);
    assert_eq!(harness.available(&alice).await, 400 * UNIT);
}

#[tokio::test]
async fn runs_the_full_confidential_lifecycle() {
    let mut harness = Harness::new().await;
    let keys = ConfidentialKeys::new();
    let mint = harness.create_confidential_mint(&keys).await;
    let alice = harness.onboard_confidential(&mint).await;
    let bob = harness.onboard_confidential(&mint).await;
    harness.issue(&mint, &alice.account, 1_000 * UNIT).await.unwrap();

    harness.deposit(&mint, &alice, 600 * UNIT).await.unwrap();
    assert_eq!(harness.balance(&alice.account).await, 400 * UNIT);
    assert_eq!(harness.pending_credits(&alice.account).await, 1);
    assert_eq!(harness.available(&alice).await, 0);

    harness.apply_pending(&alice).await.unwrap();
    assert_eq!(harness.pending_credits(&alice.account).await, 0);
    assert_eq!(harness.available(&alice).await, 600 * UNIT);

    let amount = 200 * UNIT;
    let fee = harness.expected_fee(&mint, amount).await;
    let (auditor_lo, auditor_hi) = harness
        .confidential_transfer(&mint, &alice, &bob, amount)
        .await
        .unwrap();

    assert_eq!(harness.available(&alice).await, 400 * UNIT);
    assert_eq!(harness.balance(&alice.account).await, 400 * UNIT);
    assert_eq!(harness.balance(&bob.account).await, 0);
    assert_eq!(harness.pending_credits(&bob.account).await, 1);
    assert_eq!(recombine(&keys.auditor, &auditor_lo, &auditor_hi), amount);
    assert_eq!(
        harness
            .confidential_withheld(&bob.account, &keys.withheld)
            .await,
        fee
    );

    harness.apply_pending(&bob).await.unwrap();
    assert_eq!(harness.available(&bob).await, amount - fee);

    harness.withdraw(&mint, &bob, 50 * UNIT).await.unwrap();
    assert_eq!(harness.balance(&bob.account).await, 50 * UNIT);
    assert_eq!(harness.available(&bob).await, amount - fee - 50 * UNIT);
}
