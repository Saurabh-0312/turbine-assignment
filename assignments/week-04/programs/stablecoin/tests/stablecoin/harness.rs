use std::fmt::Debug;
use std::num::NonZeroI8;

use anchor_lang::error::ERROR_CODE_OFFSET;
use anchor_lang::prelude::{Clock, Pubkey};
use anchor_lang::solana_program::instruction::{error::InstructionError, Instruction};
use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use anchor_spl::token_2022::spl_token_2022::{
    self,
    error::TokenError,
    extension::{
        confidential_transfer::{
            account_info::{
                ApplyPendingBalanceAccountInfo, TransferAccountInfo, WithdrawAccountInfo,
            },
            instruction as confidential, ConfidentialTransferAccount, ConfidentialTransferMint,
        },
        confidential_transfer_fee::{ConfidentialTransferFeeAmount, ConfidentialTransferFeeConfig},
        transfer_fee::{TransferFeeAmount, TransferFeeConfig},
        BaseStateWithExtensions, ExtensionType, StateWithExtensions,
    },
    solana_zk_sdk::{
        encryption::{
            auth_encryption::{AeCiphertext, AeKey},
            elgamal::{ElGamalCiphertext, ElGamalKeypair, ElGamalPubkey},
            pod::elgamal::{PodElGamalCiphertext, PodElGamalPubkey},
        },
        zk_elgamal_proof_program::proof_data::PubkeyValidityProofData,
    },
    state::{Account, AccountState, Mint},
};
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_keypair::Keypair;
use solana_program_test::{BanksClientError, ProgramTest, ProgramTestContext};
use solana_signer::Signer;
use solana_transaction::Transaction;
use solana_transaction_error::TransactionError;
use spl_associated_token_account::{
    get_associated_token_address_with_program_id, instruction::create_associated_token_account,
};
use spl_token_confidential_transfer_proof_extraction::instruction::{ProofData, ProofLocation};
use stablecoin::errors::StablecoinError;
use stablecoin::{accounts, instruction, ConfidentialArgs, MintArgs};

pub const DECIMALS: u8 = 6;
pub const UNIT: u64 = 1_000_000;
pub const FEE_BASIS_POINTS: u16 = 100;
pub const MAXIMUM_FEE: u64 = 5 * UNIT;
pub const NAME: &str = "Remit Dollar";
pub const SYMBOL: &str = "RUSD";
pub const URI: &str = "https://raw.githubusercontent.com/Saurabh-0312/turbine-assignment/main/assignments/week-04/assets/rusd.json";

const MAXIMUM_PENDING_CREDITS: u64 = 65_536;
const TRANSFER_AMOUNT_LO_BITS: u32 = 16;

pub struct Harness {
    pub ctx: ProgramTestContext,
    pub admin: Keypair,
    pub issuer: Pubkey,
    nonce: u64,
}

pub struct Holder {
    pub owner: Keypair,
    pub account: Pubkey,
    pub elgamal: ElGamalKeypair,
    pub aes: AeKey,
}

pub struct ConfidentialKeys {
    pub auditor: ElGamalKeypair,
    pub withheld: ElGamalKeypair,
}

impl ConfidentialKeys {
    pub fn new() -> Self {
        Self {
            auditor: ElGamalKeypair::new_rand(),
            withheld: ElGamalKeypair::new_rand(),
        }
    }
}

pub fn mint_args() -> MintArgs {
    MintArgs {
        decimals: DECIMALS,
        transfer_fee_basis_points: FEE_BASIS_POINTS,
        maximum_fee: MAXIMUM_FEE,
        name: NAME.to_string(),
        symbol: SYMBOL.to_string(),
        uri: URI.to_string(),
    }
}

pub fn error_code<T: Debug>(result: Result<T, BanksClientError>) -> u32 {
    match result {
        Err(BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(code),
        ))) => code,
        other => panic!("expected a custom program error, got {other:?}"),
    }
}

pub fn token_error(error: TokenError) -> u32 {
    error as u32
}

pub fn program_error(error: StablecoinError) -> u32 {
    ERROR_CODE_OFFSET + error as u32
}

pub fn decrypt(keypair: &ElGamalKeypair, ciphertext: &PodElGamalCiphertext) -> u64 {
    keypair
        .secret()
        .decrypt_u32(&ElGamalCiphertext::try_from(*ciphertext).unwrap())
        .unwrap()
}

