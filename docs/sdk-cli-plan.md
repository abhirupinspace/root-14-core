# Root-14 SDK & CLI Roadmap

> Making Root-14 a developer platform, not just internal tooling.

---

## 1. Current State

### r14-sdk (77 lines)

Serialization-only. Converts arkworks BLS12-381 types to hex strings for Soroban consumption.

| Export | Purpose |
|--------|---------|
| `serialize_g1(G1Affine) -> String` | Uncompressed G1 hex (96 bytes / 192 hex chars) |
| `serialize_g2(G2Affine) -> String` | Uncompressed G2 hex (192 bytes / 384 hex chars) |
| `serialize_fr(Fr) -> String` | BE Fr hex (32 bytes / 64 hex chars, LE→BE flip for Soroban) |
| `serialize_vk_for_soroban(VerifyingKey) -> SerializedVK` | Full VK serialization |
| `serialize_proof_for_soroban(Proof, &[Fr]) -> (SerializedProof, Vec<String>)` | Proof + public inputs |

No client abstraction, no connection management, no proof generation helpers.

### r14-cli (6 commands)

| Command | What it does |
|---------|-------------|
| `r14 keygen` | Generate keypair, create wallet.json with placeholder config |
| `r14 deposit <value>` | Create note locally, optionally submit on-chain. Flags: `--app-tag`, `--local-only` |
| `r14 transfer <value> <recipient>` | Find unspent note, generate ZK proof, submit transfer. Flag: `--dry-run` |
| `r14 init-contract` | Register VK on r14-core, init r14-transfer with circuit_id + empty root |
| `r14 balance` | Sync notes with indexer, show unspent notes + total |
| `r14 compute-root <commitments...>` | Offline merkle root from commitment hex values |

### r14-circuit

Single circuit: `TransferCircuit` (7,638 R1CS constraints, 4 public inputs).

**Circuit constraints enforced:**
1. Ownership — `poseidon(sk) == consumed_note.owner`
2. Commitment — `poseidon(value, app_tag, owner, nonce) == cm`
3. Merkle inclusion — path verification against `old_root`
4. Nullifier — `poseidon(sk, nonce) == nullifier`
5. Output commitments — both output notes committed correctly
6. Value conservation — `consumed.value == created[0].value + created[1].value`
7. App tag match — all notes share same `app_tag`

### On-chain contracts

**r14-core**: General-purpose Groth16 verifier registry.
- `init(admin)` — set admin
- `register(caller, vk) -> circuit_id` — content-addressed VK storage (`sha256(alpha_g1 ++ beta_g2 ++ ...)`)
- `verify(circuit_id, proof, public_inputs) -> bool` — verify against stored VK
- `get_vk(circuit_id)`, `is_registered(circuit_id)` — lookups

**r14-transfer**: Private transfer contract, delegates verification to r14-core.
- `init(core_contract, circuit_id, empty_root)` — set up with core reference
- `deposit(cm, new_root)` — store commitment, emit event
- `transfer(proof, old_root, nullifier, cm_0, cm_1, new_root)` — verify proof, mark nullifier spent, emit event
- Circular root buffer (100 entries) for root history

### Pain points

- **Manual hex everywhere** — users deal with raw hex strings for commitments, nullifiers, roots
- **No error recovery** — `init-contract` failure at step 1 leaves step 2 hanging; transfer failures give raw panics
- **Placeholder config** — keygen writes `"PLACEHOLDER"` for stellar_secret and contract IDs; no validation until command time
- **No progress feedback** — proof generation takes seconds with zero indication
- **No project templates** — integrators must manually wire everything
- **Hardcoded seed** — setup/prove use deterministic `seed=42` (fine for dev, not obvious to users)
- **Silent indexer failures** — balance sync swallows indexer errors

---

## 2. SDK Roadmap

### Tier 1: R14Client — high-level integration

Goal: **4-line integration** for any Soroban app.

