# r14-poseidon

**Poseidon hash implementation for Soroban and circuits**

## Purpose

r14-poseidon provides Poseidon hash functions optimized for:
1. **On-chain** (Soroban contract) - Hash public inputs if compression needed
2. **Off-chain** (arkworks circuit) - Merkle tree, note commitments, nullifiers

## Current Status: Phase 0 - Not Started

**Will be implemented in Phase 1 or as fallback if Phase 0 exceeds instruction budget.**

## Why Poseidon?

- **ZK-friendly:** Efficient in arithmetic circuits (vs SHA-256)
- **Native field:** Works directly with BLS12-381 Fr elements
- **Flexible:** Configurable rounds for security/performance tradeoff

## Planned Architecture

```
┌─────────────────────────────────────────┐
│ r14-poseidon                            │
├─────────────────────────────────────────┤
│                                         │
│  Poseidon<Fr, WIDTH, ROUNDS>            │
│      │                                  │
│      ├─► new() → Self                   │
│      ├─► hash(inputs: &[Fr]) → Fr      │
│      └─► hash2(a: Fr, b: Fr) → Fr      │
│                                         │
│  Two implementations:                   │
│  ┌─────────────────────────────────┐   │
│  │ Soroban (no_std)               │   │
│  │ - Manual field arithmetic       │   │
│  │ - Optimized for contract size   │   │
│  └─────────────────────────────────┘   │
│                                         │
│  ┌─────────────────────────────────┐   │
│  │ Arkworks (std)                 │   │
│  │ - Uses ark-crypto-primitives    │   │
│  │ - For circuit synthesis         │   │
│  └─────────────────────────────────┘   │
│                                         │
└─────────────────────────────────────────┘
```

## Use Cases

### 1. Input Compression (Fallback)
If Phase 0 shows >100M instructions:
```rust
// Compress 3 public inputs into 1
let compressed = poseidon::hash(&[old_root, new_root, nullifier]);
verify_proof(vk, proof, vec![compressed]);
// Saves MSM cost: 2 fewer IC points
```

### 2. Merkle Tree (Phase 1)
```rust
fn merkle_hash(left: Fr, right: Fr) -> Fr {
    poseidon::hash2(left, right)
}
```

### 3. Note Commitment (Phase 1)
```rust
fn commit_note(note: &Note) -> Commitment {
    Commitment(poseidon::hash(&[
        note.value.into(),
        note.asset.into(),
        note.owner_pk.x,
        note.randomness,
    ]))
}
```

### 4. Nullifier Derivation (Phase 1)
```rust
fn nullify(commitment: Commitment, sk: SpendingKey) -> Nullifier {
    Nullifier(poseidon::hash2(commitment.0, sk.0))
}
```

## Implementation Plan

### Parameters (BLS12-381 Fr)
- **Width:** 3 (2 inputs + 1 capacity)
- **Full rounds:** 8
- **Partial rounds:** 57
- **S-box:** α = 5
- **Security:** 128-bit

Based on: [Poseidon specification](https://eprint.iacr.org/2019/458.pdf)

### Dependencies
```toml
# Circuit
ark-crypto-primitives = { version = "0.5", features = ["sponge"] }
ark-ff = "0.5"

# Contract (manual impl)
soroban-sdk = "25.1.1"
```

### Files
```
src/
├── lib.rs              # Feature flags (std/no_std)
├── params.rs           # Round constants, MDS matrix
├── soroban.rs          # Soroban implementation
└── arkworks.rs         # Arkworks wrapper

tests/
├── consistency.rs      # Soroban ↔ arkworks match
└── vectors.rs          # Test vectors from spec
```

## Performance Estimates

### On-Chain (Soroban)
- **hash2():** ~500K-1M instructions
- **hash([Fr; 3]):** ~1-2M instructions
- Much cheaper than MSM (~5-10M per point)

### In-Circuit (Constraints)
- **hash2():** ~300 R1CS constraints
- **Merkle proof (depth 20):** ~6K constraints

## Testing Strategy

1. **Test vector validation** - Match reference implementation
2. **Consistency check** - Soroban ↔ arkworks produce same hash
3. **Determinism** - Same inputs always hash to same output
4. **Collision resistance** - Spot checks

## Next Steps

- [ ] Phase 0 GO confirmed — input compression not needed as fallback
- [ ] Implement Soroban version (for Merkle trees in Phase 1)
- [ ] Generate/validate round constants
- [ ] Implement arkworks circuit gadget
- [ ] Benchmark on-chain instruction cost
- [ ] Add to r14-circuit for commitments/nullifiers

## Resources

- [Poseidon Paper](https://eprint.iacr.org/2019/458.pdf)
- [Reference Implementation](https://github.com/filecoin-project/neptune)
- [Arkworks Poseidon](https://github.com/arkworks-rs/crypto-primitives/tree/main/src/sponge/poseidon)

## License

Apache-2.0