pub fn recombine(keypair: &ElGamalKeypair, lo: &PodElGamalCiphertext, hi: &PodElGamalCiphertext) -> u64 {
    decrypt(keypair, lo) + (decrypt(keypair, hi) << TRANSFER_AMOUNT_LO_BITS)
}

fn pod_bytes(pubkey: &ElGamalPubkey) -> [u8; 32] {
    bytemuck::cast(PodElGamalPubkey::from(*pubkey))
}

fn at<T>(offset: i8, proof: &T) -> ProofLocation<'_, T> {
    ProofLocation::InstructionOffset(
        NonZeroI8::new(offset).unwrap(),
        ProofData::InstructionData(proof),
    )
}

impl Harness {
    pub async fn new() -> Self {
        let mut program_test = ProgramTest::new("stablecoin", stablecoin::ID, None);
        program_test.prefer_bpf(true);

        let ctx = program_test.start_with_context().await;
        let admin = ctx.payer.insecure_clone();
        let issuer = Pubkey::find_program_address(
            &[b"issuer", admin.pubkey().as_ref()],
            &stablecoin::ID,
        )
        .0;

        let mut harness = Self {
            ctx,
            admin,
            issuer,
            nonce: 0,
        };

        let initialize = harness.program(
            accounts::InitializeIssuer {
                admin: harness.admin.pubkey(),
                issuer,
                system_program: system_program::ID,
            },
            instruction::InitializeIssuer {},
        );
        harness.send(&[initialize], &[]).await.unwrap();

        harness
    }

    pub async fn send(
        &mut self,
        instructions: &[Instruction],
        signers: &[&Keypair],
    ) -> Result<(), BanksClientError> {
        self.nonce += 1;

        let mut all = vec![
            ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
            ComputeBudgetInstruction::set_compute_unit_price(self.nonce),
        ];
        all.extend_from_slice(instructions);

        let mut keypairs = vec![&self.admin];
        keypairs.extend_from_slice(signers);

        let blockhash = self.ctx.banks_client.get_latest_blockhash().await?;
        let transaction = Transaction::new_signed_with_payer(
            &all,
            Some(&self.admin.pubkey()),
            &keypairs,
            blockhash,
        );

        self.ctx.banks_client.process_transaction(transaction).await
    }

    fn program(&self, accounts: impl ToAccountMetas, data: impl InstructionData) -> Instruction {
        Instruction {
            program_id: stablecoin::ID,
            accounts: accounts.to_account_metas(None),
            data: data.data(),
        }
    }

    fn create_mint_accounts(&self, mint: &Keypair) -> accounts::CreateMint {
        accounts::CreateMint {
            admin: self.admin.pubkey(),
            issuer: self.issuer,
            mint: mint.pubkey(),
            token_program: spl_token_2022::ID,
            system_program: system_program::ID,
        }
    }

    pub async fn create_mint(&mut self) -> Pubkey {
        let mint = Keypair::new();
        let create = self.program(
            self.create_mint_accounts(&mint),
            instruction::CreateMint { args: mint_args() },
        );
        self.send(&[create], &[&mint]).await.unwrap();

        mint.pubkey()
    }

    pub async fn create_confidential_mint(&mut self, keys: &ConfidentialKeys) -> Pubkey {
        let mint = Keypair::new();
        let create = self.program(
            self.create_mint_accounts(&mint),
            instruction::CreateConfidentialMint {
                args: mint_args(),
                confidential: ConfidentialArgs {
                    auditor_elgamal_pubkey: Some(pod_bytes(keys.auditor.pubkey())),
                    withdraw_withheld_authority_elgamal_pubkey: pod_bytes(keys.withheld.pubkey()),
                },
            },
        );
        self.send(&[create], &[&mint]).await.unwrap();

        mint.pubkey()
    }

    pub async fn holder(&mut self, mint: &Pubkey) -> Holder {
        let owner = Keypair::new();
        let account =
            get_associated_token_address_with_program_id(&owner.pubkey(), mint, &spl_token_2022::ID);

        let create = create_associated_token_account(
            &self.admin.pubkey(),
            &owner.pubkey(),
            mint,
            &spl_token_2022::ID,
        );
        self.send(&[create], &[]).await.unwrap();

        let seed = account.to_bytes();

        Holder {
            elgamal: ElGamalKeypair::new_from_signer(&owner, &seed).unwrap(),
            aes: AeKey::new_from_signer(&owner, &seed).unwrap(),
            owner,
            account,
        }
    }

