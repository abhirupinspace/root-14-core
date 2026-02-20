# Root14 (r14)

**Privacy-preserving transactions on Stellar using Groth16 zero-knowledge proofs**

> **Phase 3 — CLI + Indexer + Live E2E**: Full deposit → transfer → balance flow working on Stellar testnet. 30 tests passing.

## Overview

Root14 brings private transactions to Stellar through:
- **Zero-knowledge proofs** (Groth16 + BLS12-381)
- **Soroban smart contract** for on-chain verification
- **UTXO model** with encrypted notes
- **Merkle tree** commitment tracking
- **CLI** for key management, deposits, transfers, balance
- **Indexer** for blockchain event scanning + Merkle tree rebuild

Users can transfer assets privately without revealing amounts, senders, or receivers.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Root14 System                        │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌──────────────┐                  ┌────────────────┐  │
│  │   Client     │                  │  r14-kernel    │  │
│  │  (r14-cli)   │ ─── proof ────►  │  (Soroban)     │  │
│  └──────────────┘                  └────────────────┘  │
│         │                                   │          │
│         │ generate                          │ verify   │
│         ▼                                   ▼          │
│  ┌──────────────┐                  ┌────────────────┐  │
│  │  r14-circuit │                  │  BLS12-381     │  │
│  │  (arkworks)  │                  │  host funcs    │  │
│  └──────────────┘                  └────────────────┘  │
│         │                                              │
│         │ uses                                         │
│         ▼                                              │
│  ┌──────────────┐    ┌──────────────┐                  │
│  │ r14-poseidon │    │  r14-types   │                  │
│  │   (hash)     │    │   (shared)   │                  │
│  └──────────────┘    └──────────────┘                  │
│                                                         │
│  ┌──────────────────────────────────────────────────┐  │
│  │              r14-indexer                          │  │
│  │  Event watcher → Merkle tree → REST API          │  │
│  └──────────────────────────────────────────────────┘  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

## Project Structure

```
r14-dev/
├── crates/
│   ├── r14-kernel/         # ✅ Soroban contract (verifier + deposit + transfer)
│   ├── r14-types/          # ✅ Shared types (Note, Nullifier, Keys, Merkle)
│   ├── r14-poseidon/       # ✅ Poseidon hash (commitment, nullifier, owner)
│   ├── r14-circuit/        # ✅ Off-chain proof generation (7,638 constraints)
│   ├── r14-indexer/        # ✅ Event watcher + Merkle tree + REST API
│   └── r14-cli/            # ✅ CLI: keygen, deposit, transfer, balance, init-contract
│
├── docs and benchmarks/
│   ├── tech.md                         # Technical specification
│   ├── PHASE0_STATUS.md                # Phase 0 status (historical)
│   ├── PHASE0_RESULTS.md               # Early testnet results (historical)
│   ├── PHASE2_STATUS.md                # Phase 2 status
│   └── TESTNET_DEPLOYMENT_SUMMARY.md   # All testnet deployments
│
├── tasks/
│   ├── todo.md             # Task tracker
│   ├── milestones.md       # Hackathon milestones
│   └── lessons.md          # Debugging lessons
│
└── README.md               # This file
```

## Quick Start

### Prerequisites
```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown

# Stellar CLI
cargo install --locked stellar-cli

# Configure network + identity
stellar network add \
  --global testnet \
  --rpc-url https://soroban-testnet.stellar.org:443 \
  --network-passphrase "Test SDF Network ; September 2015"
stellar keys generate test-r14 --network testnet
```

### Build & Test

```bash
# Run all tests (30 tests)
cargo test --workspace

# Build contract WASM
stellar contract build --package r14-kernel
```

### Deploy & Run E2E

```bash
# 1. Deploy contract
stellar contract deploy \
  --wasm target/wasm32v1-none/release/r14_kernel.wasm \
  --network testnet --source test-r14
# → returns CONTRACT_ID

# 2. Generate wallet
cargo run -p r14-cli -- keygen
# Edit ~/.r14/wallet.json: set stellar_secret + contract_id

# 3. Initialize VK on contract
cargo run -p r14-cli -- init-contract

# 4. Deposit
cargo run -p r14-cli -- deposit 1000

# 5. Start indexer (separate terminal)
R14_CONTRACT_ID=<id> cargo run -p r14-indexer

# 6. Check balance (after indexer syncs)
cargo run -p r14-cli -- balance

# 7. Transfer
cargo run -p r14-cli -- transfer 700 <recipient_owner_hash>
```

