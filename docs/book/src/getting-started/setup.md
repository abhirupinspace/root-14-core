# Setup

## Dependencies

Add `r14-sdk` to your `Cargo.toml`. If you also need proof generation, add `r14-circuit`.

```toml
[dependencies]
r14-sdk     = { path = "crates/r14-sdk" }
r14-circuit = { path = "crates/r14-circuit" }  # only if you need proof generation
```

`r14-sdk` re-exports `r14-types` and `r14-poseidon` — you don't need to depend on them directly.

## What you get from `r14-sdk` alone

- Key generation and wallet management
- Note creation and commitment computation
- Nullifier derivation
- Merkle root computation (offline and via indexer)
- Proof/VK serialization for Soroban
- On-chain contract invocation

## What requires `r14-circuit`

- Groth16 trusted setup (`setup()`)
- Proof generation (`prove()`)
- Off-chain proof verification (`verify_offchain()`)

## Runtime requirements

- **Stellar CLI** — the `soroban` module shells out to `stellar` for contract invocation. Install from [stellar-cli](https://github.com/stellar/stellar-cli).
- **r14-indexer** — a running instance is needed for `merkle::compute_new_root` and balance sync.
- **Soroban testnet** — deployed `r14-core` and `r14-transfer` contracts.

These are only needed if you're submitting transactions on-chain. Offline operations (keygen, note creation, merkle computation) work without them.
