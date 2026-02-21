# r14-sdk

Client library for **Root14** — the ZK privacy standard for Stellar.

`r14-sdk` gives your Rust application everything it needs to create private notes, manage wallets, compute Merkle roots, serialize Groth16 proofs for Soroban, and submit transactions on-chain. Pair it with `r14-circuit` for ZK proof generation to get the full private transfer pipeline.

## Setup

```toml
[dependencies]
r14-sdk = { path = "crates/r14-sdk" }

# Include proof generation (pulls in r14-circuit automatically):
# r14-sdk = { path = "crates/r14-sdk", features = ["prove"] }
```

`r14-sdk` re-exports `r14-types` and `r14-poseidon` — you don't need to depend on them directly.

## Architecture

```
┌─────────────────────────────────────────────────┐
│                   Your Dapp                     │
├─────────────────────────────────────────────────┤
│  r14-sdk                                        │
│  ┌──────────┐ ┌────────┐ ┌────────┐ ┌────────┐ │
│  │  wallet   │ │ merkle │ │soroban │ │  ser.  │ │
│  └──────────┘ └────────┘ └────────┘ └────────┘ │
│  re-exports: SecretKey, Note, commitment, ...   │
├─────────────────────────────────────────────────┤
│  r14-sdk feature "prove" (optional)              │
│  prove::setup() · prove::prove() · ...          │
├─────────────────────────────────────────────────┤
│  Stellar / Soroban                              │
│  r14-core contract · r14-transfer contract      │
└─────────────────────────────────────────────────┘
```

## Modules

| Module | What it does |
|--------|-------------|
| *crate root* | Re-exports core types and Poseidon functions |
| `wallet` | Wallet JSON persistence, hex ↔ Fr conversion |
| `merkle` | Offline and indexer-backed Merkle root computation |
| `soroban` | Stellar CLI wrapper for on-chain contract calls |
| `serialize` | Groth16 proof/VK → hex for Soroban contracts |
| `prove` | ZK proof generation (feature-gated) |

## Core concepts

### Notes (UTXOs)

Root14 uses a UTXO model. Each private balance is a **Note**:

```rust
pub struct Note {
    pub value: u64,     // amount
    pub app_tag: u32,   // application identifier
    pub owner: Fr,      // owner_hash (derived from secret key)
    pub nonce: Fr,      // random, makes each note unique
}
```

A note's **commitment** is `Poseidon(value, app_tag, owner, nonce)` — this is what goes on-chain. The note data stays private.

### Keys

```rust
// secret key — random field element, never leaves your machine
let sk = SecretKey::random(&mut rng);

// owner hash — Poseidon(sk), safe to share publicly
let owner = owner_hash(&sk);
```

Give your `owner_hash` to senders. They use it to create notes you can spend.

### Nullifiers

When spending a note, you reveal its **nullifier** — `Poseidon(sk, nonce)`. The contract records this to prevent double-spends. The nullifier reveals nothing about the note itself.

### Merkle tree

All commitments live in a sparse Merkle tree (depth 20, Poseidon hash). To prove a note exists, you provide a Merkle inclusion proof. The on-chain contract only stores the root.

## Integration guide

### 1. Keygen

```rust
use r14_sdk::{SecretKey, owner_hash};
use r14_sdk::wallet::{self, fr_to_hex, WalletData};

let mut rng = wallet::crypto_rng();
let sk = SecretKey::random(&mut rng);
let owner = owner_hash(&sk);

let w = WalletData {
    secret_key: fr_to_hex(&sk.0),
    owner_hash: fr_to_hex(&owner.0),
    stellar_secret: "S_YOUR_STELLAR_SECRET".into(),
    notes: vec![],
    indexer_url: "http://localhost:3000".into(),
    rpc_url: "https://soroban-testnet.stellar.org:443".into(),
    core_contract_id: "C_CORE_ID".into(),
    transfer_contract_id: "C_TRANSFER_ID".into(),
};
wallet::save_wallet(&w)?;
```

### 2. Create a deposit note