    pub async fn onboard(&mut self, mint: &Pubkey) -> Holder {
        let holder = self.holder(mint).await;
        self.approve_kyc(mint, &holder.account).await.unwrap();

        holder
    }

    pub async fn onboard_confidential(&mut self, mint: &Pubkey) -> Holder {
        let holder = self.onboard(mint).await;
        self.configure(mint, &holder).await.unwrap();
        self.approve_confidential(mint, &holder.account).await.unwrap();

        holder
    }

    fn compliance(&self, mint: &Pubkey, account: &Pubkey) -> accounts::Compliance {
        accounts::Compliance {
            admin: self.admin.pubkey(),
            issuer: self.issuer,
            mint: *mint,
            token_account: *account,
            token_program: spl_token_2022::ID,
        }
    }

    pub async fn approve_kyc(&mut self, mint: &Pubkey, account: &Pubkey) -> Result<(), BanksClientError> {
        let approve = self.program(self.compliance(mint, account), instruction::ApproveKyc {});
        self.send(&[approve], &[]).await
    }

    pub async fn sanction(&mut self, mint: &Pubkey, account: &Pubkey) -> Result<(), BanksClientError> {
        let sanction = self.program(self.compliance(mint, account), instruction::Sanction {});
        self.send(&[sanction], &[]).await
    }

    pub async fn approve_confidential(
        &mut self,
        mint: &Pubkey,
        account: &Pubkey,
    ) -> Result<(), BanksClientError> {
        let approve = self.program(
            self.compliance(mint, account),
            instruction::ApproveConfidentialAccount {},
        );
        self.send(&[approve], &[]).await
    }

    pub async fn issue(
        &mut self,
        mint: &Pubkey,
        destination: &Pubkey,
        amount: u64,
    ) -> Result<(), BanksClientError> {
        let issue = self.program(
            accounts::Issue {
                admin: self.admin.pubkey(),
                issuer: self.issuer,
                mint: *mint,
                destination: *destination,
                token_program: spl_token_2022::ID,
            },
            instruction::Issue { amount },
        );
        self.send(&[issue], &[]).await
    }

    pub async fn transfer(
        &mut self,
        mint: &Pubkey,
        from: &Holder,
        to: &Pubkey,
        amount: u64,
    ) -> Result<(), BanksClientError> {
        let transfer = self.program(
            accounts::TransferWithFee {
                owner: from.owner.pubkey(),
                mint: *mint,
                source: from.account,
                destination: *to,
                token_program: spl_token_2022::ID,
            },
            instruction::TransferWithFee { amount },
        );
        self.send(&[transfer], &[&from.owner]).await
    }

    pub async fn update_fee(
        &mut self,
        mint: &Pubkey,
        transfer_fee_basis_points: u16,
        maximum_fee: u64,
    ) -> Result<(), BanksClientError> {
        let update = self.program(
            accounts::UpdateTransferFee {
                admin: self.admin.pubkey(),
                issuer: self.issuer,
                mint: *mint,
                token_program: spl_token_2022::ID,
            },
            instruction::UpdateTransferFee {
                transfer_fee_basis_points,
                maximum_fee,
            },
        );
        self.send(&[update], &[]).await
    }

    pub async fn seize(
        &mut self,
        mint: &Pubkey,
        source: &Pubkey,
        treasury: &Pubkey,
        amount: u64,
    ) -> Result<(), BanksClientError> {
        let seize = self.program(
            accounts::Seize {
                admin: self.admin.pubkey(),
                issuer: self.issuer,
                mint: *mint,
                source: *source,
                treasury: *treasury,
                token_program: spl_token_2022::ID,
            },
            instruction::Seize { amount },
        );
        self.send(&[seize], &[]).await
    }

    pub async fn data(&mut self, address: &Pubkey) -> Vec<u8> {
        self.ctx
            .banks_client
            .get_account(*address)
            .await
            .unwrap()
            .expect("account should exist")
            .data
    }

    pub async fn epoch(&mut self) -> u64 {
        self.ctx
            .banks_client
            .get_sysvar::<Clock>()
            .await
            .unwrap()
            .epoch
    }

