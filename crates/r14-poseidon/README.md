# r14-poseidon

**Poseidon hash for BLS12-381 — commitments, nullifiers, Merkle trees**

## Status: SHIPPED

**Tests:** 6 passing

## Functions

| Function | Signature | Purpose |
|----------|-----------|---------|
| `poseidon_hash` | `(&[Fr]) → Fr` | Variable-length Poseidon hash |
| `hash2` | `(Fr, Fr) → Fr` | 2-input hash (Merkle nodes) |
| `commitment` | `(&Note) → Fr` | `Poseidon(value, app_tag, owner, nonce)` |
| `nullifier` | `(sk, nonce) → Fr` | `Poseidon(secret_key, nonce)` — spend proof |
| `owner_hash` | `(&SecretKey) → OwnerHash` | `Poseidon(sk)` — public identifier |

## Parameters (BLS12-381 Fr)

| Param | Value |
|-------|-------|
| Rate | 2 |
| Full rounds | 8 |
| Partial rounds | 31 |
| Alpha (S-box) | 17 |
| Security | 128-bit |

Uses `ark-crypto-primitives` `PoseidonSponge` with `find_poseidon_ark_and_mds` for round constant generation.

## Usage

```rust
use r14_poseidon::{commitment, nullifier, owner_hash, hash2};
use r14_types::{Note, SecretKey};

// Key derivation
let sk = SecretKey::random(&mut rng);
let owner = owner_hash(&sk);

// Note commitment
let note = Note::new(1000, 1, owner.0, &mut rng);
let cm = commitment(&note);

// Nullifier (for spending)
let nf = nullifier(&sk, &note);

// Merkle tree node
let parent = hash2(left_child, right_child);
```

## Tests

```bash
cargo test -p r14-poseidon  # 6 tests
```

| Test | What |
|------|------|
| `test_owner_hash_deterministic` | Same sk → same owner |
| `test_hash2_deterministic` | Same inputs → same hash |
| `test_hash2_order_matters` | hash2(a,b) ≠ hash2(b,a) |
| `test_nullifier_deterministic` | Same sk+nonce → same nullifier |
| `test_commitment_deterministic` | Same note → same commitment |
| `test_different_nonces_different_nullifiers` | Different nonces → different nullifiers |

## License

Apache-2.0
