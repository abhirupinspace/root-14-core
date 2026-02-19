# r14-circuit

**Off-chain Groth16 circuit for Root14 privacy transactions**

## Current Status: SHIPPED

**Constraints:** 7,638 | **Public inputs:** 4 | **Tests:** 7 passing

## Circuit: TransferCircuit (1-in-2-out)

### Statement
"I know a note in the commitment tree, I can spend it, the nullifier is correct, and output notes conserve value."

### Public Inputs (4)
1. **old_root** — Merkle root (inclusion proof)
2. **nullifier** — Poseidon(secret_key, nonce)
3. **out_commitment_0** — commitment to first output note
4. **out_commitment_1** — commitment to second output note

### Private Witnesses
- **secret_key** — proves note ownership
- **consumed_note** — Note being spent (value, app_tag, owner, nonce)
- **merkle_path** — 20 siblings + 20 direction bits
- **created_notes** — [recipient_note, change_note]

### Constraints
1. **Ownership:** `consumed.owner == Poseidon(secret_key)`
2. **Inclusion:** Merkle path hashes up to `old_root`
3. **Nullifier:** `nullifier == Poseidon(secret_key, consumed.nonce)`
4. **Commitments:** `cm_i == Poseidon(value, app_tag, owner, nonce)` for each output
5. **Value conservation:** `consumed.value == created[0].value + created[1].value`
6. **App tag:** `consumed.app_tag == created[i].app_tag`

## API

```rust
// Trusted setup
let (pk, vk) = r14_circuit::setup(&mut rng);

// Prove
let (proof, public_inputs) = r14_circuit::prove(
    &pk, secret_key, consumed_note, merkle_path, created_notes, &mut rng
);

// Verify off-chain
assert!(r14_circuit::verify_offchain(&vk, &proof, &public_inputs));

// Serialize for Soroban
let svk = r14_circuit::serialize_vk_for_soroban(&vk);
let (sp, spi) = r14_circuit::serialize_proof_for_soroban(&proof, &public_inputs);
```

## File Structure

```
src/
├── lib.rs              # setup, prove, verify_offchain, serialization
├── transfer.rs         # TransferCircuit (ConstraintSynthesizer impl)
├── poseidon_gadget.rs  # poseidon_hash_var, hash2_var (PoseidonSpongeVar)
└── merkle_gadget.rs    # verify_merkle_path (depth 20)
```

## Serialization

| Type | Bytes | Hex chars | Format |
|------|-------|-----------|--------|
| G1 | 96 | 192 | arkworks uncompressed (BE/Zcash) |
| G2 | 192 | 384 | arkworks uncompressed (BE/Zcash) |
| Fr | 32 | 64 | arkworks LE → reversed to BE for Soroban |

**VK IC length:** 5 (ic[0] constant + 4 for public inputs)

## Tests

```bash
cargo test -p r14-circuit
# 7 tests
```

| Test | What |
|------|------|
| `test_valid_transfer` | Full setup/prove/verify roundtrip |
| `test_wrong_secret_key` | Wrong sk → constraints unsatisfied |
| `test_wrong_merkle_path` | Tampered root → verify fails |
| `test_value_mismatch` | 600+300≠1000 → unsatisfied |
| `test_constraint_count` | 1K < count < 20K |
| `test_serialization_roundtrip` | IC=5, G1=192ch, G2=384ch, Fr=64ch |
| `test_app_tag_mismatch` | tag 1 vs 2 → unsatisfied |

## Benchmarks

| Metric | Value |
|--------|-------|
| Constraints | 7,638 |
| Proof size | 384 bytes |
| Proof generation | ~10-15s (dev machine) |
| VK IC points | 5 |
| Merkle depth | 20 (1M capacity) |

## License

Apache-2.0
