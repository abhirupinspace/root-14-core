# Core Concepts

## Notes (UTXOs)

Root14 uses a UTXO model. Each private balance is a **Note**:

```rust
pub struct Note {
    pub value: u64,     // amount
    pub app_tag: u32,   // application identifier (must match in transfers)
    pub owner: Fr,      // owner_hash — derived from secret key
    pub nonce: Fr,      // random, makes each note unique
}
```

A note's **commitment** is `Poseidon(value, app_tag, owner, nonce)`. This is what goes on-chain. The note data itself stays private — only the creator knows the preimage.

## Keys

```rust
// secret key — random BLS12-381 field element, never leaves your machine
let sk = SecretKey::random(&mut rng);

// owner hash — Poseidon(sk), safe to share publicly
let owner = owner_hash(&sk);
```

**Give your `owner_hash` to senders.** They use it as the `owner` field when creating notes addressed to you. You prove ownership by demonstrating knowledge of the secret key whose hash matches `owner`.

## Nullifiers

When spending a note, you reveal its **nullifier**: `Poseidon(sk, nonce)`.

The on-chain contract records every nullifier it has seen. If a nullifier appears twice, the transaction is rejected — this prevents double-spending.

The nullifier is deterministic (same sk + nonce = same nullifier) but reveals nothing about which note was spent, the amount, or the owner.

## Commitments

A commitment is a Poseidon hash of the full note:

```
commitment = Poseidon(value, app_tag, owner, nonce)
```

Commitments are stored on-chain as leaves in the Merkle tree. To spend a note, you prove (in zero knowledge) that you know a note whose commitment exists in the tree.

## Merkle tree

All commitments live in a **sparse Merkle tree**:

- **Depth**: 20 (supports up to 2^20 = ~1M notes)
- **Hash function**: Poseidon `hash2`
- **Empty leaf value**: `Fr::ZERO`
- **Storage**: the on-chain contract only stores the root; the full tree is maintained by the indexer

To prove a note exists in the tree, you provide a **Merkle path** (20 sibling hashes + 20 index bits). The ZK circuit verifies this path recomputes the correct root.

## App tags

Each note carries an `app_tag` — an application identifier. During a transfer, the circuit enforces that input and output notes share the same `app_tag`. This enables multiple applications to use Root14's privacy infrastructure while keeping their note pools isolated.

## Transfer constraints

The ZK circuit enforces:

1. **Ownership**: prover knows `sk` such that `owner_hash(sk) == consumed_note.owner`
2. **Inclusion**: consumed note's commitment exists in the Merkle tree (valid path to root)
3. **Value conservation**: `consumed.value == output_0.value + output_1.value`
4. **App tag preservation**: all notes share the same `app_tag`
5. **Correct nullifier**: nullifier is correctly derived from `sk` and `nonce`
6. **Correct output commitments**: output commitments match the declared output notes