```rust
use r14_sdk::R14Client;

let client = R14Client::new(
    "https://soroban-testnet.stellar.org",
    "https://localhost:8080",   // indexer
    R14Contracts {
        core: "CABC...".parse()?,
        transfer: "CDEF...".parse()?,
    },
)?;

// Deposit
let tx = client.deposit(1000, app_tag).await?;

// Transfer
let (tx, change_note) = client.transfer(
    &my_note,
    &recipient_owner_hash,
    700,
).await?;

// Balance
let notes = client.balance(&secret_key).await?;
println!("total: {}", notes.total());
```

**R14Client internals:**
- Wraps indexer REST client + Soroban RPC client
- Handles: proof generation, serialization (calls `r14-circuit` + `r14-sdk`), root computation, tx submission
- `client.sync()` — fetches latest state from indexer, updates local note set
- Caches proving key after first load (setup is expensive)
- Returns typed results: `DepositResult { tx_hash, note, commitment }`, `TransferResult { tx_hash, change_note, nullifier }`

**Error types:**
```rust
pub enum R14Error {
    InsufficientBalance { available: u64, requested: u64 },
    IndexerUnreachable(String),
    NoteAlreadySpent(String),
    ProofGenerationFailed(String),
    SorobanSubmitFailed(String),
    InvalidConfig(String),
}
```

**Shipping criteria:** User can `cargo add r14-sdk`, create client, deposit+transfer without touching hex, proofs, or serialization.

### Tier 2: Pre-built Circuit Library

Reusable circuits beyond `TransferCircuit`, each with setup/prove/verify + Soroban serialization.

| Circuit | Proves | Public inputs |
|---------|--------|---------------|
| `MembershipCircuit` | Element in committed set without revealing it | `root` |
| `RangeCircuit` | Value in `[a, b]` without revealing value | `a`, `b`, `commitment` |
| `PreimageCircuit` | Knowledge of hash preimage | `hash` |
| `OwnershipCircuit` | Note ownership without revealing secret key | `owner_hash`, `commitment` |

Each circuit follows the same pattern:

```rust
pub struct MembershipCircuit {
    // Private witnesses
    pub element: Option<Fr>,
    pub merkle_path: Option<MerklePath>,
}

pub struct MembershipPublicInputs {
    pub root: Fr,
}

impl ConstraintSynthesizer<Fr> for MembershipCircuit { ... }

// Convenience API (wraps r14-circuit patterns)
pub fn membership_setup(rng: &mut R) -> (ProvingKey, VerifyingKey);
pub fn membership_prove(pk: &ProvingKey, element: Fr, path: MerklePath, rng: &mut R) -> (Proof, MembershipPublicInputs);
pub fn membership_verify(vk: &VerifyingKey, proof: &Proof, pi: &MembershipPublicInputs) -> bool;
```

All circuits reuse existing `poseidon_gadget` and `merkle_gadget` from `r14-circuit`.

### Tier 3: R14AppConstraints — pluggable custom circuits

Users implement one trait, get the full prove/verify/serialize pipeline for free.

```rust
pub trait R14AppConstraints: ConstraintSynthesizer<Fr> + Clone {
    /// Public inputs for this circuit
    fn public_inputs(&self) -> Vec<Fr>;

    /// Unique identifier for this circuit type
    fn circuit_id() -> &'static str;

    /// Empty instance for setup (None witnesses)
    fn empty() -> Self;
}
```

SDK provides generic helpers:

```rust
// One-time setup
let (pk, vk) = r14_sdk::setup::<MyCircuit>(rng)?;

// Register on-chain
let circuit_id = r14_sdk::register_circuit(&client, &vk).await?;

// Prove and submit in one call
let tx = r14_sdk::prove_and_submit(&client, &pk, my_circuit_instance).await?;

// Or just prove locally
let (proof, pi) = r14_sdk::prove(&pk, my_circuit_instance, rng)?;
let (serialized_proof, serialized_pi) = r14_sdk::serialize(&proof, &pi);
```

**Shipping criteria:** A developer can implement `R14AppConstraints` for a custom circuit, call `prove_and_submit()`, and get an on-chain verified proof without writing any serialization or Soroban interaction code.

---

## 3. CLI Roadmap

### Phase A: DX Polish

