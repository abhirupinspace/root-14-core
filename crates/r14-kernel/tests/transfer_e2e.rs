// Copyright 2026 abhirupbanerjee
// Licensed under the Apache License, Version 2.0

//! End-to-end integration test: off-chain prove → on-chain verify

use r14_circuit::{
    serialize_proof_for_soroban, serialize_vk_for_soroban, SerializedProof, SerializedVK,
};
use r14_kernel::{Proof, R14Kernel, R14KernelClient, VerificationKey};
use soroban_sdk::crypto::bls12_381::{G1Affine, G2Affine};
use soroban_sdk::{BytesN, Env, Vec};

// ── Hex helpers (test-side, uses `hex` crate with std) ──

fn hex_to_g1(env: &Env, h: &str) -> G1Affine {
    let bytes: [u8; 96] = hex::decode(h).unwrap().try_into().unwrap();
    G1Affine::from_bytes(BytesN::from_array(env, &bytes))
}

fn hex_to_g2(env: &Env, h: &str) -> G2Affine {
    let bytes: [u8; 192] = hex::decode(h).unwrap().try_into().unwrap();
    G2Affine::from_bytes(BytesN::from_array(env, &bytes))
}

fn hex_to_bytes32(env: &Env, h: &str) -> BytesN<32> {
    let bytes: [u8; 32] = hex::decode(h).unwrap().try_into().unwrap();
    BytesN::from_array(env, &bytes)
}

// ── Soroban type builders ──

fn build_soroban_vk(env: &Env, svk: &SerializedVK) -> VerificationKey {
    VerificationKey {
        alpha_g1: hex_to_g1(env, &svk.alpha_g1),
        beta_g2: hex_to_g2(env, &svk.beta_g2),
        gamma_g2: hex_to_g2(env, &svk.gamma_g2),
        delta_g2: hex_to_g2(env, &svk.delta_g2),
        ic_0: hex_to_g1(env, &svk.ic[0]),
        ic_rest: {
            let mut v = Vec::new(env);
            for ic_hex in &svk.ic[1..] {
                v.push_back(hex_to_g1(env, ic_hex));
            }
            v
        },
    }
}

fn build_soroban_proof(env: &Env, sp: &SerializedProof) -> Proof {
    Proof {
        a: hex_to_g1(env, &sp.a),
        b: hex_to_g2(env, &sp.b),
        c: hex_to_g1(env, &sp.c),
    }
}

// ── Test scenario (mirrors r14-circuit test pattern) ──

use ark_bls12_381::Fr;
use ark_ff::UniformRand;
use ark_std::rand::{rngs::StdRng, SeedableRng};
use r14_types::{MerklePath, Note, SecretKey, MERKLE_DEPTH};

fn test_rng() -> StdRng {
    StdRng::seed_from_u64(42)
}

fn build_dummy_merkle_path(rng: &mut impl ark_std::rand::RngCore) -> MerklePath {
    let siblings: std::vec::Vec<Fr> = (0..MERKLE_DEPTH).map(|_| Fr::rand(rng)).collect();
    let indices: std::vec::Vec<bool> = (0..MERKLE_DEPTH).map(|i| i % 2 == 0).collect();
    MerklePath { siblings, indices }
}

struct TestScenario {
    proof: SerializedProof,
    public_inputs: std::vec::Vec<String>,
    svk: SerializedVK,
}

fn setup_and_prove() -> TestScenario {
    let mut rng = test_rng();

    let sk = SecretKey::random(&mut rng);
    let owner = r14_poseidon::owner_hash(&sk);
    let consumed = Note::new(1000, 1, owner.0, &mut rng);
    let path = build_dummy_merkle_path(&mut rng);

    let recipient_sk = SecretKey::random(&mut rng);
    let recipient_owner = r14_poseidon::owner_hash(&recipient_sk);
    let note_0 = Note::new(700, 1, recipient_owner.0, &mut rng);
    let note_1 = Note::new(300, 1, owner.0, &mut rng);

    let (pk, vk) = r14_circuit::setup(&mut rng);
    let (proof, pi) = r14_circuit::prove(&pk, sk.0, consumed, path, [note_0, note_1], &mut rng);

    // Verify off-chain first (sanity)
    assert!(r14_circuit::verify_offchain(&vk, &proof, &pi));

    let svk = serialize_vk_for_soroban(&vk);
    let (sp, spi) = serialize_proof_for_soroban(&proof, &pi);

    TestScenario {
        proof: sp,
        public_inputs: spi,
        svk,
    }
}

