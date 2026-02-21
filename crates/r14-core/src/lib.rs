// Copyright 2026 abhirupbanerjee
// Licensed under the Apache License, Version 2.0

//! r14-core: Root14 general-purpose Groth16 verifier standard on Soroban

#![no_std]

mod contract;
mod types;
mod verifier;

pub use contract::*;
pub use types::*;
pub use verifier::*;
