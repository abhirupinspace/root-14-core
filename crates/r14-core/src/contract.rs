// Copyright 2026 abhirupbanerjee
// Licensed under the Apache License, Version 2.0

//! R14 Core — general-purpose Groth16 verifier registry

use crate::types::{Proof, VerificationKey};
use crate::verifier::verify_groth16;
use soroban_sdk::crypto::bls12_381::Fr;
use soroban_sdk::{contract, contractimpl, contracttype, Address, Bytes, BytesN, Env, Vec};

#[contracttype]
#[derive(Clone, Debug)]
pub struct VerifyEvent {
    pub circuit_id: BytesN<32>,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Admin,
    Circuit(BytesN<32>),
}

const PERSISTENT_TTL: u32 = 535_680; // ~30 days
const PERSISTENT_THRESHOLD: u32 = 267_840; // ~15 days

#[contract]
pub struct R14Core;

#[contractimpl]
impl R14Core {
    /// Initialize with admin address
    pub fn init(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .extend_ttl(PERSISTENT_THRESHOLD, PERSISTENT_TTL);
    }

    /// Register a verification key, returns content-addressed circuit_id
    pub fn register(env: Env, caller: Address, vk: VerificationKey) -> BytesN<32> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("not initialized");
        admin.require_auth();
        if caller != admin {
            panic!("only admin can register");
        }

        let circuit_id = Self::compute_circuit_id(&env, &vk);
        let key = DataKey::Circuit(circuit_id.clone());

        if env.storage().persistent().has(&key) {
            panic!("circuit already registered");
        }

        env.storage().persistent().set(&key, &vk);
        env.storage()
            .persistent()
            .extend_ttl(&key, PERSISTENT_THRESHOLD, PERSISTENT_TTL);
        env.storage()
            .instance()
            .extend_ttl(PERSISTENT_THRESHOLD, PERSISTENT_TTL);
        circuit_id
    }

    /// Verify a proof against a registered circuit
    pub fn verify(
        env: Env,
        circuit_id: BytesN<32>,
        proof: Proof,
        public_inputs: Vec<Fr>,
    ) -> bool {
        let key = DataKey::Circuit(circuit_id.clone());
        let vk: VerificationKey = env
            .storage()
            .persistent()
            .get(&key)
            .expect("circuit not registered");
        env.storage()
            .persistent()
            .extend_ttl(&key, PERSISTENT_THRESHOLD, PERSISTENT_TTL);
        env.storage()
            .instance()
            .extend_ttl(PERSISTENT_THRESHOLD, PERSISTENT_TTL);
        let result = verify_groth16(&env, &vk, &proof, &public_inputs);
        if result {
            env.events().publish(("verify",), VerifyEvent { circuit_id });
        }
        result
    }

    /// Get stored verification key for a circuit
    pub fn get_vk(env: Env, circuit_id: BytesN<32>) -> VerificationKey {
        let key = DataKey::Circuit(circuit_id);
        env.storage()
            .persistent()
            .get(&key)
            .expect("circuit not registered")
    }

    /// Check if a circuit is registered
    pub fn is_registered(env: Env, circuit_id: BytesN<32>) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::Circuit(circuit_id))
    }

    /// Compute circuit_id = sha256(alpha_g1 ++ beta_g2 ++ gamma_g2 ++ delta_g2 ++ ic[0..n])
    fn compute_circuit_id(env: &Env, vk: &VerificationKey) -> BytesN<32> {
        let mut buf = Bytes::new(env);
        buf.extend_from_array(&vk.alpha_g1.to_bytes().to_array());
        buf.extend_from_array(&vk.beta_g2.to_bytes().to_array());
        buf.extend_from_array(&vk.gamma_g2.to_bytes().to_array());
        buf.extend_from_array(&vk.delta_g2.to_bytes().to_array());
        for i in 0..vk.ic.len() {
            let pt: soroban_sdk::crypto::bls12_381::G1Affine = vk.ic.get(i).unwrap();
            buf.extend_from_array(&pt.to_bytes().to_array());
        }
        env.crypto().sha256(&buf).into()
    }
}
