# prove

`r14_sdk::prove` — ZK proof generation (feature-gated).

Available when the `prove` feature is enabled:

```toml
[dependencies]
r14-sdk = { path = "crates/r14-sdk", features = ["prove"] }
```

This module re-exports everything from `r14-circuit` plus serialization helpers from `r14-sdk::serialize`, so `r14-sdk` becomes the single dependency for the full pipeline.

## Re-exports

| Item | Type | Description |
|------|------|-------------|
| `setup` | fn | Groth16 trusted setup for the transfer circuit |
| `prove` | fn | Generate a Groth16 proof for a private transfer |
| `verify_offchain` | fn | Verify a proof off-chain |
| `constraint_count` | fn | Count constraints in the transfer circuit |
| `TransferCircuit` | struct | The R1CS circuit for private transfers |
| `PublicInputs` | struct | Public inputs (old_root, nullifier, cm_0, cm_1) |
| `serialize_proof_for_soroban` | fn | Proof + public inputs → hex strings |
| `serialize_vk_for_soroban` | fn | Verification key → hex strings |
| `SerializedProof` | struct | Hex-encoded proof (a, b, c) |
| `SerializedVK` | struct | Hex-encoded verification key |

## Example

```rust,no_run
use ark_std::rand::{rngs::StdRng, SeedableRng};

let mut rng = StdRng::seed_from_u64(42);
let (pk, vk) = r14_sdk::prove::setup(&mut rng);

// ... build consumed note, merkle path, output notes ...
// let (proof, pi) = r14_sdk::prove::prove(&pk, sk, consumed, path, outputs, &mut rng);
// let (sp, spi) = r14_sdk::prove::serialize_proof_for_soroban(&proof, &pi.to_vec());
```
