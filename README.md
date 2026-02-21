# Root14 (r14)

**Privacy-preserving transactions on Stellar using Groth16 zero-knowledge proofs**

> **Phase 3.5 — Standard Extraction**: r14-kernel split into r14-core (verifier registry) + r14-transfer (privacy app) + r14-sdk (serialization). Both contracts deployed to testnet. 39 tests passing.

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
┌──────────────────────────────────────────────────────────────┐
│                      Root14 System                           │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐     ┌─────────────────────────────────┐   │
│  │   Client     │     │  On-Chain (Soroban)              │   │
│  │  (r14-cli)   │     │                                  │   │
│  │              │     │  r14-core (6.6KB)                │   │
│  │              │     │  ├── register(vk) → circuit_id   │   │
│  │              │     │  └── verify(id, proof, inputs)   │   │
│  │              │     │         ▲                         │   │
│  │              │──►  │         │ cross-contract call     │   │
│  │              │     │  r14-transfer (3.3KB)             │   │
│  │              │     │  ├── deposit(cm)                  │   │
│  └──────────────┘     │  └── transfer(proof, ...)        │   │
│         │             └─────────────────────────────────┘   │
│         │ generate                                           │
│         ▼                                                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │  r14-circuit │  │  r14-sdk     │  │  r14-types   │      │
│  │  (arkworks)  │  │  (serialize) │  │  (shared)    │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│         │                                                    │
│         ▼                                                    │
│  ┌──────────────┐                                            │
│  │ r14-poseidon │                                            │
│  │   (hash)     │                                            │
│  └──────────────┘                                            │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              r14-indexer                              │   │
│  │  Event watcher → Merkle tree → REST API              │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

## Project Structure

```
r14-dev/
├── crates/
│   ├── r14-core/           # ✅ Soroban contract — general-purpose Groth16 verifier registry
│   ├── r14-transfer/       # ✅ Soroban contract — private transfer app (calls r14-core)
│   ├── r14-sdk/            # ✅ Arkworks → Soroban serialization helpers
│   ├── r14-types/          # ✅ Shared types (Note, Nullifier, Keys, Merkle)
│   ├── r14-poseidon/       # ✅ Poseidon hash (commitment, nullifier, owner)
│   ├── r14-circuit/        # ✅ Off-chain proof generation (7,638 constraints)
│   ├── r14-indexer/        # ✅ Event watcher + Merkle tree + REST API
│   └── r14-cli/            # ✅ CLI: keygen, deposit, transfer, balance, init-contract
│
├── docs and benchmarks/
│   ├── tech.md                         # Technical specification
│   └── TESTNET_DEPLOYMENT_SUMMARY.md   # All testnet deployments
│
├── research/
│   └── extraction.md       # Standard extraction results + migration guide
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
# Run all tests (39 tests)
cargo test --workspace

# Build contract WASMs
stellar contract build --package r14-core
stellar contract build --package r14-transfer
```

### Deploy & Run E2E

```bash
# 1. Deploy r14-core (verifier registry)
stellar contract deploy \
  --wasm target/wasm32v1-none/release/r14_core.wasm \
  --network testnet --source test-r14
# → returns CORE_CONTRACT_ID

# 2. Initialize r14-core with admin
stellar contract invoke --id <CORE_CONTRACT_ID> \
  --network testnet --source test-r14 \
  -- init --admin <YOUR_ADDRESS>

# 3. Register transfer circuit VK on r14-core
#    (returns circuit_id)

# 4. Deploy r14-transfer
stellar contract deploy \
  --wasm target/wasm32v1-none/release/r14_transfer.wasm \
  --network testnet --source test-r14
# → returns TRANSFER_CONTRACT_ID

# 5. Initialize r14-transfer with core address + circuit_id
stellar contract invoke --id <TRANSFER_CONTRACT_ID> \
  --network testnet --source test-r14 \
  -- init --core_contract <CORE_CONTRACT_ID> --circuit_id <CIRCUIT_ID>

# 6. Generate wallet + deposit + transfer (via CLI)
cargo run -p r14-cli -- keygen
cargo run -p r14-cli -- deposit 1000
cargo run -p r14-cli -- transfer 700 <recipient_owner_hash>
```

## Latest: Phase 3.5 — Standard Extraction

**Status: SHIPPED**

