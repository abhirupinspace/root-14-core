# Prerequisites

## Required for all usage

- **Rust** — edition 2021 (stable or nightly)

## Required for on-chain operations

- **[Stellar CLI](https://github.com/stellar/stellar-cli)** — the `soroban` module shells out to the `stellar` binary for contract invocation and key derivation. Must be on `$PATH`.

- **r14-indexer** — a running instance is needed for:
  - `merkle::compute_new_root` (fetches existing leaves)
  - Balance sync (queries leaf index by commitment)
  - Merkle proof retrieval (for transfers)

- **Soroban testnet** — deployed instances of:
  - `r14-core` — verification key registry
  - `r14-transfer` — deposit and transfer contract

- **Stellar account** — a funded testnet account with a secret key (`S...`)

## Not required

These operations work fully offline with no external dependencies:

- Key generation (`SecretKey::random`)
- Note creation (`Note::new`)
- Commitment/nullifier computation
- `merkle::empty_root_hex`, `merkle::compute_root_from_leaves`
- Proof serialization (`serialize::*`)
- Wallet load/save (local filesystem only)

## Installing Stellar CLI

```bash
cargo install stellar-cli
```

Or follow the [official instructions](https://github.com/stellar/stellar-cli#installation).

## Running the indexer

```bash
cargo run -p r14-indexer
```

By default, the indexer listens on `http://localhost:3000`.
