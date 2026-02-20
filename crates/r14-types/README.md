# r14-types

**Shared type definitions for Root14 privacy protocol**

## Status: SHIPPED

**Tests:** 2 passing | `no_std` by default, `std` feature for off-chain

## Types

```rust
// Keys
pub struct SecretKey(pub Fr);      // Random BLS12-381 scalar
pub struct OwnerHash(pub Fr);      // Poseidon(sk) — public identifier

// Notes (UTXO)
pub struct Note {
    pub value: u64,                // Amount
    pub app_tag: u32,              // Asset identifier
    pub owner: Fr,                 // Owner hash
    pub nonce: Fr,                 // Random blinding factor
}

// Nullifiers
pub struct Nullifier(pub Fr);      // Poseidon(sk, nonce) — spend proof

// Merkle tree
pub struct MerklePath {
    pub siblings: Vec<Fr>,         // 20 sibling hashes
    pub indices: Vec<bool>,        // 20 direction bits
}
pub struct MerkleRoot(pub Fr);
pub const MERKLE_DEPTH: usize = 20;  // 1M leaf capacity
```

## Usage

```rust
use r14_types::{Note, SecretKey, MerklePath, MERKLE_DEPTH};

let sk = SecretKey::random(&mut rng);
let note = Note::new(1000, 1, owner_hash, &mut rng);
let note_with_nonce = Note::with_nonce(1000, 1, owner_hash, specific_nonce);
```

## Features

- `default` — `no_std` (for on-chain / WASM)
- `std` — enables standard library (for CLI, indexer, tests)

## Used By

All crates in the workspace: r14-circuit, r14-poseidon, r14-cli, r14-indexer, r14-kernel (dev-deps only).

## License

Apache-2.0
