// Copyright 2026 abhirupbanerjee
// Licensed under the Apache License, Version 2.0

//! Type definitions for Groth16 verification

use soroban_sdk::crypto::bls12_381::{G1Affine, G2Affine};
use soroban_sdk::{contracttype, Vec};

/// Groth16 verification key for BLS12-381
#[contracttype]
#[derive(Clone, Debug)]
pub struct VerificationKey {
    /// Alpha in G1
    pub alpha_g1: G1Affine,
    /// Beta in G2
    pub beta_g2: G2Affine,
    /// Gamma in G2
    pub gamma_g2: G2Affine,
    /// Delta in G2
    pub delta_g2: G2Affine,
    /// IC[0] in G1 (constant term)
    pub ic_0: G1Affine,
    /// IC[1..] in G1 (coefficients for public inputs)
    pub ic_rest: Vec<G1Affine>,
}

/// Groth16 proof for BLS12-381
#[contracttype]
#[derive(Clone, Debug)]
pub struct Proof {
    /// A in G1
    pub a: G1Affine,
    /// B in G2
    pub b: G2Affine,
    /// C in G1
    pub c: G1Affine,
}

// Note: Vec<Fr> is used directly in function signatures instead of type alias
// to avoid contracttype export issues