```rust
use r14_sdk::{Note, commitment};
use r14_sdk::wallet::{self, fr_to_hex, hex_to_fr, NoteEntry};

let mut w = wallet::load_wallet()?;
let owner = hex_to_fr(&w.owner_hash)?;

let mut rng = wallet::crypto_rng();
let note = Note::new(1_000, 1, owner, &mut rng);
let cm = commitment(&note);

// persist locally
w.notes.push(NoteEntry {
    value: note.value,
    app_tag: note.app_tag,
    owner: fr_to_hex(&note.owner),
    nonce: fr_to_hex(&note.nonce),
    commitment: fr_to_hex(&cm),
    index: None,    // set after on-chain confirmation
    spent: false,
});
wallet::save_wallet(&w)?;
```

### 3. Submit deposit on-chain

```rust
use r14_sdk::merkle;
use r14_sdk::soroban;

// compute new merkle root (fetches existing leaves from indexer)
let new_root = merkle::compute_new_root(&w.indexer_url, &[cm]).await?;

// strip 0x prefix for Soroban BytesN<32>
let cm_hex = fr_to_hex(&cm).strip_prefix("0x").unwrap().to_string();

soroban::invoke_contract(
    &w.transfer_contract_id,
    "testnet",
    &w.stellar_secret,
    "deposit",
    &[("cm", &cm_hex), ("new_root", &new_root)],
).await?;
```

### 4. Private transfer (with proof)

This step requires `r14-circuit` in addition to `r14-sdk`.

```rust
use r14_sdk::{Note, MerklePath, commitment};
use r14_sdk::wallet::{crypto_rng, fr_to_hex, hex_to_fr, load_wallet, save_wallet};
use ark_std::rand::{rngs::StdRng, SeedableRng};

let mut w = load_wallet()?;
let sk_fr = hex_to_fr(&w.secret_key)?;

// find an unspent note
let entry = w.notes.iter().find(|n| !n.spent && n.value >= amount).unwrap();
let consumed = Note::with_nonce(
    entry.value, entry.app_tag,
    hex_to_fr(&entry.owner)?, hex_to_fr(&entry.nonce)?,
);

// fetch merkle proof from indexer (GET /v1/proof/{leaf_index})
let merkle_path: MerklePath = /* fetch from indexer */;

// build output notes
let mut rng = crypto_rng();
let note_out = Note::new(amount, 1, recipient_owner, &mut rng);
let note_change = Note::new(entry.value - amount, 1, hex_to_fr(&w.owner_hash)?, &mut rng);

// generate ZK proof (r14-circuit)
let setup_rng = &mut StdRng::seed_from_u64(42); // deterministic setup
let (pk, _vk) = r14_circuit::setup(setup_rng);
let (proof, pi) = r14_circuit::prove(
    &pk, sk_fr, consumed, merkle_path,
    [note_out.clone(), note_change.clone()], &mut rng,
);

// serialize for Soroban
let (sp, spi) = r14_circuit::serialize_proof_for_soroban(&proof, &pi);

// submit on-chain
let cm_0 = commitment(&note_out);
let cm_1 = commitment(&note_change);
let new_root = r14_sdk::merkle::compute_new_root(&w.indexer_url, &[cm_0, cm_1]).await?;

r14_sdk::soroban::invoke_contract(
    &w.transfer_contract_id, "testnet", &w.stellar_secret,
    "transfer",
    &[
        ("proof", &format!(r#"{{"a":"{}","b":"{}","c":"{}"}}"#, sp.a, sp.b, sp.c)),
        ("old_root", &spi[0]),
        ("nullifier", &spi[1]),
        ("cm_0", &spi[2]),
        ("cm_1", &spi[3]),
        ("new_root", &new_root),
    ],
).await?;
```

### 5. Check balance

```rust
let w = wallet::load_wallet()?;
let unspent: Vec<_> = w.notes.iter().filter(|n| !n.spent).collect();
let balance: u64 = unspent.iter().map(|n| n.value).sum();
```

### 6. Offline Merkle computation

For tooling that doesn't need the indexer:

