# r14-kernel

**Soroban smart contract for on-chain Groth16 proof verification**

## Current Status: Phase 3 - SHIPPED

**Testnet:** `CDV6FRX7GFHZIRYB474LNW4325V7HYHD6WXBDHW4C2XEMCYPT4NF3GPN`
**WASM:** 10.3KB | **Tests:** 9 passing

### Entrypoints

| Function | Purpose |
|----------|---------|
| `init(vk)` | Store verification key (one-time) |
| `deposit(cm)` | Accept commitment, emit deposit event |
| `transfer(proof, old_root, nullifier, cm_0, cm_1)` | Verify transfer + mark nullifier spent |
| `verify_proof(vk, proof, inputs)` | Generic Groth16 verifier |
| `verify_dummy_proof()` | Phase 0 hardcoded test |

### Storage

| Key | Value |
|-----|-------|
| `DataKey::Vk` | VerificationKey (persistent) |
| `DataKey::Nullifier(BytesN<32>)` | bool (persistent) |

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│ R14Kernel Contract                                       │
├──────────────────────────────────────────────────────────┤
│                                                          │
│  init(vk)                                                │
│    └─► store VK in persistent storage                    │
│                                                          │
│  deposit(cm: BytesN<32>)                                 │
│    └─► emit ("deposit", (cm,)) event for indexer         │
│                                                          │
│  transfer(proof, old_root, nullifier, cm_0, cm_1)        │
│    ├─► load VK from storage                              │
│    ├─► check nullifier not spent                         │
│    ├─► convert BytesN<32> → Fr (4 public inputs)         │
│    ├─► verify_groth16(vk, proof, inputs)                 │
│    │     ├─► L = IC[0] + g1_msm(IC[1..], inputs)        │
│    │     └─► pairing_check(4 pairs)                      │
│    ├─► mark nullifier spent                              │
│    └─► emit ("transfer", (nullifier, cm_0, cm_1)) event  │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

## File Structure

```
src/
├── lib.rs              # Module exports, #![no_std]
├── contract.rs         # R14Kernel: init, deposit, transfer, verify_proof, verify_dummy_proof
├── verifier.rs         # verify_groth16() — pairing equation check
├── types.rs            # VerificationKey, Proof (#[contracttype])
├── test_vectors.rs     # Hardcoded Phase 0 hex constants
└── test_vectors_bytes.rs  # (unused, partial byte arrays)

tests/
├── proof_generator.rs  # Phase 0 dummy circuit test vector gen
└── transfer_e2e.rs     # E2E: off-chain prove → on-chain verify (4 tests)
```

## Tests

```bash
cargo test -p r14-kernel
# 9 tests: 4 unit + 1 proof_gen + 4 E2E
```

| Test | What |
|------|------|
| `test_dummy_verification` | Phase 0 hardcoded proof accepted |
| `test_wrong_public_input_fails` | Wrong y-value rejected |
| `test_tampered_proof_a_fails` | Swapped proof.a rejected |
| `test_tampered_proof_c_fails` | Swapped proof.c rejected |
| `generate_test_vectors` | Off-chain arkworks proof gen |
| `test_transfer_e2e` | Full happy path: prove → init → transfer |
| `test_double_spend_rejected` | Same nullifier twice panics |
| `test_invalid_proof_rejected` | Tampered proof.a panics |
| `test_wrong_nullifier_rejected` | Mismatched nullifier panics |

## BLS12-381 Host Functions Used

| Function | Purpose |
|----------|---------|
| `g1_msm(points, scalars)` | Multi-scalar multiplication |
| `g1_add(p1, p2)` | Point addition |
| `g1_mul(point, scalar)` | Scalar multiplication |
| `pairing_check(g1s, g2s)` | Multi-pairing verification |
| `fr_sub(a, b)` | Field subtraction |

## Groth16 Verification Equation

```
e(A, B) · e(-L, gamma) · e(-C, delta) · e(-alpha, beta) = 1
```

Where `L = IC[0] + Σ(IC[i] · public_input[i-1])` for i=1..n

## Build

```bash
stellar contract build --package r14-kernel
# Output: target/wasm32v1-none/release/r14_kernel.wasm (10.3KB)
```

Note: uses `stellar contract build` (not raw `cargo build`) to target `wasm32v1-none` correctly.

## Known Issues

- `publish()` deprecated — migrate to `#[contractevent]`
- No admin auth on `init()`
- No storage TTL / `extend_ttl()`
- No on-chain Merkle root tracking — contract trusts `old_root` from caller

## License

Apache-2.0
