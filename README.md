# Turbin3 Builders Cohort — Q3 2026

Assignments, notes, and capstone work.

**Cadet:** Saurabh Singh ([@Saurabh-0312](https://github.com/Saurabh-0312))
All work runs against **Solana devnet**.

---

## Assignments

| Week | Assignment | Status |
|---|---|---|
| **1** | [SPL token and MPL Core NFT](assignments/week-01/) | Submitted |
| **2** | [Vault and escrow programs](assignments/week-02/) | Submitted |
| **3** | [Constant product AMM](assignments/week-03/) | Submitted |
| **4** | [Token-2022 remittance stablecoin](assignments/week-04/) | Submitted |
| **5** | [Metaplex Core NFT staking](assignments/week-05/) | Submitted |

### Week 1 — SPL token and MPL Core NFT

Minted a 6-decimal SPL token with on-chain metadata and transferred it to a second
wallet, then minted an MPL Core NFT and updated its name and metadata URI as the
update authority. Verified by 13 tests that query devnet directly.

| | |
|---|---|
| Token mint | [`AjbayCwY…hBQFu`](https://explorer.solana.com/address/AjbayCwY5udQxpgn6Jg6yFMVZxCgye96QWQT225hBQFu?cluster=devnet) |
| NFT asset | [`2vMZfV9J…cFxTs`](https://explorer.solana.com/address/2vMZfV9JmABqgVZrZYPjxPZFeVCguKF3nvBRWLPcFxTs?cluster=devnet) |

Full write-up: [`assignments/week-01/README.md`](assignments/week-01/README.md)

### Week 2 — Vault and escrow

Two Anchor programs: a native SOL vault with deposit, withdraw and close, and an SPL
token escrow with make, take, refund and update. The escrow carries a deadline read
from the clock sysvar, so a stale offer can no longer be filled. Covered by 13 tests
against a local validator.

Full write-up: [`assignments/week-02/README.md`](assignments/week-02/README.md)

### Week 3 — Constant product AMM

A pool over `x * y = k` with deposit, withdraw and swap. Two separate fees: the swap
fee stays in the vaults and grows every liquidity position, while a protocol fee is
skimmed before the curve into treasury accounts only the pool authority can claim.
Covered by 14 tests against a local validator.

Full write-up: [`assignments/week-03/README.md`](assignments/week-03/README.md)

### Week 4 — Token-2022 remittance stablecoin

A stablecoin mint stacking a transfer fee, on-chain metadata, frozen-by-default
accounts and a close authority, with a KYC thaw path and transfers priced from the
live epoch. A re-issue adds confidential transfers and a seizure authority, and the
confidential lifecycle runs end to end with real zero-knowledge proofs. Covered by
18 tests on `solana-program-test`.

Full write-up: [`assignments/week-04/README.md`](assignments/week-04/README.md)

### Week 5 — Metaplex Core NFT staking

Stakes Core NFTs by freezing them under a program-controlled freeze delegate.
Holders can claim rewards without unstaking, burn a staked NFT through a burn
delegate for a one-time bonus, and the collection itself carries a `total_staked`
attribute that moves with every stake, unstake and burn. Covered by 17 tests
against a local validator running Metaplex Core.

Full write-up: [`assignments/week-05/README.md`](assignments/week-05/README.md)

---

## Repository layout

| Path | Contents |
|---|---|
| [`assignments/`](assignments/) | Weekly assignments, weeks 1–5 |
| [`capstone/`](capstone/) | Capstone project |
| [`notes/`](notes/) | Learning notes |
| [`prereq/`](prereq/) | Prerequisite challenge |

## References

- [Solana docs](https://solana.com/docs) · [core concepts](https://solana.com/docs/core)
- [Anchor docs](https://www.anchor-lang.com/docs) · [Metaplex Core](https://developers.metaplex.com/core)
- [Solana Stack Exchange](https://solana.stackexchange.com)
