// Copyright 2026 abhirupbanerjee
// Licensed under the Apache License, Version 2.0

//! r14-sdk: Serialization helpers for Root14 Groth16 types
//!
//! Converts arkworks BLS12-381 types into hex strings suitable for
//! Soroban contract consumption.

use ark_bls12_381::{Bls12_381, Fr, G1Affine, G2Affine};
use ark_serialize::CanonicalSerialize;

/// Serialized verification key (hex strings)
pub struct SerializedVK {
    pub alpha_g1: String,
    pub beta_g2: String,
    pub gamma_g2: String,
    pub delta_g2: String,
    /// ic[0] = constant term, ic[1..] = public input coefficients
    pub ic: Vec<String>,
}

/// Serialized Groth16 proof (hex strings)
pub struct SerializedProof {
    pub a: String,
    pub b: String,
    pub c: String,
}

/// Serialize G1 point to uncompressed hex (96 bytes = 192 hex chars)
pub fn serialize_g1(point: &G1Affine) -> String {
    let mut bytes = Vec::new();
    point.serialize_uncompressed(&mut bytes).unwrap();
    hex::encode(&bytes)
}

/// Serialize G2 point to uncompressed hex (192 bytes = 384 hex chars)
pub fn serialize_g2(point: &G2Affine) -> String {
    let mut bytes = Vec::new();
    point.serialize_uncompressed(&mut bytes).unwrap();
    hex::encode(&bytes)
}

/// Serialize Fr to big-endian hex (32 bytes = 64 hex chars)
///
/// arkworks uses LE serialization; Soroban Fr::from_bytes expects BE.
pub fn serialize_fr(fr: &Fr) -> String {
    let mut bytes = Vec::new();
    fr.serialize_compressed(&mut bytes).unwrap();
    bytes.reverse(); // LE -> BE for Soroban
    hex::encode(&bytes)
}

/// Convert an arkworks VerifyingKey to hex-serialized form
pub fn serialize_vk_for_soroban(vk: &ark_groth16::VerifyingKey<Bls12_381>) -> SerializedVK {
    SerializedVK {
        alpha_g1: serialize_g1(&vk.alpha_g1),
        beta_g2: serialize_g2(&vk.beta_g2),
        gamma_g2: serialize_g2(&vk.gamma_g2),
        delta_g2: serialize_g2(&vk.delta_g2),
        ic: vk.gamma_abc_g1.iter().map(serialize_g1).collect(),
    }
}

/// Convert an arkworks Proof + public inputs to hex-serialized form
pub fn serialize_proof_for_soroban(
    proof: &ark_groth16::Proof<Bls12_381>,
    public_inputs: &[Fr],
) -> (SerializedProof, Vec<String>) {
    let sp = SerializedProof {
        a: serialize_g1(&proof.a),
        b: serialize_g2(&proof.b),
        c: serialize_g1(&proof.c),
    };
    let pi: Vec<String> = public_inputs.iter().map(serialize_fr).collect();
    (sp, pi)
}