**New commands:**
- `r14 status` — wallet loaded? contracts configured? indexer reachable? notes synced?
- `r14 config set <key> <value>` — set `rpc_url`, `indexer_url`, `core_contract_id`, `transfer_contract_id`, `stellar_secret`
- `r14 config show` — print current config (mask secret key)

**UX improvements:**
- Colored output (green success, red errors, yellow warnings)
- Progress spinners during proof generation and on-chain submission
- Structured error messages with next-step hints (e.g., "run `r14 config set stellar_secret <key>` first")
- `--json` flag on all commands for scripting / CI integration
- Validate config on every command, fail fast with clear message

### Phase B: Project Scaffolding

**New commands:**
- `r14 new <project-name>` — scaffold a new R14 app:
  ```
  my-app/
  ├── Cargo.toml          # r14-sdk + r14-circuit deps
  ├── src/
  │   ├── main.rs          # R14Client setup boilerplate
  │   └── circuit.rs       # R14AppConstraints template
  ├── tests/
  │   └── circuit_test.rs  # Constraint satisfaction test
  └── scripts/
      └── deploy.sh        # stellar CLI deploy commands
  ```
- `r14 circuit test` — run constraint satisfaction check locally (catches bugs before proving)
- `r14 circuit bench` — measure constraint count + prove time + verify time

### Phase C: Deployment Flow

**New commands:**
- `r14 deploy` — single command: build WASM → deploy contracts → init → register VK
- `r14 upgrade` — redeploy contracts preserving state (bump WASM, re-register VK if changed)
- `r14 verify <tx-hash>` — fetch proof from tx, verify locally or on-chain

**Full lifecycle:**
```
r14 keygen                     # 1. create wallet
r14 config set rpc_url ...     # 2. configure
r14 deploy                     # 3. deploy everything
r14 deposit 1000               # 4. deposit
r14 balance                    # 5. check
r14 transfer 700 <recipient>   # 6. transfer
r14 verify <tx-hash>           # 7. verify
```

### CLI command matrix (existing + planned)

| Command | Status | Phase |
|---------|--------|-------|
| `r14 keygen` | Exists | - |
| `r14 deposit` | Exists | - |
| `r14 transfer` | Exists | - |
| `r14 init-contract` | Exists (superseded by `deploy`) | - |
| `r14 balance` | Exists | - |
| `r14 compute-root` | Exists | - |
| `r14 status` | Planned | A |
| `r14 config set/show` | Planned | A |
| `r14 new` | Planned | B |
| `r14 circuit test/bench` | Planned | B |
| `r14 deploy` | Planned | C |
| `r14 upgrade` | Planned | C |
| `r14 verify` | Planned | C |

---

## 4. Benchmarks

### Current measurements

| Metric | Value | Source |
|--------|-------|--------|
| Transfer circuit constraints | 7,638 R1CS | `r14_circuit::constraint_count()` |
| Public inputs | 4 (root, nullifier, cm_0, cm_1) | `TransferCircuit` |
| Proof size | 384 bytes (G1 + G2 + G1) | Groth16 BLS12-381 |
| VK IC length | 5 (1 constant + 4 public inputs) | Serialization test |
| r14-core WASM | 6.6 KB | Build output |
| r14-transfer WASM | 3.3 KB | Build output |
| Combined on-chain | ~10 KB | Sum |
| Deposit fee | ~42K stroops (~0.004 XLM) | Testnet measurement |
| Merkle depth | 20 (1M leaves) | `MERKLE_DEPTH` constant |
| Poseidon params | rate=2, full=8, partial=31, alpha=17 | r14-poseidon config |
| Root history buffer | 100 entries | `ROOT_HISTORY_SIZE` |
| Storage TTL | 535,680 ledgers (~30 days) | Contract constants |
| Test count | 39 | Test suite |

### G1/G2/Fr serialization sizes

| Type | Bytes | Hex chars |
|------|-------|-----------|
| G1Affine (uncompressed) | 96 | 192 |
| G2Affine (uncompressed) | 192 | 384 |
| Fr (compressed, BE) | 32 | 64 |

### Targets to measure

