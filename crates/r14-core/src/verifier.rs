// Copyright 2026 abhirupbanerjee
// Licensed under the Apache License, Version 2.0

//! Groth16 verifier using Soroban BLS12-381 host functions

use crate::types::{Proof, VerificationKey};
use soroban_sdk::crypto::bls12_381::{Fr, G1Affine};
use soroban_sdk::{BytesN, Env, Vec};

/// Verify a Groth16 proof using BLS12-381 pairing check
///
/// Algorithm:
/// 1. Compute L = IC[0] + MSM(IC[1..], public_inputs)
/// 2. Check: e(A,B) * e(-L,gamma) * e(-C,delta) * e(-alpha,beta) == 1
pub fn verify_groth16(
    env: &Env,
    vk: &VerificationKey,
    proof: &Proof,
    public_inputs: &Vec<Fr>,
) -> bool {
    let bls = env.crypto().bls12_381();

    let ic_0: G1Affine = vk.ic.get(0).expect("VK must have at least ic[0]");

    // Step 1: Compute L = IC[0] + MSM(IC[1..], public_inputs)
    let l = if public_inputs.is_empty() {
        ic_0
    } else {
        let ic_rest: Vec<G1Affine> = vk.ic.slice(1..);
        let msm_result = bls.g1_msm(ic_rest, public_inputs.clone());
        bls.g1_add(&ic_0, &msm_result)
    };

    // Step 2: Negate G1 points via scalar mul by -1
    let zero = Fr::from_bytes(BytesN::from_array(env, &[0u8; 32]));
    let one = Fr::from_bytes(BytesN::from_array(env, &{
        let mut b = [0u8; 32];
        b[31] = 1;
        b
    }));
    let neg_one = bls.fr_sub(&zero, &one);

    let neg_l = bls.g1_mul(&l, &neg_one);
    let neg_c = bls.g1_mul(&proof.c, &neg_one);
    let neg_alpha = bls.g1_mul(&vk.alpha_g1, &neg_one);

    // Step 3: Pairing check
    // e(A,B) * e(-L,gamma) * e(-C,delta) * e(-alpha,beta) == 1
    let g1_points: Vec<G1Affine> = Vec::from_array(
        env,
        [proof.a.clone(), neg_l, neg_c, neg_alpha],
    );
    let g2_points = Vec::from_array(
        env,
        [
            proof.b.clone(),
            vk.gamma_g2.clone(),
            vk.delta_g2.clone(),
            vk.beta_g2.clone(),
        ],
    );

    bls.pairing_check(g1_points, g2_points)
}
