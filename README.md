# Root14 (r14)

**Privacy-preserving transactions on Stellar using Groth16 zero-knowledge proofs.**

## Architecture

```
                          ┌────────────┐
                          │  r14-cli   │
                          └─────┬──────┘
                                │
                   ┌────────────┼────────────┐
                   ▼            ▼            ▼
             ┌──────────┐ ┌──────────┐ ┌───────────┐
             │ r14-sdk  │ │r14-circuit│ │r14-indexer│
             └────┬─────┘ └────┬─────┘ └───────────┘
                  │            │
         ┌────────┼────────────┤
         ▼        ▼            ▼
   ┌───────────┐ ┌──────────┐ ┌────────────┐
   │r14-poseidon│ │r14-types │ │r14-circuits│
   └───────────┘ └──────────┘ └────────────┘

   On-chain (Soroban):
   ┌──────────┐    cross-contract    ┌──────────────┐
   │ r14-core │◄────────────────────│ r14-transfer  │
   │(verifier)│                      │ (privacy app) │
   └──────────┘                      └──────────────┘
```

## Quickstart

```bash
# build
cargo build --workspace

# generate wallet
r14 keygen

# configure
r14 config set stellar_secret S...
r14 config set core_contract_id CA...
r14 config set transfer_contract_id CB...

# deposit
r14 deposit 1000

# transfer
r14 transfer 700 0x<recipient_owner_hash>

# check balance
r14 balance
```

## Crates

| Crate | Description |
|-------|-------------|
| `r14-types` | Shared types: Note, Nullifier, SecretKey, MerklePath |
| `r14-poseidon` | Poseidon hash (commitment, nullifier, owner_hash, hash2) |
| `r14-circuit` | 1-in-2-out transfer circuit (Groth16/BLS12-381, 7638 constraints) |
| `r14-circuits` | Pre-built ZK circuits (preimage, ownership, membership, range) |
| `r14-sdk` | Client SDK: wallet, merkle, serialization, soroban invocation |
| `r14-cli` | CLI: keygen, deposit, transfer, balance, init-contract, status |
| `r14-indexer` | Event scanner + Poseidon Merkle tree (depth 20) + REST API |
| `r14-core` | Soroban contract: general-purpose Groth16 verifier registry |
| `r14-transfer` | Soroban contract: private transfer app (calls r14-core) |

## Pre-built Circuits (`r14-circuits`)

| Circuit | Statement | Public Inputs |
|---------|-----------|---------------|
| **Preimage** | "I know `x` such that `Poseidon(x) == hash`" | hash |
| **Ownership** | "I know `sk` such that `Poseidon(sk) == owner_hash`" | owner_hash |
| **Membership** | "leaf is in Merkle tree with given root" | root, leaf_commitment |
| **Range** | "committed value is within `[min, max]`" | min, max, commitment |

## CLI Reference

```
r14 keygen                            # generate keypair + wallet
r14 deposit <value> [--app-tag N]     # create note + submit on-chain
r14 deposit <value> --local-only      # create note without submitting
r14 transfer <value> <recipient>      # private transfer with ZK proof
r14 transfer <value> <recipient> --dry-run  # generate proof only
r14 balance                           # sync with indexer, show balance
r14 init-contract                     # register VK + initialize contracts
r14 status                            # wallet + indexer health
r14 config set <key> <value>          # set config value
r14 config show                       # show current config
r14 compute-root [commitments...]     # offline merkle root computation
r14 --version                         # print version
r14 --json <command>                  # machine-readable JSON output
```

## Build & Test

```bash
cargo build --workspace              # build all crates
cargo test --workspace               # run all tests
cargo fmt --all                      # format
cargo clippy --all-targets           # lint

# build Soroban contract WASMs
stellar contract build --package r14-core
stellar contract build --package r14-transfer
```

## License

Apache-2.0
