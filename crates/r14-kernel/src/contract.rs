// Copyright 2026 abhirupbanerjee
// Licensed under the Apache License, Version 2.0

//! Root14 Kernel - Groth16 verifier contract

use crate::types::{Proof, VerificationKey};
use crate::verifier::verify_groth16;
use soroban_sdk::crypto::bls12_381::{Fr, G1Affine, G2Affine};
use soroban_sdk::{contract, contractimpl, contracttype, BytesN, Env, Vec};

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Vk,
    Nullifier(BytesN<32>),
}

#[contract]
pub struct R14Kernel;

#[contractimpl]
impl R14Kernel {
    /// Verify a Groth16 proof for the dummy circuit (Phase 0 test)
    ///
    /// This function uses hardcoded test vectors for the feasibility 
    /// spike.
    /// In production, VK will be stored in contract storage and proof/inputs
    /// will be passed as parameters.
    pub fn verify_dummy_proof(env: Env) -> bool {
        // Parse test vectors from hex strings
        let vk = Self::parse_verification_key(&env);
        let proof = Self::parse_proof(&env);
        let public_inputs = Self::parse_public_inputs(&env);

        verify_groth16(&env, &vk, &proof, &public_inputs)
    }

    /// Verify a Groth16 proof with provided parameters
    ///
    /// Production interface for verification
    pub fn verify_proof(
        env: Env,
        vk: VerificationKey,
        proof: Proof,
        public_inputs: Vec<Fr>,
    ) -> bool {
        verify_groth16(&env, &vk, &proof, &public_inputs)
    }

    /// Deposit a commitment into the pool (emits event for indexer)
    pub fn deposit(env: Env, cm: BytesN<32>) {
        env.events().publish(("deposit",), (cm,));
    }

    /// Store verification key (one-time initialization)
    pub fn init(env: Env, vk: VerificationKey) {
        if env.storage().persistent().has(&DataKey::Vk) {
            panic!("already initialized");
        }
        env.storage().persistent().set(&DataKey::Vk, &vk);
    }

    /// Verify a private transfer and mark nullifier as spent
    ///
    /// All field elements are passed as BytesN<32> (big-endian) for clean
    /// contracttype boundary.
    pub fn transfer(
        env: Env,
        proof: Proof,
        old_root: BytesN<32>,
        nullifier: BytesN<32>,
        cm_0: BytesN<32>,
        cm_1: BytesN<32>,
    ) -> bool {
        // Load VK
        let vk: VerificationKey = env
            .storage()
            .persistent()
            .get(&DataKey::Vk)
            .expect("not initialized");

        // Check nullifier not already spent
        let nf_key = DataKey::Nullifier(nullifier.clone());
        if env.storage().persistent().has(&nf_key) {
            panic!("nullifier already spent");
        }

        // Convert BytesN<32> → Fr
        let old_root_fr = Fr::from_bytes(old_root);
        let nullifier_fr = Fr::from_bytes(nullifier.clone());
        let cm_0_fr = Fr::from_bytes(cm_0.clone());
        let cm_1_fr = Fr::from_bytes(cm_1.clone());

        let public_inputs = Vec::from_array(
            &env,
            [old_root_fr, nullifier_fr, cm_0_fr, cm_1_fr],
        );

        // Verify proof
        if !verify_groth16(&env, &vk, &proof, &public_inputs) {
            panic!("proof verification failed");
        }

        // Mark nullifier as spent
        env.storage().persistent().set(&nf_key, &true);

        // Emit event
        env.events()
            .publish(("transfer",), (nullifier, cm_0, cm_1));

        true
    }

    fn parse_verification_key(env: &Env) -> VerificationKey {
        use crate::test_vectors::*;

        VerificationKey {
            alpha_g1: Self::hex_to_g1(env, VK_ALPHA_G1),
            beta_g2: Self::hex_to_g2(env, VK_BETA_G2),
            gamma_g2: Self::hex_to_g2(env, VK_GAMMA_G2),
            delta_g2: Self::hex_to_g2(env, VK_DELTA_G2),
            ic_0: Self::hex_to_g1(env, VK_IC_0),
            ic_rest: Vec::from_array(env, [Self::hex_to_g1(env, VK_IC_1)]),
        }
    }

    fn parse_proof(env: &Env) -> Proof {
        use crate::test_vectors::*;

        Proof {
            a: Self::hex_to_g1(env, PROOF_A),
            b: Self::hex_to_g2(env, PROOF_B),
            c: Self::hex_to_g1(env, PROOF_C),
        }
    }