    pub async fn expected_fee(&mut self, mint: &Pubkey, amount: u64) -> u64 {
        let epoch = self.epoch().await;
        let data = self.data(mint).await;

        StateWithExtensions::<Mint>::unpack(&data)
            .unwrap()
            .get_extension::<TransferFeeConfig>()
            .unwrap()
            .calculate_epoch_fee(epoch, amount)
            .unwrap()
    }

    pub async fn balance(&mut self, account: &Pubkey) -> u64 {
        let data = self.data(account).await;

        StateWithExtensions::<Account>::unpack(&data)
            .unwrap()
            .base
            .amount
    }

    pub async fn withheld(&mut self, account: &Pubkey) -> u64 {
        let data = self.data(account).await;
        let state = StateWithExtensions::<Account>::unpack(&data).unwrap();

        u64::from(state.get_extension::<TransferFeeAmount>().unwrap().withheld_amount)
    }

    pub async fn state(&mut self, account: &Pubkey) -> AccountState {
        let data = self.data(account).await;

        StateWithExtensions::<Account>::unpack(&data)
            .unwrap()
            .base
            .state
    }

    async fn confidential_account(&mut self, account: &Pubkey) -> ConfidentialTransferAccount {
        let data = self.data(account).await;

        *StateWithExtensions::<Account>::unpack(&data)
            .unwrap()
            .get_extension::<ConfidentialTransferAccount>()
            .unwrap()
    }

    pub async fn approved(&mut self, account: &Pubkey) -> bool {
        bool::from(self.confidential_account(account).await.approved)
    }

    pub async fn pending_credits(&mut self, account: &Pubkey) -> u64 {
        u64::from(
            self.confidential_account(account)
                .await
                .pending_balance_credit_counter,
        )
    }

    pub async fn available(&mut self, holder: &Holder) -> u64 {
        let account = self.confidential_account(&holder.account).await;

        holder
            .aes
            .decrypt(&AeCiphertext::try_from(account.decryptable_available_balance).unwrap())
            .unwrap()
    }

    pub async fn confidential_withheld(&mut self, account: &Pubkey, authority: &ElGamalKeypair) -> u64 {
        let data = self.data(account).await;
        let state = StateWithExtensions::<Account>::unpack(&data).unwrap();

        decrypt(
            authority,
            &state
                .get_extension::<ConfidentialTransferFeeAmount>()
                .unwrap()
                .withheld_amount,
        )
    }

    pub async fn reallocate(&mut self, holder: &Holder) -> Result<(), BanksClientError> {
        let reallocate = spl_token_2022::instruction::reallocate(
            &spl_token_2022::ID,
            &holder.account,
            &self.admin.pubkey(),
            &holder.owner.pubkey(),
            &[],
            &[
                ExtensionType::ConfidentialTransferAccount,
                ExtensionType::ConfidentialTransferFeeAmount,
            ],
        )
        .unwrap();

        self.send(&[reallocate], &[&holder.owner]).await
    }

    pub async fn configure_as(
        &mut self,
        mint: &Pubkey,
        account: &Pubkey,
        authority: &Keypair,
        elgamal: &ElGamalKeypair,
        aes: &AeKey,
    ) -> Result<(), BanksClientError> {
        let proof = PubkeyValidityProofData::new(elgamal).unwrap();
        let instructions = confidential::configure_account(
            &spl_token_2022::ID,
            account,
            mint,
            &aes.encrypt(0).into(),
            MAXIMUM_PENDING_CREDITS,
            &authority.pubkey(),
            &[],
            at(1, &proof),
        )
        .unwrap();

        self.send(&instructions, &[authority]).await
    }

    pub async fn configure(&mut self, mint: &Pubkey, holder: &Holder) -> Result<(), BanksClientError> {
        self.reallocate(holder).await?;
        self.configure_as(mint, &holder.account, &holder.owner, &holder.elgamal, &holder.aes)
            .await
    }

    pub async fn deposit(
        &mut self,
        mint: &Pubkey,
        holder: &Holder,
        amount: u64,
    ) -> Result<(), BanksClientError> {
        let deposit = confidential::deposit(
            &spl_token_2022::ID,
            &holder.account,
            mint,
            amount,
            DECIMALS,
            &holder.owner.pubkey(),
            &[],
        )
        .unwrap();

        self.send(&[deposit], &[&holder.owner]).await
    }

