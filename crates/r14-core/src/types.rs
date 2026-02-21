// Copyright 2026 abhirupbanerjee
// Licensed under the Apache License, Version 2.0

//! Type definitions for Groth16 verification (Root14 standard)

use soroban_sdk::crypto::bls12_381::{G1Affine, G2Affine};
use soroban_sdk::{contracttype, Vec};

/// Groth16 verification key for BLS12-381
///
/// IC is a unified vector: ic[0] is the constant term, ic[1..] are coefficients
/// for public inputs.
#[contracttype]
#[derive(Clone, Debug)]
pub struct VerificationKey {
    pub alpha_g1: G1Affine,
    pub beta_g2: G2Affine,
    pub gamma_g2: G2Affine,
    pub delta_g2: G2Affine,
    /// IC[0..n] in G1 — ic[0] is the constant term, ic[1..] match public inputs
    pub ic: Vec<G1Affine>,
}

/// Groth16 proof for BLS12-381
#[contracttype]
#[derive(Clone, Debug)]
pub struct Proof {
    pub a: G1Affine,
    pub b: G2Affine,
    pub c: G1Affine,
}
