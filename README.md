# Root14 (r14)

**Privacy-preserving transactions on Stellar using Groth16 zero-knowledge proofs**

> **Phase 2 — Circuit + Kernel Integration**: Production transfer circuit (7,638 constraints) verified on-chain. E2E: off-chain prove → on-chain verify.

## Overview

Root14 brings private transactions to Stellar through:
- **Zero-knowledge proofs** (Groth16 + BLS12-381)
- **Soroban smart contract** for on-chain verification
- **UTXO model** with encrypted notes
- **Merkle tree** commitment tracking

Users can transfer assets privately without revealing amounts, senders, or receivers.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Root14 System                        │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌──────────────┐                  ┌────────────────┐   │
│  │   Client     │                  │  r14-kernel    │   │
│  │  (r14-cli)   │ ─── proof ────►  │  (Soroban)     │   │
│  └──────────────┘                  └────────────────┘ 
│
│         │                                   │           │
│         │ generate                          │ verify    │
│         ▼                                   ▼           │
│  ┌──────────────┐                  ┌────────────────┐  │
│  │  r14-circuit │                  │  BLS12-381     │  │
│  │  (arkworks)  │                  │  host funcs    │  │
│  └──────────────┘                  └────────────────┘  │
│         │                                               │
│         │ uses                                          │
│         ▼                                               │
│  ┌──────────────┐    ┌──────────────┐                  │
│  │ r14-poseidon │    │  r14-types   │                  │
│  │   (hash)     │    │   (shared)   │                  │
│  └──────────────┘    └──────────────┘                  │
│                                                         │
│  ┌──────────────────────────────────────────────────┐  │
│  │              r14-indexer                         │  │
│  │  (Scan blockchain, decrypt notes)                │  │
│  └──────────────────────────────────────────────────┘  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

## Project Structure

```
r14-dev/
├── crates/
│   ├── r14-kernel/         # ✅ Soroban contract (verifier + transfer)
│   ├── r14-types/          # ✅ Shared types (Note, Nullifier, Keys, Merkle)
│   ├── r14-poseidon/       # ✅ Poseidon hash (commitment, nullifier, owner)
│   ├── r14-circuit/        # ✅ Off-chain proof generation (7,638 constraints)
│   ├── r14-indexer/        # 📦 Blockchain scanner
│   └── r14-cli/            # 📦 User CLI tool
│
├── scripts/
│   └── deploy_phase0.sh    # Testnet deployment helper
│
├── tech.md                 # Technical specification
├── PHASE0_STATUS.md        # Current implementation status
└── README.md               # This file
```

**Legend:**
- ✅ Shipped
- 📦 Placeholder

## Quick Start

### Prerequisites
```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown

# Stellar CLI
cargo install --locked stellar-cli

# Configure network
stellar network add \
  --global testnet \
  --rpc-url https://soroban-testnet.stellar.org:443 \
  --network-passphrase "Test SDF Network ; September 2015"
```

### Build

```bash
# Build all crates
cargo build --release

# Build contract WASM
cargo build --target wasm32-unknown-unknown --release --package r14-kernel
```

### Test

```bash
# Generate test proof (off-chain)
cargo test --test proof_generator -- --nocapture

# Run contract tests
cargo test --package r14-kernel

# Run all tests
cargo test --workspace
```

### Deploy (Phase 0)

```bash
# Deploy contract
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/r14_kernel.wasm \
  --network testnet \
  --source <YOUR_ACCOUNT>

# Test verification
stellar contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- verify_dummy_proof
```

## Latest: Phase 2 — Circuit + Kernel Integration

**Status: SHIPPED**

