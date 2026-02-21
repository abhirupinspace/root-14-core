// Copyright 2026 abhirupbanerjee
// Licensed under the Apache License, Version 2.0

//! ZK proof generation (re-exports from `r14-circuit`).
//!
//! Available when the `prove` feature is enabled:
//!
//! ```toml
//! [dependencies]
//! r14-sdk = { workspace = true, features = ["prove"] }
//! ```

pub use r14_circuit::{
    constraint_count, prove, setup, verify_offchain, PublicInputs, TransferCircuit,
};

// Re-export serialization from r14-sdk::serialize for convenience
pub use crate::serialize::{
    serialize_proof_for_soroban, serialize_vk_for_soroban, SerializedProof, SerializedVK,
};
