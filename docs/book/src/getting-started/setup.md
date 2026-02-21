# Setup

## Dependencies

Add `r14-sdk` to your `Cargo.toml`. Enable the `prove` feature for ZK proof generation.

```toml
[dependencies]
r14-sdk = { path = "crates/r14-sdk" }

# Include proof generation (pulls in r14-circuit automatically):
# r14-sdk = { path = "crates/r14-sdk", features = ["prove"] }
```

`r14-sdk` re-exports `r14-types` and `r14-poseidon` — you don't need to depend on them directly.

## What you get from `r14-sdk` alone

- Key generation and wallet management
- Note creation and commitment computation
- Nullifier derivation
- Merkle root computation (offline and via indexer)
- Proof/VK serialization for Soroban
- On-chain contract invocation

## What requires the `prove` feature

- Groth16 trusted setup (`r14_sdk::prove::setup()`)
- Proof generation (`r14_sdk::prove::prove()`)
- Off-chain proof verification (`r14_sdk::prove::verify_offchain()`)

## Runtime requirements

- **Stellar CLI** — the `soroban` module shells out to `stellar` for contract invocation. Install from [stellar-cli](https://github.com/stellar/stellar-cli).
- **r14-indexer** — a running instance is needed for `merkle::compute_new_root` and balance sync.
- **Soroban testnet** — deployed `r14-core` and `r14-transfer` contracts.

These are only needed if you're submitting transactions on-chain. Offline operations (keygen, note creation, merkle computation) work without them.
