# Turbin3 Builders Cohort — Q3 2026

Assignments, notes, and capstone work for the Turbin3 Builders Cohort.

**Cadet:** Saurabh Singh ([@Saurabh-0312](https://github.com/Saurabh-0312))
**Devnet wallet:** `55FJao825sA7rR9aKNtUEuGzN2gQNN9nZBw41WCWjvwb`

---

## Structure

| Path | Contents |
|---|---|
| [`prereq/`](prereq/) | Prerequisite challenge — Anchor vault + registration CPI |
| [`assignments/`](assignments/) | Weekly assignments, weeks 1–5 |
| [`capstone/`](capstone/) | Capstone: LOI, architecture diagrams, project, demo deck |
| [`notes/`](notes/) | Learning notes and references |

---

## Prerequisite Challenge

Source vault program forked from
[`ShrinathNR/pre-req-vault`](https://github.com/ShrinathNR/pre-req-vault)
@ `f9f7ff7` (vendored into [`prereq/pre-req-vault/`](prereq/pre-req-vault/)).

### Tasks

- [ ] **1.** Understand the vault program — instructions, accounts, state, flow
- [ ] **2.** Extend `withdraw` with a CPI to the registration program, deploy own program
- [ ] **3.** Architecture diagram
- [ ] **4.** Video walkthrough (max 3 min, captioned, YouTube)

### Key addresses

| | |
|---|---|
| Registration program | `TRBZyQHB3m68FGeVsqTK39Wm4xejadjVhP5MAZaKWDM` |
| Registration instruction | `initialize(github: String)` |
| Application PDA seeds | `["prereqs", user]` |
| Vault program (upstream) | `DBoobRVqT7PaAhq8obqo95723CHyWvcCLRUKvQ5wYWE4` — replaced with own deployment |

Registration is **one per wallet** and records the GitHub username `Saurabh-0312`.

---

## Toolchain

| Tool | Version |
|---|---|
| Rust | 1.95.0 |
| Solana CLI | 2.2.7 (Agave) |
| Anchor | 1.1.2 (required by `programs/pre-req-vault/Cargo.toml`) |
| Node | 25.6.0 |

Cluster: **devnet**

```bash
avm install 1.1.2 && avm use 1.1.2
cd prereq/pre-req-vault
anchor build
anchor test
```

---

## Links

- [Solana docs](https://solana.com/docs) · [core concepts](https://solana.com/docs/core)
- [Anchor docs](https://www.anchor-lang.com/docs)
- [Solana Stack Exchange](https://solana.stackexchange.com)