## Latest: Phase 3 — CLI + Indexer + Live E2E

**Status: SHIPPED**

**Deployed:**
- Contract: `CDV6FRX7GFHZIRYB474LNW4325V7HYHD6WXBDHW4C2XEMCYPT4NF3GPN`
- [Explorer](https://lab.stellar.org/r/testnet/contract/CDV6FRX7GFHZIRYB474LNW4325V7HYHD6WXBDHW4C2XEMCYPT4NF3GPN) (testnet)

**Results:**
- Full E2E: keygen → deposit → indexer sync → balance → transfer (verified on-chain)
- 30 tests passing across 6 crates
- WASM: 10.3KB (built with `stellar contract build`)
- Indexer: event polling, SQLite persistence, Merkle tree rebuild, REST API
- CLI: keygen, deposit, transfer (dry-run + live), balance, init-contract

## Crates

### [r14-kernel](crates/r14-kernel/) ✅
**Soroban smart contract** — Groth16 verifier + deposit + transfer

- `init(vk)` → store VK, `deposit(cm)` → emit event, `transfer(proof, ...)` → verify + nullifier check
- BLS12-381 host functions: `g1_msm`, `pairing_check`
- 10.3KB WASM, 9 tests

[→ Read more](crates/r14-kernel/README.md)

### [r14-types](crates/r14-types/) ✅
**Shared types** — Note, Nullifier, SecretKey, MerklePath

- `no_std` compatible, `std` feature for off-chain
- Note (value, app_tag, owner, nonce), MERKLE_DEPTH=20

[→ Read more](crates/r14-types/README.md)

### [r14-poseidon](crates/r14-poseidon/) ✅
**Poseidon hash** — ZK-friendly hash for BLS12-381

- `commitment()`, `nullifier()`, `owner_hash()`, `hash2()`
- Rate=2, full_rounds=8, partial_rounds=31, alpha=17

[→ Read more](crates/r14-poseidon/README.md)

### [r14-circuit](crates/r14-circuit/) ✅
**Off-chain circuit** — 1-in-2-out transfer, 7,638 constraints

- Merkle inclusion, nullifier, commitment, value conservation
- `setup()` → `prove()` → `verify_offchain()` → `serialize_*_for_soroban()`
- 7 tests

[→ Read more](crates/r14-circuit/README.md)

### [r14-indexer](crates/r14-indexer/) ✅
**Blockchain scanner** — Event watcher + Merkle tree + REST API

- Polls Soroban RPC for deposit/transfer events
- In-memory Poseidon Merkle tree (depth 20), SQLite persistence
- REST API: `/v1/root`, `/v1/proof/:index`, `/v1/leaves`
- Configurable via env vars: `R14_CONTRACT_ID`, `R14_RPC_URL`, `R14_DB_PATH`, `R14_LISTEN_ADDR`
- 11 tests (5 tree + 1 E2E + 5 duplicate bin tests)

### [r14-cli](crates/r14-cli/) ✅
**User CLI** — Full private transaction lifecycle

- `keygen` — generate secret key + owner_hash, create wallet
- `deposit` — create note + submit commitment on-chain
- `transfer` — select note, generate ZK proof, submit on-chain (or `--dry-run`)
- `balance` — sync with indexer, show unspent notes
- `init-contract` — initialize contract with verification key
- Wallet stored at `~/.r14/wallet.json`

## How It Works

### Private Transfer Flow

```
1. Alice deposits 1000 tokens
   ├─► Creates note (value, owner, nonce) locally
   ├─► Computes Poseidon commitment
   └─► Submits commitment to contract (deposit event emitted)

2. Indexer picks up deposit
   ├─► Inserts commitment into Merkle tree
   ├─► Persists to SQLite
   └─► Serves Merkle proofs via REST API

3. Alice transfers 700 to Bob
   ├─► Fetches Merkle proof from indexer
   ├─► Generates ZK proof (proves ownership, value conservation)
   └─► Submits proof + nullifier + new commitments to contract

4. Contract verifies
   ├─► Groth16 pairing check (proof valid?)
   ├─► Nullifier not already spent?
   ├─► Mark nullifier spent, emit transfer event
   └─► Indexer picks up new commitments
```

### Security Model

- **Anonymity:** Sender/receiver hidden
- **Confidentiality:** Amounts encrypted in notes
- **Unlinkability:** Can't trace transaction graph
- **Double-spend prevention:** Nullifier uniqueness enforced on-chain
- **Soundness:** Invalid proofs rejected (Groth16 security)

## Roadmap

- [x] **Phase 0:** Feasibility spike (Groth16 + BLS12-381) — **SHIPPED**
- [x] **Phase 1:** Shared primitives (r14-types + r14-poseidon) — **SHIPPED**
- [x] **Phase 2:** Circuit + kernel integration — **SHIPPED**
- [x] **Phase 3:** CLI + Indexer + Live E2E — **SHIPPED**
- [ ] **Phase 4:** Hardening
  - [ ] On-chain Merkle root tracking (prevent forged inclusion proofs)
  - [ ] Admin auth on `init()`, storage TTL
  - [ ] `#[contractevent]` migration
  - [ ] Historical indexer backfill
  - [ ] Recipient note discovery
- [ ] **Phase 5:** Launch
  - [ ] Audits, trusted setup ceremony
  - [ ] Mainnet deployment

## Technical Specs

**ZK Proof System:**
- Groth16 (trusted setup, 384 byte proofs)
- BLS12-381 elliptic curve
- 7,638 R1CS constraints (transfer circuit)

**On-Chain:**
- Soroban smart contract (10.3KB WASM)
- BLS12-381 host functions (g1_msm, pairing_check)
- 5 entrypoints: init, deposit, transfer, verify_proof, verify_dummy_proof

**Merkle Tree:**
- Depth: 20 (1M capacity)
- Hash: Poseidon (ZK-friendly)
- Sparse tree, SQLite-backed persistence

**Cryptography:**
- Commitments: Poseidon(value, app_tag, owner, nonce)
- Nullifiers: Poseidon(secret_key, nonce)
- Keys: BLS12-381 Fr scalars

## Test Suite (30 tests)

| Crate | Tests | What |
|-------|-------|------|
| r14-circuit | 7 | valid transfer, wrong sk, wrong path, value mismatch, app tag, constraints, serialization |
| r14-kernel | 9 | dummy verify, wrong input, tampered proof (x2), test vectors, E2E transfer, double spend, invalid proof, wrong nullifier |
| r14-poseidon | 6 | determinism, order, nullifier, commitment, nonce sensitivity |
| r14-types | 2 | key gen, note creation |
| r14-indexer | 11 | tree ops (x5), E2E flow, plus bin duplicates |

## Development

```bash
cargo test --workspace          # all 30 tests
stellar contract build -p r14-kernel  # build WASM
cargo fmt --all                 # format
cargo clippy --all-targets      # lint
```

## Testnet Deployments

| Phase | Contract | WASM | Date |
|-------|----------|------|------|
| Phase 3 | `CDV6FRX7GFHZIRYB474LNW4325V7HYHD6WXBDHW4C2XEMCYPT4NF3GPN` | 10.3KB | 2026-02-20 |
| Phase 2 | `CDAXRSKM4VL4MPP7KNPNRDGEU6BWC4KXVXGT4RZ5TNHSQXHJCV3KVGMZ` | 11.8KB | 2026-02-19 |
| Phase 0 | `CC4QPAKN2J6NUCW4QVW5ZA2BOUC4O4KUH6FMFANI34W2N7I7WGEKLZGW` | 6.2KB | 2026-02-17 |

## Resources

**Stellar/Soroban:**
- [Soroban Docs](https://soroban.stellar.org/)
- [BLS12-381 Host Functions](https://docs.rs/soroban-sdk/25.1.1/soroban_sdk/crypto/bls12_381/)

**Zero-Knowledge Proofs:**
- [Groth16 Paper](https://eprint.iacr.org/2016/260.pdf)
- [arkworks](https://arkworks.rs/)
- [Zcash Protocol Spec](https://zips.z.cash/protocol/protocol.pdf)

## License

Apache-2.0

## Security

**Pre-alpha software** — Do not use with real funds

Production deployment requires:
- [ ] Circuit audit
- [ ] Contract audit
- [ ] Trusted setup ceremony (currently using seed=42)
- [ ] On-chain Merkle root validation
- [ ] Testnet stress testing

## Contact

Project maintained by [@abhirupbanerjee](https://github.com/abhirupbanerjee)

---

**Current Status:** Phase 3 shipped — full deposit → transfer → balance E2E on Stellar testnet
