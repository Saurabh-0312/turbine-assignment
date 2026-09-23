use std::mem::size_of;

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::program_option::COption;
use anchor_spl::token_2022::spl_token_2022::{
    extension::{
        default_account_state::DefaultAccountState, metadata_pointer::MetadataPointer,
        mint_close_authority::MintCloseAuthority, transfer_fee::TransferFeeConfig,
        BaseStateWithExtensions, ExtensionType, Length, StateWithExtensions,
    },
    state::{AccountState, Mint},
};
use anchor_spl::token_2022_extensions::spl_token_metadata_interface::state::TokenMetadata;

use crate::harness::*;

const FIXED_EXTENSIONS: [ExtensionType; 4] = [
    ExtensionType::TransferFeeConfig,
    ExtensionType::MetadataPointer,
    ExtensionType::DefaultAccountState,
    ExtensionType::MintCloseAuthority,
];

#[tokio::test]
async fn stacks_the_four_extensions_on_a_single_mint() {
    let mut harness = Harness::new().await;
    let mint = harness.create_mint().await;

    let data = harness.data(&mint).await;
    let state = StateWithExtensions::<Mint>::unpack(&data).unwrap();

    let extensions = state.get_extension_types().unwrap();
    assert_eq!(extensions.len(), FIXED_EXTENSIONS.len() + 1);
    for expected in FIXED_EXTENSIONS {
        assert!(extensions.contains(&expected), "missing {expected:?}");
    }
    assert!(extensions.contains(&ExtensionType::TokenMetadata));

    let fee = state.get_extension::<TransferFeeConfig>().unwrap();
    assert_eq!(
        u16::from(fee.newer_transfer_fee.transfer_fee_basis_points),
        FEE_BASIS_POINTS
    );
    assert_eq!(u64::from(fee.newer_transfer_fee.maximum_fee), MAXIMUM_FEE);
    assert_eq!(
        Option::<Pubkey>::from(fee.transfer_fee_config_authority),
        Some(harness.issuer)
    );
    assert_eq!(
        Option::<Pubkey>::from(fee.withdraw_withheld_authority),
        Some(harness.issuer)
    );

    let pointer = state.get_extension::<MetadataPointer>().unwrap();
    assert_eq!(Option::<Pubkey>::from(pointer.metadata_address), Some(mint));
    assert_eq!(Option::<Pubkey>::from(pointer.authority), Some(harness.issuer));

    let default_state = state.get_extension::<DefaultAccountState>().unwrap();
    assert_eq!(default_state.state, AccountState::Frozen as u8);

    let close_authority = state.get_extension::<MintCloseAuthority>().unwrap();
    assert_eq!(
        Option::<Pubkey>::from(close_authority.close_authority),
        Some(harness.issuer)
    );

    assert_eq!(state.base.mint_authority, COption::Some(harness.issuer));
    assert_eq!(state.base.freeze_authority, COption::Some(harness.issuer));
    assert_eq!(state.base.decimals, DECIMALS);
}

#[tokio::test]
async fn keeps_its_metadata_inside_the_mint_account() {
    let mut harness = Harness::new().await;
    let mint = harness.create_mint().await;

    let data = harness.data(&mint).await;
    let state = StateWithExtensions::<Mint>::unpack(&data).unwrap();
    let metadata = state.get_variable_len_extension::<TokenMetadata>().unwrap();

    assert_eq!(metadata.name, NAME);
    assert_eq!(metadata.symbol, SYMBOL);
    assert_eq!(metadata.uri, URI);
    assert_eq!(metadata.mint, mint);
    assert_eq!(
        Option::<Pubkey>::from(metadata.update_authority),
        Some(harness.issuer)
    );
}

#[tokio::test]
async fn sizes_the_account_from_its_extension_list() {
    let mut harness = Harness::new().await;
    let mint = harness.create_mint().await;

    let data = harness.data(&mint).await;
    let state = StateWithExtensions::<Mint>::unpack(&data).unwrap();
    let metadata = state.get_variable_len_extension::<TokenMetadata>().unwrap();

    let packed_metadata = borsh::to_vec(&metadata).unwrap();
    let metadata_entry = size_of::<ExtensionType>() + size_of::<Length>() + packed_metadata.len();

    let fixed = ExtensionType::try_calculate_account_len::<Mint>(&FIXED_EXTENSIONS).unwrap();
    assert_eq!(data.len(), fixed + metadata_entry);
    assert!(metadata.tlv_size_of().unwrap() > metadata_entry);

    let account = harness
        .ctx
        .banks_client
        .get_account(mint)
        .await
        .unwrap()
        .unwrap();
    let rent = harness.ctx.banks_client.get_rent().await.unwrap();
    assert_eq!(account.lamports, rent.minimum_balance(account.data.len()));
}
