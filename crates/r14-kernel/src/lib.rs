// Copyright 2026 abhirupbanerjee
// Licensed under the Apache License, Version 2.0

//! r14-kernel: Root14 Groth16 verifier on Soroban
//! Phase 0: Feasibility spike for BLS12-381 verification

#![no_std]

mod contract;
mod test_vectors;
mod types;
mod verifier;

pub use contract::*;
pub use types::*;
pub use verifier::*;