    pub async fn apply_pending(&mut self, holder: &Holder) -> Result<(), BanksClientError> {
        let account = self.confidential_account(&holder.account).await;
        let info = ApplyPendingBalanceAccountInfo::new(&account);
        let balance = info
            .new_decryptable_available_balance(holder.elgamal.secret(), &holder.aes)
            .unwrap();

        let apply = confidential::apply_pending_balance(
            &spl_token_2022::ID,
            &holder.account,
            info.pending_balance_credit_counter(),
            &balance.into(),
            &holder.owner.pubkey(),
            &[],
        )
        .unwrap();

        self.send(&[apply], &[&holder.owner]).await
    }

    pub async fn confidential_transfer(
        &mut self,
        mint: &Pubkey,
        from: &Holder,
        to: &Holder,
        amount: u64,
    ) -> Result<(PodElGamalCiphertext, PodElGamalCiphertext), BanksClientError> {
        let epoch = self.epoch().await;
        let mint_data = self.data(mint).await;
        let mint_state = StateWithExtensions::<Mint>::unpack(&mint_data).unwrap();

        let fee = *mint_state
            .get_extension::<TransferFeeConfig>()
            .unwrap()
            .get_epoch_fee(epoch);

        let withheld_key = ElGamalPubkey::try_from(
            mint_state
                .get_extension::<ConfidentialTransferFeeConfig>()
                .unwrap()
                .withdraw_withheld_authority_elgamal_pubkey,
        )
        .unwrap();

        let auditor_key = Option::<PodElGamalPubkey>::from(
            mint_state
                .get_extension::<ConfidentialTransferMint>()
                .unwrap()
                .auditor_elgamal_pubkey,
        )
        .map(|key| ElGamalPubkey::try_from(key).unwrap());

        let source = self.confidential_account(&from.account).await;
        let destination = self.confidential_account(&to.account).await;
        let destination_key = ElGamalPubkey::try_from(destination.elgamal_pubkey).unwrap();

        let info = TransferAccountInfo::new(&source);
        let proofs = info
            .generate_split_transfer_with_fee_proof_data(
                amount,
                &from.elgamal,
                &from.aes,
                &destination_key,
                auditor_key.as_ref(),
                &withheld_key,
                u16::from(fee.transfer_fee_basis_points),
                u64::from(fee.maximum_fee),
            )
            .unwrap();
        let balance = info
            .new_decryptable_available_balance(amount, &from.aes)
            .unwrap();
        let validity = &proofs.transfer_amount_ciphertext_validity_proof_data_with_ciphertext;

        let instructions = confidential::transfer_with_fee(
            &spl_token_2022::ID,
            &from.account,
            mint,
            &to.account,
            &balance.into(),
            &validity.ciphertext_lo,
            &validity.ciphertext_hi,
            &from.owner.pubkey(),
            &[],
            at(1, &proofs.equality_proof_data),
            at(2, &validity.proof_data),
            at(3, &proofs.percentage_with_cap_proof_data),
            at(4, &proofs.fee_ciphertext_validity_proof_data),
            at(5, &proofs.range_proof_data),
        )
        .unwrap();

        self.send(&instructions, &[&from.owner]).await?;

        Ok((validity.ciphertext_lo, validity.ciphertext_hi))
    }

    pub async fn can_prove_withdraw(&mut self, holder: &Holder, amount: u64) -> bool {
        let account = self.confidential_account(&holder.account).await;

        WithdrawAccountInfo::new(&account)
            .generate_proof_data(amount, &holder.elgamal, &holder.aes)
            .is_ok()
    }

    pub async fn withdraw(
        &mut self,
        mint: &Pubkey,
        holder: &Holder,
        amount: u64,
    ) -> Result<(), BanksClientError> {
        let account = self.confidential_account(&holder.account).await;
        let info = WithdrawAccountInfo::new(&account);
        let proofs = info
            .generate_proof_data(amount, &holder.elgamal, &holder.aes)
            .unwrap();
        let balance = info
            .new_decryptable_available_balance(amount, &holder.aes)
            .unwrap();

        let instructions = confidential::withdraw(
            &spl_token_2022::ID,
            &holder.account,
            mint,
            amount,
            DECIMALS,
            &balance.into(),
            &holder.owner.pubkey(),
            &[],
            at(1, &proofs.equality_proof_data),
            at(2, &proofs.range_proof_data),
        )
        .unwrap();

        self.send(&instructions, &[&holder.owner]).await
    }
}
