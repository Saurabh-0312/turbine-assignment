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
| 3 | — | Not started |
| 4 | — | Not started |
| 5 | — | Not started |

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
