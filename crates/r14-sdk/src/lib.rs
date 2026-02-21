// Copyright 2026 abhirupbanerjee
// Licensed under the Apache License, Version 2.0

//! # r14-sdk
//!
//! Client library for **Root14** — the ZK privacy standard for Stellar.
//!
//! `r14-sdk` provides everything a dapp needs to create private notes,
//! manage wallets, compute Merkle roots, serialize proofs for Soroban,
//! and submit transactions on-chain. Pair it with `r14-circuit` for
//! ZK proof generation to get the full private transfer pipeline.
//!
//! ## Crate layout
//!
//! | Module | Purpose |
//! |---|---|
//! | *crate root* | Re-exports core types (`SecretKey`, `Note`, `commitment`, …) |
//! | [`wallet`] | Key/note persistence, hex ↔ `Fr` conversion |
//! | [`merkle`] | Offline and indexer-backed Merkle root computation |
//! | [`soroban`] | Stellar CLI wrapper for on-chain contract invocation |
//! | [`serialize`] | Arkworks → hex serialization for Soroban contracts |
//!
//! ## Quick start
//!
//! ```toml
//! [dependencies]
//! r14-sdk     = { path = "crates/r14-sdk" }
//! r14-circuit = { path = "crates/r14-circuit" }   # for proof generation
//! ```
//!
//! ## Typical integration flow
//!
//! ```rust,no_run
//! use r14_sdk::{SecretKey, Note, owner_hash, commitment, nullifier};
//! use r14_sdk::wallet::{self, fr_to_hex, hex_to_fr};
//!
//! # fn example() -> anyhow::Result<()> {
//! // 1. Keygen
//! let mut rng = wallet::crypto_rng();
//! let sk = SecretKey::random(&mut rng);
//! let owner = owner_hash(&sk);
//!
//! // 2. Create a private note
//! let note = Note::new(1_000, 1, owner.0, &mut rng);
//! let cm = commitment(&note);
//!
//! // 3. Persist to wallet
//! let mut w = wallet::load_wallet()?;
//! w.notes.push(wallet::NoteEntry {
//!     value: note.value,
//!     app_tag: note.app_tag,
//!     owner: fr_to_hex(&note.owner),
//!     nonce: fr_to_hex(&note.nonce),
//!     commitment: fr_to_hex(&cm),
//!     index: None,
//!     spent: false,
//! });
//! wallet::save_wallet(&w)?;
//!
//! // 4. Compute Merkle root (offline or via indexer)
//! let root = r14_sdk::merkle::compute_root_from_leaves(&[cm]);
//!
//! // 5. Generate proof (via r14-circuit — separate crate)
//! //    let (proof, pi) = r14_circuit::prove(&pk, sk, note, path, outputs, &mut rng);
//!
//! // 6. Serialize for Soroban
//! //    let (sp, spi) = r14_sdk::serialize::serialize_proof_for_soroban(&proof, &pi_vec);
//!
//! // 7. Submit on-chain
//! //    r14_sdk::soroban::invoke_contract(&contract_id, "testnet", &secret, "deposit", &args).await?;
//! # Ok(())
//! # }
//! ```

// Re-exports from r14-types
pub use r14_types::{MerklePath, MerkleRoot, Note, Nullifier, SecretKey, MERKLE_DEPTH};

// Re-exports from r14-poseidon
pub use r14_poseidon::{commitment, hash2, nullifier, owner_hash};

pub mod merkle;
pub mod serialize;
pub mod soroban;
pub mod wallet;