| Metric | Target | Notes |
|--------|--------|-------|
| Prove time (M1 Mac) | < 3s | Currently unmeasured; `r14 circuit bench` will track |
| Verify time (on-chain) | < 50K instructions | Soroban CPU budget limit is 100M |
| Setup time (keygen) | < 5s | One-time cost per circuit |
| Indexer sync latency | < 2s per event | Event polling interval |
| E2E deposit→transfer | < 30s | Including proof gen + 2 tx submissions |
| Circuit library overhead | < 2x transfer constraints per circuit | Keep circuits lean |

---

## 5. Developer Journey

### Quickstart (< 5 minutes)

```bash
# Install
cargo install r14-cli

# Generate wallet
r14 keygen

# Configure testnet
r14 config set rpc_url https://soroban-testnet.stellar.org
r14 config set indexer_url http://localhost:8080

# Deploy (handles init-contract internally)
r14 deploy

# Use it
r14 deposit 1000
r14 balance
r14 transfer 700 <recipient_owner_hash>
r14 balance
```

### SDK integration guide

```rust
// Cargo.toml
// [dependencies]
// r14-sdk = "0.1"

use r14_sdk::{R14Client, R14Contracts};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = R14Client::new(
        "https://soroban-testnet.stellar.org",
        "http://localhost:8080",
        R14Contracts {
            core: "CABC...".parse()?,
            transfer: "CDEF...".parse()?,
        },
    )?;

    // Load wallet
    let wallet = r14_sdk::Wallet::load("wallet.json")?;

    // Deposit
    let result = client.deposit(1000, 1).await?;
    println!("deposited: tx={}", result.tx_hash);

    // Check balance
    let notes = client.balance(&wallet.secret_key).await?;
    println!("balance: {}", notes.total());

    // Transfer
    let (result, change) = client.transfer(
        &notes.unspent[0],
        &recipient_owner_hash,
        700,
    ).await?;
    println!("transferred: tx={}", result.tx_hash);

    Ok(())
}
```

### Custom circuit guide

```rust
use r14_sdk::R14AppConstraints;
use ark_bls12_381::Fr;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};

#[derive(Clone)]
struct MyCircuit {
    secret: Option<Fr>,
    public_hash: Option<Fr>,
}

impl R14AppConstraints for MyCircuit {
    fn public_inputs(&self) -> Vec<Fr> {
        vec![self.public_hash.unwrap_or_default()]
    }

    fn circuit_id() -> &'static str { "my-preimage-proof" }

    fn empty() -> Self {
        Self { secret: None, public_hash: None }
    }
}

impl ConstraintSynthesizer<Fr> for MyCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        // ... your constraints here
        Ok(())
    }
}

// Usage:
// let (pk, vk) = r14_sdk::setup::<MyCircuit>(&mut rng)?;
// let circuit_id = r14_sdk::register_circuit(&client, &vk).await?;
// let tx = r14_sdk::prove_and_submit(&client, &pk, my_instance).await?;
```

---

## 6. Phasing & Dependencies

```
Tier 1: R14Client          ──┐
Phase A: DX Polish          ──┼── can ship independently
Tier 2: Circuit Library     ──┘

Tier 3: R14AppConstraints   ── depends on Tier 2 patterns
Phase B: Scaffolding         ── depends on Tier 3 (templates use trait)
Phase C: Deploy Flow         ── depends on Phase A (config system)
```

**Priority order:** Phase A → Tier 1 → Tier 2 → Phase C → Tier 3 → Phase B

Rationale: Config/UX polish unblocks everything. R14Client is highest developer impact. Circuit library validates patterns before exposing the trait. Deploy flow needs config. Scaffolding comes last because it templates everything else.

---

## 7. Resolved Questions

| Question | Decision | Rationale |
|----------|----------|-----------|
| Gas profiling numbers? | Defer to Phase C | Need `r14 deploy` to measure reliably |
| SDK crate name | Keep `r14-sdk` | Rename breaks imports for zero benefit now |
| R14Client sync support? | Async-only | All ops hit network; users can `block_on` if needed |
| `r14 deploy` auto-fund from faucet? | No, print hint | Faucet API too fragile; just show friendbot URL |
| Circuit library crate location? | Separate `r14-circuits` | Keeps `r14-circuit` focused on core TransferCircuit |