**Deployed:**
- Contract: `CDAXRSKM4VL4MPP7KNPNRDGEU6BWC4KXVXGT4RZ5TNHSQXHJCV3KVGMZ`
- [Explorer](https://lab.stellar.org/r/testnet/contract/CDAXRSKM4VL4MPP7KNPNRDGEU6BWC4KXVXGT4RZ5TNHSQXHJCV3KVGMZ) (testnet)

**Results:**
- Transfer circuit: 7,638 constraints, 4 public inputs
- WASM: 11.8KB optimized
- 16 tests passing (9 kernel + 7 circuit)
- E2E: off-chain Groth16 prove → on-chain verify
- Double-spend prevention via nullifier storage

## Crates

### [r14-kernel](crates/r14-kernel/) ✅
**Soroban smart contract** — Groth16 verifier + transfer entrypoint

- `init(vk)` → store VK, `transfer(proof, ...)` → verify + nullifier check
- BLS12-381 host functions: `g1_msm`, `pairing_check`
- 11.8KB WASM, 9 tests

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

### [r14-indexer](crates/r14-indexer/) 📦
**Blockchain scanner** - Decrypts user notes

- Scans contract events
- Tries to decrypt with user viewing key
- Builds local UTXO set

### [r14-cli](crates/r14-cli/) 📦
**User CLI** - Send/receive private transactions

- Key management
- Proof generation
- Transaction submission

## How It Works

### Private Transfer Flow

```
1. Alice wants to send 100 tokens to Bob
   ├─► Selects her UTXO (note) from local state
   ├─► Creates new note for Bob (encrypted)
   └─► Generates ZK proof off-chain

2. Proof proves (without revealing):
   ├─► "I own a note in the commitment tree"
   ├─► "I computed its nullifier correctly"
   ├─► "New notes sum to same value"
   └─► "Tree root updates correctly"

3. Submit to r14-kernel contract:
   ├─► Public: [old_root, new_root, nullifier]
   ├─► Proof: 384 bytes
   └─► Contract verifies → accepts/rejects

4. If accepted:
   ├─► Nullifier stored (prevent double-spend)
   ├─► New commitments added to tree
   └─► Bob scans blockchain, decrypts his note
```

### Security Model

- **Anonymity:** Sender/receiver hidden
- **Confidentiality:** Amounts encrypted
- **Unlinkability:** Can't trace transaction graph
- **Double-spend prevention:** Nullifier uniqueness enforced
- **Soundness:** Invalid proofs rejected (Groth16 security)

## Roadmap

- [x] **Phase 0:** Feasibility spike (Groth16 + BLS12-381) — **SHIPPED**
- [x] **Phase 1:** Shared primitives (r14-types + r14-poseidon) — **SHIPPED**
- [x] **Phase 2:** Circuit + kernel integration — **SHIPPED**
- [ ] **Phase 3:** CLI + Indexer
  - [ ] `r14-cli`: keygen, deposit, transfer, balance
  - [ ] `r14-indexer`: event watcher, Merkle tree, REST API
- [ ] **Phase 4:** Hardening
  - [ ] Gas profiling, edge cases, view key compliance
  - [ ] Admin auth, storage TTL, contractevent migration
- [ ] **Phase 5:** Launch
  - [ ] Audits, trusted setup ceremony
  - [ ] Mainnet deployment

## Technical Specs

**ZK Proof System:**
- Groth16 (trusted setup, 384 byte proofs)
- BLS12-381 elliptic curve
- 7,638 R1CS constraints (transfer circuit)

**On-Chain:**
- Soroban smart contract
- BLS12-381 host functions
- Target: <80M instructions per verification

**Merkle Tree:**
- Depth: 20 (1M capacity)
- Hash: Poseidon (ZK-friendly)
- Sparse tree representation

**Cryptography:**
- Commitments: Poseidon hash
- Nullifiers: Poseidon(commitment, sk)
- Keys: EdDSA-like (BLS12-381 Fr scalars)

## Development

### Run Tests
```bash
cargo test --workspace
```

### Build WASM
```bash
cargo build --target wasm32-unknown-unknown --release
```

### Format
```bash
cargo fmt --all
```

### Lint
```bash
cargo clippy --all-targets --all-features
```

## Resources

**Stellar/Soroban:**
- [Soroban Docs](https://soroban.stellar.org/)
- [BLS12-381 Host Functions](https://docs.rs/soroban-sdk/25.1.1/soroban_sdk/crypto/bls12_381/)

**Zero-Knowledge Proofs:**
- [Groth16 Paper](https://eprint.iacr.org/2016/260.pdf)
- [arkworks](https://arkworks.rs/)
- [Zcash Protocol Spec](https://zips.z.cash/protocol/protocol.pdf)

**Privacy Coins:**
- [Tornado Cash](https://tornado.cash/)
- [Aztec Network](https://aztec.network/)
- [Zcash](https://z.cash/)

## Contributing

1. Check Phase 0 results first
2. Read [tech.md](tech.md) for full spec
3. Pick a crate README for details
4. Submit PR with tests

## License

Apache-2.0

## Security

⚠️ **Pre-alpha software** - Do not use with real funds

Phase 0 is a feasibility study. Production deployment requires:
- [ ] Circuit audit
- [ ] Contract audit
- [ ] Trusted setup ceremony
- [ ] Testnet stress testing
- [ ] Economic security analysis

## Contact

Project maintained by [@abhirupbanerjee](https://github.com/abhirupbanerjee)

---

**Current Status:** Phase 2 shipped — production circuit verified on testnet