**Deployed (testnet):**
- r14-core: [`CA4UEWIHNJRNIAICTTINMNKBVYXMXMIGTVRSCB6YVOSWZ4WLSJY3ZNFS`](https://lab.stellar.org/r/testnet/contract/CA4UEWIHNJRNIAICTTINMNKBVYXMXMIGTVRSCB6YVOSWZ4WLSJY3ZNFS)
- r14-transfer: [`CB57STZJ6DEFQAWRORLZKXO2IZ7ZJYBOVN5VBQ7RS4MEHRN4ZLNURBRU`](https://lab.stellar.org/r/testnet/contract/CB57STZJ6DEFQAWRORLZKXO2IZ7ZJYBOVN5VBQ7RS4MEHRN4ZLNURBRU)

**Results:**
- Monolithic r14-kernel split into r14-core (verifier) + r14-transfer (app) + r14-sdk (serialization)
- 39 tests passing across 8 crates
- Combined WASM: 10.0KB (6.6KB + 3.3KB) — smaller than 10.3KB monolith
- Cross-contract verification working on testnet
- Transfer circuit registered, `is_registered` returns true

## Crates

### [r14-core](crates/r14-core/) ✅
**Soroban contract** — General-purpose Groth16 verifier registry

- `init(admin)`, `register(caller, vk) → circuit_id`, `verify(circuit_id, proof, inputs) → bool`
- Content-addressed circuit_id via sha256 of VK bytes
- Admin-gated registration, unified IC representation
- 6.6KB WASM, 8 tests

### [r14-transfer](crates/r14-transfer/) ✅
**Soroban contract** — Private transfer app (calls r14-core)

- `init(core_addr, circuit_id)`, `deposit(cm)`, `transfer(proof, ...) → bool`
- Cross-contract call to r14-core via `env.invoke_contract()`
- Nullifier tracking, event emission for indexer
- 3.3KB WASM, 4 tests

### [r14-sdk](crates/r14-sdk/) ✅
**Rust library** — Arkworks → Soroban serialization

- `serialize_g1/g2/fr`, `serialize_vk_for_soroban`, `serialize_proof_for_soroban`
- Handles LE→BE byte reversal for Fr scalars

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
- [x] **Phase 3.5:** Standard Extraction (r14-kernel → r14-core + r14-transfer + r14-sdk) — **SHIPPED**
- [ ] **Phase 4:** Hardening
  - [ ] On-chain Merkle root tracking
  - [ ] Storage TTL / `extend_ttl()`
  - [ ] Update CLI for two-contract architecture
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
- r14-core: 6.6KB WASM — verifier registry (init, register, verify, get_vk, is_registered)
- r14-transfer: 3.3KB WASM — transfer app (init, deposit, transfer)
- BLS12-381 host functions (g1_msm, pairing_check)
- Cross-contract verification via `env.invoke_contract()`

**Merkle Tree:**
- Depth: 20 (1M capacity)
- Hash: Poseidon (ZK-friendly)
- Sparse tree, SQLite-backed persistence

**Cryptography:**
- Commitments: Poseidon(value, app_tag, owner, nonce)
- Nullifiers: Poseidon(secret_key, nonce)
- Keys: BLS12-381 Fr scalars

## Test Suite (39 tests)

| Crate | Tests | What |
|-------|-------|------|
| r14-core | 8 | register+verify, wrong input, unregistered circuit, duplicate register, non-admin, is_registered, get_vk, proof generator |
| r14-transfer | 4 | E2E transfer, double spend, invalid proof, wrong nullifier |
| r14-circuit | 7 | valid transfer, wrong sk, wrong path, value mismatch, app tag, constraints, serialization |
| r14-poseidon | 6 | determinism, order, nullifier, commitment, nonce sensitivity |
| r14-types | 2 | key gen, note creation |
| r14-indexer | 11 | tree ops (x5), E2E flow, plus bin duplicates |
| r14-sdk | 1 | (implicit via r14-circuit re-exports) |

## Development

```bash
cargo test --workspace                    # all 39 tests
stellar contract build -p r14-core        # build verifier WASM
stellar contract build -p r14-transfer    # build transfer WASM
cargo fmt --all                           # format
cargo clippy --all-targets                # lint
```

## Testnet Deployments

| Phase | Contract | WASM | Date |
|-------|----------|------|------|
| Extraction: r14-core | `CA4UEWIHNJRNIAICTTINMNKBVYXMXMIGTVRSCB6YVOSWZ4WLSJY3ZNFS` | 6.6KB | 2026-02-20 |
| Extraction: r14-transfer | `CB57STZJ6DEFQAWRORLZKXO2IZ7ZJYBOVN5VBQ7RS4MEHRN4ZLNURBRU` | 3.3KB | 2026-02-20 |
| Phase 3 (superseded) | `CDV6FRX7GFHZIRYB474LNW4325V7HYHD6WXBDHW4C2XEMCYPT4NF3GPN` | 10.3KB | 2026-02-20 |
| Phase 2 (superseded) | `CDAXRSKM4VL4MPP7KNPNRDGEU6BWC4KXVXGT4RZ5TNHSQXHJCV3KVGMZ` | 11.8KB | 2026-02-19 |
| Phase 0 (superseded) | `CC4QPAKN2J6NUCW4QVW5ZA2BOUC4O4KUH6FMFANI34W2N7I7WGEKLZGW` | 6.2KB | 2026-02-17 |

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

**Current Status:** Phase 3.5 shipped — r14-kernel split into r14-core + r14-transfer + r14-sdk, both contracts deployed to testnet with cross-contract verification working
