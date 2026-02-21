# Architecture

## Stack overview

```text
┌─────────────────────────────────────────────────┐
│                   Your Dapp                     │
├─────────────────────────────────────────────────┤
│  r14-sdk                                        │
│  ┌──────────┐ ┌────────┐ ┌────────┐ ┌────────┐ │
│  │  wallet   │ │ merkle │ │soroban │ │  ser.  │ │
│  └──────────┘ └────────┘ └────────┘ └────────┘ │
│  re-exports: SecretKey, Note, commitment, ...   │
├─────────────────────────────────────────────────┤
│  r14-sdk feature "prove" (optional)              │
│  prove::setup() · prove::prove() · ...          │
├─────────────────────────────────────────────────┤
│  Stellar / Soroban                              │
│  r14-core contract · r14-transfer contract      │
└─────────────────────────────────────────────────┘
```

## Module responsibilities

| Module | What it does |
|--------|-------------|
| *crate root* | Re-exports core types (`SecretKey`, `Note`, `commitment`, `nullifier`, `owner_hash`, `hash2`, etc.) |
| `wallet` | JSON wallet persistence at `~/.r14/wallet.json`, hex-to-Fr conversion, RNG |
| `merkle` | Sparse Merkle tree root computation — offline from leaf list or live via indexer |
| `soroban` | Thin async wrapper around the `stellar` CLI for contract invocation |
| `serialize` | Converts arkworks Groth16 types (G1, G2, Fr, VK, Proof) into hex strings for Soroban |
| `prove` | ZK proof generation — feature-gated, enable with `features = ["prove"]` |

## Data flow: deposit

```text
SecretKey ──→ owner_hash ──→ Note::new(value, tag, owner)
                                │
                                ▼
                           commitment(note) ──→ cm (Fr)
                                │
                        ┌───────┴───────┐
                        ▼               ▼
                  save to wallet   compute_new_root
                                        │
                                        ▼
                               invoke_contract("deposit")
                                        │
                                        ▼
                                   on-chain
```

## Data flow: private transfer

```text
load consumed note from wallet
        │
        ▼
fetch merkle proof from indexer ──→ MerklePath
        │
        ▼
build output notes (recipient + change)
        │
        ▼
r14_sdk::prove::prove(sk, consumed, path, outputs)
        │
        ▼
serialize_proof_for_soroban ──→ hex strings
        │
        ▼
compute_new_root(indexer, [cm_0, cm_1])
        │
        ▼
invoke_contract("transfer", proof, nullifier, roots, cms)
        │
        ▼
mark consumed note as spent, save new notes to wallet
```

## On-chain contracts

**r14-core** — verification key registry. Stores Groth16 VKs and verifies proofs. Shared across applications.

**r14-transfer** — deposit and transfer logic. Maintains the Merkle root, nullifier set, and emits events. Calls r14-core for proof verification.
