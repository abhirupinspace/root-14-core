# Root14

**Root14** is the ZK privacy standard for Stellar. It enables private transfers on Soroban using Groth16 zero-knowledge proofs over BLS12-381.

`r14-sdk` is the client library that gives your Rust application everything it needs to:

- Generate keys and manage wallets
- Create private notes (UTXOs) with Poseidon commitments
- Compute and verify Merkle roots
- Serialize Groth16 proofs for Soroban contracts
- Submit deposits and transfers on-chain

Pair it with `r14-circuit` for ZK proof generation to get the full private transfer pipeline.

## Who is this for?

- **Dapp builders** integrating private transfers into Rust backends or services
- **Tooling developers** building indexers, dashboards, or analytics for Root14
- **Protocol contributors** extending the Root14 standard

## How private transfers work

1. A sender creates a **Note** (value + owner + random nonce)
2. The note's **commitment** (Poseidon hash) is posted on-chain — the note data stays private
3. To spend, the sender generates a ZK proof that they know the note's secret key and the note exists in the Merkle tree
4. The proof reveals a **nullifier** (prevents double-spend) but nothing about the note itself
5. Two new output notes are created: one for the recipient, one for change

The on-chain contract only sees: commitments, nullifiers, Merkle roots, and proofs. All amounts and ownership remain private.

## Crate layout

| Crate | Role |
|-------|------|
| `r14-sdk` | Client library — wallet, merkle, soroban, serialization |
| `r14-circuit` | ZK proof generation and verification (Groth16) |
| `r14-types` | Core types (Note, SecretKey, MerklePath) — re-exported by SDK |
| `r14-poseidon` | Poseidon hash functions — re-exported by SDK |
| `r14-core` | Soroban contract: VK registry and proof verification |
| `r14-transfer` | Soroban contract: deposit and transfer logic |
| `r14-indexer` | Off-chain service: Merkle tree and event indexing |
| `r14-cli` | CLI tool wrapping the SDK |