```rust
use r14_sdk::merkle;

// empty tree root
let root = merkle::empty_root_hex();

// root from known leaves
let leaves = vec![cm_a, cm_b, cm_c];
let root = merkle::compute_root_from_leaves(&leaves);
// returns 64-char hex string (no 0x prefix)
```

## API reference

### Re-exported types (crate root)

| Item | Source | Description |
|------|--------|-------------|
| `SecretKey` | r14-types | Wrapper around `Fr`, use `.random(&mut rng)` |
| `Note` | r14-types | UTXO: value + app_tag + owner + nonce |
| `Nullifier` | r14-types | Spend tag, prevents double-spend |
| `MerklePath` | r14-types | Siblings + index bits for inclusion proof |
| `MerkleRoot` | r14-types | Wrapper around `Fr` |
| `MERKLE_DEPTH` | r14-types | Tree depth, currently `20` |
| `commitment()` | r14-poseidon | `Poseidon(value, app_tag, owner, nonce)` |
| `nullifier()` | r14-poseidon | `Poseidon(sk, nonce)` |
| `owner_hash()` | r14-poseidon | `Poseidon(sk)` |
| `hash2()` | r14-poseidon | Two-input Poseidon hash |

### `wallet` module

| Function / Type | Description |
|----------------|-------------|
| `WalletData` | Full wallet state: keys, notes, config URLs |
| `NoteEntry` | Serializable note record (hex strings, not `Fr`) |
| `wallet_path()` | Returns `~/.r14/wallet.json` |
| `load_wallet()` | Deserialize wallet from disk |
| `save_wallet(&w)` | Serialize wallet to disk |
| `fr_to_hex(&fr)` | `Fr` → `0x`-prefixed 64-char BE hex |
| `hex_to_fr("0x...")` | Hex → `Fr`, accepts with/without `0x`, pads short input |
| `crypto_rng()` | Time-seeded `StdRng` |

### `merkle` module

| Function | Description |
|----------|-------------|
| `empty_root()` | Empty tree root as `Fr` |
| `empty_root_hex()` | Empty tree root as 64-char hex (no `0x`) |
| `compute_root_from_leaves(&[Fr])` | Root from leaf list, 64-char hex |
| `compute_new_root(url, &[Fr])` | Fetch leaves from indexer, append, return new root hex |

### `soroban` module

Requires the [Stellar CLI](https://github.com/stellar/stellar-cli) on `$PATH`.

| Function | Description |
|----------|-------------|
| `get_public_key(secret)` | Stellar secret → public key (`G...`) |
| `invoke_contract(id, network, secret, fn, args)` | Call a Soroban contract function |

### `serialize` module

| Function / Type | Description |
|----------------|-------------|
| `SerializedVK` | VK as hex strings (alpha_g1, beta_g2, gamma_g2, delta_g2, ic) |
| `SerializedProof` | Proof as hex strings (a, b, c) |
| `serialize_g1(&G1Affine)` | G1 → 192-char uncompressed hex |
| `serialize_g2(&G2Affine)` | G2 → 384-char uncompressed hex |
| `serialize_fr(&Fr)` | Fr → 64-char BE hex |
| `serialize_vk_for_soroban(&vk)` | Full VK serialization |
| `serialize_proof_for_soroban(&proof, &[Fr])` | Proof + public inputs serialization |

## Hex conventions

| Context | Format | Example |
|---------|--------|---------|
| `wallet` module (`fr_to_hex`) | `0x`-prefixed, 66 chars | `0x00ab...ef` |
| `merkle` module (root hex) | No prefix, 64 chars | `00ab...ef` |
| `serialize` module | No prefix, variable length | G1: 192 chars, G2: 384 chars, Fr: 64 chars |
| Soroban contract args | No prefix | strip `0x` before passing |

## Prerequisites

- **Rust** — nightly or stable with edition 2021
- **Stellar CLI** — required by `soroban` module ([install](https://github.com/stellar/stellar-cli))
- **r14 indexer** — running instance for `merkle::compute_new_root` and balance sync
- **Soroban testnet** — deployed `r14-core` and `r14-transfer` contracts
