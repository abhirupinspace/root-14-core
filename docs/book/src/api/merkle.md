# merkle

`r14_sdk::merkle` — Merkle root computation for Root14's sparse Merkle tree.

All hex outputs are **64 chars, no `0x` prefix** (raw hex for Soroban `BytesN<32>`).

## Functions

### `empty_root() -> Fr`

The root of a tree with all-zero leaves. Computed as `hash2(0, 0)` iterated `MERKLE_DEPTH` times.

### `empty_root_hex() -> String`

Same as `empty_root()` but returns 64-char raw hex.

```rust
let root = r14_sdk::merkle::empty_root_hex();
assert_eq!(root.len(), 64);
```

### `compute_root_from_leaves(leaves: &[Fr]) -> String`

Compute the Merkle root from a list of leaf values. Returns 64-char raw hex.

Empty positions use the zero hash for that level. Mirrors the indexer's `SparseMerkleTree::root`.

```rust
let root = r14_sdk::merkle::compute_root_from_leaves(&[cm_a, cm_b]);
```

**Properties:**
- `compute_root_from_leaves(&[])` equals `empty_root_hex()`
- Leaf order matters
- Deterministic

### `compute_new_root(indexer_url: &str, new_commitments: &[Fr]) -> Result<String>` *(async)*

Fetch existing leaves from the indexer (`GET /v1/leaves`), append `new_commitments`, and compute the resulting root. Returns 64-char raw hex.

```rust
let new_root = r14_sdk::merkle::compute_new_root(
    "http://localhost:3000",
    &[cm_0, cm_1],
).await?;
```

This is used before submitting deposits and transfers to provide the expected new root to the on-chain contract.
