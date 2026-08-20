# Vault Program — Architecture

Program ID: `BUrv79rKUhUxJgnvuxf6TNYkWPgc5NVKfxUD2kgjJbua` (devnet)

A per-user SOL vault. Each user opens their own vault, deposits and withdraws SOL,
and closes it to reclaim everything. `withdraw` additionally registers the user's
GitHub username with the Turbin3 registration program via CPI.

## Accounts

| Account | Type | Seeds | Holds |
|---|---|---|---|
| `vault_state` | `Account<VaultState>` | `["state", user]` | `vault_bump`, `state_bump` (10 bytes total) |
| `vault` | `SystemAccount` | `["vault", vault_state]` | SOL only, no data |
| `application_account` | `UncheckedAccount` | `["prereqs", user]`, derived against the **registration** program | `ApplicationAccount` written by the CPI |

Data and lamports are deliberately split. A data account must stay rent-exempt, so
draining it to zero would delete it. Keeping SOL in a separate `SystemAccount` lets
the balance go to zero safely.

The derivation chain is two links deep:

```
user  --["state", user]-->  vault_state  --["vault", vault_state]-->  vault
```

`initialize` derives both bumps once and stores them in `vault_state`; every later
instruction reads them back via `bump = vault_state.*_bump` instead of recomputing.

## Instructions

| Instruction | Accounts | SOL movement | Signer |
|---|---|---|---|
| `initialize` | user, vault_state, vault, system_program | user pays rent for `vault_state` | user |
| `deposit(amount)` | user, vault, vault_state, system_program | user -> vault | user |
| `withdraw(amount)` | user, vault, vault_state, application_account, application_program, system_program | vault -> user | `vault` PDA (seeds) |
| `close` | user, vault, vault_state, system_program | vault -> user (all), `vault_state` rent refunded | `vault` PDA (seeds) |

## Signing

Two distinct mechanisms, both present in this program:

- `CpiContext::new` — used by `deposit`. SOL leaves `user`, whose real signature is
  already on the transaction.
- `CpiContext::new_with_signer` — used by `withdraw` and `close`. SOL leaves the
  `vault` PDA, which has no private key. The program proves ownership by passing
  `["vault", vault_state, bump]` as signer seeds.

Rule: money leaving a user -> `new`. Money leaving a PDA -> `new_with_signer`.

## The withdraw CPI

After transferring SOL to the user, `withdraw` invokes the registration program's
`initialize(github: String)`:

| Registration program | |
|---|---|
| Program ID | `TRBZyQHB3m68FGeVsqTK39Wm4xejadjVhP5MAZaKWDM` |
| Instruction | `initialize(github: String)` |
| Accounts | `user` (signer, writable), `account` (writable PDA), `system_program` |
| Writes | `ApplicationAccount { user, bump, pre_req_ts, pre_req_rs, github }` |

This CPI uses `CpiContext::new` — the required signer is `user`, not a PDA.
`declare_program!(registration)` generates the Rust bindings from
`idls/registration.json`.

Because the CPI is inside the same instruction, it is atomic: if registration fails,
the SOL transfer reverts too.

## Invocation depth during withdraw

```
[1] pre_req_vault : Withdraw
    [2] system_program : transfer          vault -> user
    [2] registration : Initialize
        [3] system_program : create_account   allocate application_account
```

Measured on devnet: 28,382 of 200,000 compute units.

## Module layout

```
lib.rs            declare_id! + #[program] routing to the four instructions
instructions.rs   re-exports the four instruction modules
instructions/
  initialize.rs   creates vault_state, saves both bumps
  deposit.rs      user -> vault transfer
  withdraw.rs     vault -> user transfer, then the registration CPI
  close.rs        drains vault, closes vault_state
state.rs          VaultState { vault_bump, state_bump }
constants.rs      unused (leftover from a counter template)
error.rs          unused (leftover from a counter template)
```

Every instruction file imports `crate::state::VaultState`. Nothing imports
`constants.rs` or `error.rs`.
