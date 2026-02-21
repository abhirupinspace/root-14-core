# Offline Merkle

Compute Merkle roots without an indexer. Useful for tooling, testing, and verification.

## Empty tree root

```rust
use r14_sdk::merkle;

let root = merkle::empty_root_hex();
// 64-char hex string (no 0x prefix), deterministic
```

This is the root of a tree with all-zero leaves — the initial state before any deposits.

## Root from known leaves

```rust
use r14_sdk::{commitment, Note};
use r14_sdk::wallet::crypto_rng;

let mut rng = crypto_rng();
let note_a = Note::new(100, 1, owner, &mut rng);
let note_b = Note::new(200, 1, owner, &mut rng);

let cm_a = commitment(&note_a);
let cm_b = commitment(&note_b);

let root = merkle::compute_root_from_leaves(&[cm_a, cm_b]);
// 64-char hex string (no 0x prefix)
```

## Properties

- Leaf order matters: `root([a, b]) != root([b, a])`
- Empty list returns the empty root: `root([]) == empty_root_hex()`
- Deterministic: same leaves always produce the same root
- Uses Poseidon `hash2` at every level, depth 20
- Unfilled positions use zero-hashes (Poseidon hash of zeros at each level)

## Use cases

- **Testing**: compute expected roots without a running indexer
- **Verification**: independently verify that an indexer's root is correct
- **Analytics**: rebuild tree state from a list of commitments
