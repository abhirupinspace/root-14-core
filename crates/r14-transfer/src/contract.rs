// Copyright 2026 abhirupbanerjee
// Licensed under the Apache License, Version 2.0

//! Private transfer contract — delegates proof verification to r14-core

use soroban_sdk::crypto::bls12_381::{Fr, G1Affine, G2Affine};
use soroban_sdk::{contract, contractimpl, contracttype, Address, BytesN, Env, IntoVal, Symbol, Vec};

/// Groth16 proof (same layout as r14-core::Proof — identical XDR encoding)
#[contracttype]
#[derive(Clone, Debug)]
pub struct Proof {
    pub a: G1Affine,
    pub b: G2Affine,
    pub c: G1Affine,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct DepositEvent {
    pub cm: BytesN<32>,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct TransferEvent {
    pub nullifier: BytesN<32>,
    pub cm_0: BytesN<32>,
    pub cm_1: BytesN<32>,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    CoreContract,
    CircuitId,
    Nullifier(BytesN<32>),
    Root(BytesN<32>),
    RootIndex,
    RootAt(u32),
}

const PERSISTENT_TTL: u32 = 535_680; // ~30 days
const PERSISTENT_THRESHOLD: u32 = 267_840; // ~15 days
const ROOT_HISTORY_SIZE: u32 = 100;

#[contract]
pub struct R14Transfer;

#[contractimpl]
impl R14Transfer {
    /// Initialize with core contract address, circuit_id, and empty tree root
    pub fn init(env: Env, core_contract: Address, circuit_id: BytesN<32>, empty_root: BytesN<32>) {
        if env.storage().instance().has(&DataKey::CoreContract) {
            panic!("already initialized");
        }
        env.storage()
            .instance()
            .set(&DataKey::CoreContract, &core_contract);
        env.storage()
            .instance()
            .set(&DataKey::CircuitId, &circuit_id);
        env.storage()
            .instance()
            .extend_ttl(PERSISTENT_THRESHOLD, PERSISTENT_TTL);
        Self::commit_root(&env, empty_root);
    }

    /// Deposit a commitment (emits event for indexer)
    pub fn deposit(env: Env, cm: BytesN<32>, new_root: BytesN<32>) {
        if cm == BytesN::from_array(&env, &[0u8; 32]) {
            panic!("zero commitment");
        }
        Self::commit_root(&env, new_root);
        env.events().publish(("deposit",), DepositEvent { cm });
    }

    /// Verify a private transfer and mark nullifier as spent
    pub fn transfer(
        env: Env,
        proof: Proof,
        old_root: BytesN<32>,
        nullifier: BytesN<32>,
        cm_0: BytesN<32>,
        cm_1: BytesN<32>,
        new_root: BytesN<32>,
    ) -> bool {
        // Validate old_root is known
        if !env
            .storage()
            .persistent()
            .has(&DataKey::Root(old_root.clone()))
        {
            panic!("unknown merkle root");
        }

        // Check nullifier not already spent
        let nf_key = DataKey::Nullifier(nullifier.clone());
        if env.storage().persistent().has(&nf_key) {
            panic!("nullifier already spent");
        }

        // Build public inputs
        let old_root_fr = Fr::from_bytes(old_root);
        let nullifier_fr = Fr::from_bytes(nullifier.clone());
        let cm_0_fr = Fr::from_bytes(cm_0.clone());
        let cm_1_fr = Fr::from_bytes(cm_1.clone());

        let public_inputs: Vec<Fr> =
            Vec::from_array(&env, [old_root_fr, nullifier_fr, cm_0_fr, cm_1_fr]);

        // Cross-contract call to r14-core via env.invoke_contract
        let core_addr: Address = env
            .storage()
            .instance()
            .get(&DataKey::CoreContract)
            .expect("not initialized");
        let circuit_id: BytesN<32> = env
            .storage()
            .instance()
            .get(&DataKey::CircuitId)
            .expect("not initialized");

        let args: Vec<soroban_sdk::Val> = (circuit_id, proof, public_inputs).into_val(&env);
        let verified: bool =
            env.invoke_contract(&core_addr, &Symbol::new(&env, "verify"), args);

        if !verified {
            panic!("proof verification failed");
        }

        // Mark nullifier as spent
        env.storage().persistent().set(&nf_key, &true);
        env.storage()
            .persistent()
            .extend_ttl(&nf_key, PERSISTENT_THRESHOLD, PERSISTENT_TTL);
        env.storage()
            .instance()
            .extend_ttl(PERSISTENT_THRESHOLD, PERSISTENT_TTL);

        // Store new merkle root
        Self::commit_root(&env, new_root);

        // Emit event
        env.events()
            .publish(("transfer",), TransferEvent { nullifier, cm_0, cm_1 });

        true
    }

    /// Store a root in the circular buffer
    fn commit_root(env: &Env, root: BytesN<32>) {
        let idx: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::RootIndex)
            .unwrap_or(0);

        // Remove old root at this buffer slot if it exists
        let slot_key = DataKey::RootAt(idx);
        if let Some(old_root) = env
            .storage()
            .persistent()
            .get::<_, BytesN<32>>(&slot_key)
        {
            env.storage()
                .persistent()
                .remove(&DataKey::Root(old_root));
        }

        // Store new root
        let root_key = DataKey::Root(root.clone());
        env.storage().persistent().set(&root_key, &true);
        env.storage()
            .persistent()
            .extend_ttl(&root_key, PERSISTENT_THRESHOLD, PERSISTENT_TTL);

        // Store in buffer slot
        env.storage().persistent().set(&slot_key, &root);
        env.storage()
            .persistent()
            .extend_ttl(&slot_key, PERSISTENT_THRESHOLD, PERSISTENT_TTL);

        // Advance index
        let next_idx = (idx + 1) % ROOT_HISTORY_SIZE;
        env.storage()
            .persistent()
            .set(&DataKey::RootIndex, &next_idx);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::RootIndex, PERSISTENT_THRESHOLD, PERSISTENT_TTL);
    }
}
