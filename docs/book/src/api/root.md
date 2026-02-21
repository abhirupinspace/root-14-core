# Crate Root (Re-exports)

These types and functions are available directly from `r14_sdk::*`. They are re-exported from `r14-types` and `r14-poseidon`.

## Types

| Type | Description |
|------|-------------|
| `SecretKey` | Wrapper around `Fr`. Create with `SecretKey::random(&mut rng)`. |
| `Note` | UTXO: `value: u64`, `app_tag: u32`, `owner: Fr`, `nonce: Fr`. Create with `Note::new(value, app_tag, owner, &mut rng)` or `Note::with_nonce(...)`. |
| `Nullifier` | Spend tag. Wraps `Fr`. Prevents double-spending. |
| `MerklePath` | Inclusion proof: `siblings: Vec<Fr>`, `indices: Vec<bool>`. Length = `MERKLE_DEPTH`. |
| `MerkleRoot` | Wrapper around `Fr`. |

## Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `MERKLE_DEPTH` | `20` | Sparse Merkle tree depth. Supports up to 2^20 leaves. |

## Functions

### `commitment(note: &Note) -> Fr`

Compute a note's Poseidon commitment: `Poseidon(value, app_tag, owner, nonce)`.

```rust
let cm = r14_sdk::commitment(&note);
```

### `nullifier(sk: &SecretKey, nonce: &Fr) -> Nullifier`

Derive a nullifier: `Poseidon(sk, nonce)`.

```rust
let nul = r14_sdk::nullifier(&sk, &note.nonce);
```

### `owner_hash(sk: &SecretKey) -> OwnerHash`

Derive the public owner hash: `Poseidon(sk)`.

```rust
let owner = r14_sdk::owner_hash(&sk);
// share owner.0 as the recipient address
```

### `hash2(a: Fr, b: Fr) -> Fr`

Two-input Poseidon hash. Used internally by the Merkle tree.

```rust
let h = r14_sdk::hash2(left, right);
```
