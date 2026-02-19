# r14-types

**Shared type definitions for Root14 privacy protocol**

## Purpose

r14-types provides common data structures used across the Root14 ecosystem:
- On-chain contract (r14-kernel)
- Off-chain circuit (r14-circuit)
- Client libraries (r14-cli)
- Indexer (r14-indexer)

## Current Status: Phase 0 - Placeholder

### Planned Types

```rust
// Core privacy primitives
pub struct Note {
    pub value: u64,
    pub asset: AssetId,
    pub owner_pk: PublicKey,
    pub randomness: Fr,
}

pub struct Nullifier(pub Fr);

pub struct Commitment(pub Fr);

// Key management
pub struct ViewingKey(pub Fr);
pub struct SpendingKey(pub Fr);
pub struct PublicKey(pub G1Affine);

// Merkle tree
pub struct MerkleProof {
    pub path: Vec<Fr>,
    pub indices: Vec<bool>,
}

// Poseidon
pub struct PoseidonConfig {
    pub rounds_full: usize,
    pub rounds_partial: usize,
    pub alpha: u64,
}
```

## Implementation Status

- [ ] Note structure
- [ ] Nullifier/Commitment types
- [ ] Key derivation helpers
- [ ] Merkle proof types
- [ ] Poseidon config
- [ ] Serialization traits

## Dependencies

```toml
soroban-sdk = "25.1.1"  # For on-chain types
serde = "1"             # For off-chain serialization
```

## Usage (Future)

```rust
use r14_types::{Note, Nullifier, Commitment};

// Create note
let note = Note {
    value: 1000,
    asset: AssetId::native(),
    owner_pk: recipient_pk,
    randomness: Fr::random(),
};

// Compute commitment
let commitment = note.commit();

// Compute nullifier
let nullifier = note.nullify(&spending_key);
```

## Next Steps

- [ ] Define core types (Phase 0 GO confirmed)
- [ ] Align with r14-circuit requirements
- [ ] Add tests for serialization
- [ ] Document encoding formats

## License

Apache-2.0