    fn parse_public_inputs(env: &Env) -> Vec<Fr> {
        use crate::test_vectors::*;

        Vec::from_array(env, [Self::hex_to_fr(env, PUBLIC_INPUT)])
    }

    fn hex_char_to_u8(c: u8) -> u8 {
        match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            b'A'..=b'F' => c - b'A' + 10,
            _ => 0,
        }
    }

    fn hex_to_g1(env: &Env, hex: &str) -> G1Affine {
        use soroban_sdk::BytesN;
        let hex_bytes = hex.as_bytes();
        let mut arr = [0u8; 96];
        let mut i = 0;
        while i < hex_bytes.len() {
            let high = Self::hex_char_to_u8(hex_bytes[i]);
            let low = Self::hex_char_to_u8(hex_bytes[i + 1]);
            arr[i / 2] = (high << 4) | low;
            i += 2;
        }
        G1Affine::from_bytes(BytesN::from_array(env, &arr))
    }

    fn hex_to_g2(env: &Env, hex: &str) -> G2Affine {
        use soroban_sdk::BytesN;
        let hex_bytes = hex.as_bytes();
        let mut arr = [0u8; 192];
        let mut i = 0;
        while i < hex_bytes.len() {
            let high = Self::hex_char_to_u8(hex_bytes[i]);
            let low = Self::hex_char_to_u8(hex_bytes[i + 1]);
            arr[i / 2] = (high << 4) | low;
            i += 2;
        }
        G2Affine::from_bytes(BytesN::from_array(env, &arr))
    }

    fn hex_to_fr(env: &Env, hex: &str) -> Fr {
        use soroban_sdk::BytesN;
        let hex_bytes = hex.as_bytes();
        let mut arr = [0u8; 32];
        let mut i = 0;
        while i < hex_bytes.len() {
            let high = Self::hex_char_to_u8(hex_bytes[i]);
            let low = Self::hex_char_to_u8(hex_bytes[i + 1]);
            arr[i / 2] = (high << 4) | low;
            i += 2;
        }
        Fr::from_bytes(BytesN::from_array(env, &arr))
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::crypto::bls12_381::Fr;
    use soroban_sdk::{BytesN, Env};

    #[test]
    fn test_dummy_verification() {
        let env = Env::default();
        assert!(R14Kernel::verify_dummy_proof(env));
    }

    #[test]
    fn test_wrong_public_input_fails() {
        let env = Env::default();
        let vk = R14Kernel::parse_verification_key(&env);
        let proof = R14Kernel::parse_proof(&env);
        // y=15 instead of y=14
        let wrong_input = {
            let mut b = [0u8; 32];
            b[31] = 15;
            b
        };
        let public_inputs = Vec::from_array(
            &env,
            [Fr::from_bytes(BytesN::from_array(&env, &wrong_input))],
        );
        let result = crate::verifier::verify_groth16(&env, &vk, &proof, &public_inputs);
        assert!(!result, "Wrong public input must fail");
    }

    #[test]
    fn test_tampered_proof_a_fails() {
        let env = Env::default();
        let vk = R14Kernel::parse_verification_key(&env);
        let proof = R14Kernel::parse_proof(&env);
        let public_inputs = R14Kernel::parse_public_inputs(&env);

        // Swap proof.a with a different valid G1 point (use IC_0 from VK)
        let tampered = crate::types::Proof {
            a: vk.ic_0.clone(),
            b: proof.b.clone(),
            c: proof.c.clone(),
        };
        let result = crate::verifier::verify_groth16(&env, &vk, &tampered, &public_inputs);
        assert!(!result, "Tampered proof.a must fail");
    }

    #[test]
    fn test_tampered_proof_c_fails() {
        let env = Env::default();
        let vk = R14Kernel::parse_verification_key(&env);
        let proof = R14Kernel::parse_proof(&env);
        let public_inputs = R14Kernel::parse_public_inputs(&env);

        // Swap proof.c with proof.a
        let tampered = crate::types::Proof {
            a: proof.a.clone(),
            b: proof.b.clone(),
            c: proof.a.clone(),
        };
        let result = crate::verifier::verify_groth16(&env, &vk, &tampered, &public_inputs);
        assert!(!result, "Tampered proof.c must fail");
    }
}