// ── Tests ──

#[test]
fn test_transfer_e2e() {
    let scenario = setup_and_prove();
    let env = Env::default();

    let contract_id = env.register(R14Kernel, ());
    let client = R14KernelClient::new(&env, &contract_id);

    let vk = build_soroban_vk(&env, &scenario.svk);
    let proof = build_soroban_proof(&env, &scenario.proof);
    let old_root = hex_to_bytes32(&env, &scenario.public_inputs[0]);
    let nullifier = hex_to_bytes32(&env, &scenario.public_inputs[1]);
    let cm_0 = hex_to_bytes32(&env, &scenario.public_inputs[2]);
    let cm_1 = hex_to_bytes32(&env, &scenario.public_inputs[3]);

    client.init(&vk);
    let result = client.transfer(&proof, &old_root, &nullifier, &cm_0, &cm_1);
    assert!(result);
}

#[test]
#[should_panic(expected = "nullifier already spent")]
fn test_double_spend_rejected() {
    let scenario = setup_and_prove();
    let env = Env::default();

    let contract_id = env.register(R14Kernel, ());
    let client = R14KernelClient::new(&env, &contract_id);

    let vk = build_soroban_vk(&env, &scenario.svk);
    let proof = build_soroban_proof(&env, &scenario.proof);
    let old_root = hex_to_bytes32(&env, &scenario.public_inputs[0]);
    let nullifier = hex_to_bytes32(&env, &scenario.public_inputs[1]);
    let cm_0 = hex_to_bytes32(&env, &scenario.public_inputs[2]);
    let cm_1 = hex_to_bytes32(&env, &scenario.public_inputs[3]);

    client.init(&vk);
    client.transfer(&proof, &old_root, &nullifier, &cm_0, &cm_1);
    // Second call with same nullifier should panic
    client.transfer(&proof, &old_root, &nullifier, &cm_0, &cm_1);
}

#[test]
#[should_panic(expected = "proof verification failed")]
fn test_invalid_proof_rejected() {
    let scenario = setup_and_prove();
    let env = Env::default();

    let contract_id = env.register(R14Kernel, ());
    let client = R14KernelClient::new(&env, &contract_id);

    let vk = build_soroban_vk(&env, &scenario.svk);
    let old_root = hex_to_bytes32(&env, &scenario.public_inputs[0]);
    let nullifier = hex_to_bytes32(&env, &scenario.public_inputs[1]);
    let cm_0 = hex_to_bytes32(&env, &scenario.public_inputs[2]);
    let cm_1 = hex_to_bytes32(&env, &scenario.public_inputs[3]);

    // Tamper proof: swap proof.a with IC[0] from VK
    let tampered_proof = Proof {
        a: hex_to_g1(&env, &scenario.svk.ic[0]),
        b: hex_to_g2(&env, &scenario.proof.b),
        c: hex_to_g1(&env, &scenario.proof.c),
    };

    client.init(&vk);
    client.transfer(&tampered_proof, &old_root, &nullifier, &cm_0, &cm_1);
}

#[test]
#[should_panic(expected = "proof verification failed")]
fn test_wrong_nullifier_rejected() {
    let scenario = setup_and_prove();
    let env = Env::default();

    let contract_id = env.register(R14Kernel, ());
    let client = R14KernelClient::new(&env, &contract_id);

    let vk = build_soroban_vk(&env, &scenario.svk);
    let proof = build_soroban_proof(&env, &scenario.proof);
    let old_root = hex_to_bytes32(&env, &scenario.public_inputs[0]);
    let cm_0 = hex_to_bytes32(&env, &scenario.public_inputs[2]);
    let cm_1 = hex_to_bytes32(&env, &scenario.public_inputs[3]);

    // Random nullifier that doesn't match the proof
    let wrong_nullifier = BytesN::from_array(&env, &[0xABu8; 32]);

    client.init(&vk);
    client.transfer(&proof, &old_root, &wrong_nullifier, &cm_0, &cm_1);
}
