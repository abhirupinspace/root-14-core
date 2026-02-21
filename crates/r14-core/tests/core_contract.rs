// Copyright 2026 abhirupbanerjee
// Licensed under the Apache License, Version 2.0

//! Unit tests for r14-core contract: register, verify, get_vk, is_registered

use r14_core::{Proof, R14Core, R14CoreClient, VerificationKey};
use r14_sdk::{serialize_proof_for_soroban, serialize_vk_for_soroban, SerializedProof, SerializedVK};
use soroban_sdk::crypto::bls12_381::{Fr, G1Affine, G2Affine};
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, Vec};

// ── Hex helpers ──

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

// ── Build Soroban types from serialized (unified IC) ──

fn build_soroban_vk(env: &Env, svk: &SerializedVK) -> VerificationKey {
    let mut ic = Vec::new(env);
    for ic_hex in &svk.ic {
        ic.push_back(hex_to_g1(env, ic_hex));
    }
    VerificationKey {
        alpha_g1: hex_to_g1(env, &svk.alpha_g1),
        beta_g2: hex_to_g2(env, &svk.beta_g2),
        gamma_g2: hex_to_g2(env, &svk.gamma_g2),
        delta_g2: hex_to_g2(env, &svk.delta_g2),
        ic,
    }
}

fn build_soroban_proof(env: &Env, sp: &SerializedProof) -> Proof {
    Proof {
        a: hex_to_g1(env, &sp.a),
        b: hex_to_g2(env, &sp.b),
        c: hex_to_g1(env, &sp.c),
    }
}

// ── Test scenario: transfer circuit ──

use ark_bls12_381::Fr as ArkFr;
use ark_ff::UniformRand;
use ark_std::rand::{rngs::StdRng, SeedableRng};
use r14_types::{MerklePath, Note, SecretKey, MERKLE_DEPTH};

fn test_rng() -> StdRng {
    StdRng::seed_from_u64(42)
}

fn build_dummy_merkle_path(rng: &mut impl ark_std::rand::RngCore) -> MerklePath {
    let siblings: std::vec::Vec<ArkFr> = (0..MERKLE_DEPTH).map(|_| ArkFr::rand(rng)).collect();
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

    assert!(r14_circuit::verify_offchain(&vk, &proof, &pi));

    let svk = serialize_vk_for_soroban(&vk);
    let (sp, spi) = serialize_proof_for_soroban(&proof, &pi.to_vec());

    TestScenario {
        proof: sp,
        public_inputs: spi,
        svk,
    }
}

// ── Tests ──

#[test]
fn register_and_verify() {
    let scenario = setup_and_prove();
    let env = Env::default();
    let admin = Address::generate(&env);

    let core_id = env.register(R14Core, ());
    let client = R14CoreClient::new(&env, &core_id);
    client.init(&admin);

    let vk = build_soroban_vk(&env, &scenario.svk);
    env.mock_all_auths();
    let circuit_id = client.register(&admin, &vk);

    let proof = build_soroban_proof(&env, &scenario.proof);
    let inputs: Vec<Fr> = Vec::from_array(
        &env,
        [
            Fr::from_bytes(hex_to_bytes32(&env, &scenario.public_inputs[0])),
            Fr::from_bytes(hex_to_bytes32(&env, &scenario.public_inputs[1])),
            Fr::from_bytes(hex_to_bytes32(&env, &scenario.public_inputs[2])),
            Fr::from_bytes(hex_to_bytes32(&env, &scenario.public_inputs[3])),
        ],
    );

    assert!(client.verify(&circuit_id, &proof, &inputs));
}

#[test]
fn verify_wrong_input() {
    let scenario = setup_and_prove();
    let env = Env::default();
    let admin = Address::generate(&env);

    let core_id = env.register(R14Core, ());
    let client = R14CoreClient::new(&env, &core_id);
    client.init(&admin);

    let vk = build_soroban_vk(&env, &scenario.svk);
    env.mock_all_auths();
    let circuit_id = client.register(&admin, &vk);

    let proof = build_soroban_proof(&env, &scenario.proof);
    // Wrong inputs — all zeros
    let wrong_inputs: Vec<Fr> = Vec::from_array(
        &env,
        [
            Fr::from_bytes(BytesN::from_array(&env, &[0u8; 32])),
            Fr::from_bytes(BytesN::from_array(&env, &[0u8; 32])),
            Fr::from_bytes(BytesN::from_array(&env, &[0u8; 32])),
            Fr::from_bytes(BytesN::from_array(&env, &[0u8; 32])),
        ],
    );

    assert!(!client.verify(&circuit_id, &proof, &wrong_inputs));
}

#[test]
#[should_panic(expected = "circuit not registered")]
fn unregistered_circuit_panics() {
    let scenario = setup_and_prove();
    let env = Env::default();
    let admin = Address::generate(&env);

    let core_id = env.register(R14Core, ());
    let client = R14CoreClient::new(&env, &core_id);
    client.init(&admin);

    let proof = build_soroban_proof(&env, &scenario.proof);
    let fake_circuit_id = BytesN::from_array(&env, &[0xFFu8; 32]);
    let inputs: Vec<Fr> = Vec::from_array(
        &env,
        [Fr::from_bytes(BytesN::from_array(&env, &[0u8; 32]))],
    );

    client.verify(&fake_circuit_id, &proof, &inputs);
}

#[test]
#[should_panic(expected = "circuit already registered")]
fn duplicate_register_panics() {
    let scenario = setup_and_prove();
    let env = Env::default();
    let admin = Address::generate(&env);

    let core_id = env.register(R14Core, ());
    let client = R14CoreClient::new(&env, &core_id);
    client.init(&admin);

    let vk = build_soroban_vk(&env, &scenario.svk);
    env.mock_all_auths();
    client.register(&admin, &vk);
    // Duplicate should panic
    client.register(&admin, &vk);
}

#[test]
#[should_panic]
fn non_admin_register_panics() {
    let scenario = setup_and_prove();
    let env = Env::default();
    let admin = Address::generate(&env);
    let imposter = Address::generate(&env);

    let core_id = env.register(R14Core, ());
    let client = R14CoreClient::new(&env, &core_id);
    client.init(&admin);

    let vk = build_soroban_vk(&env, &scenario.svk);
    // Only mock auth for imposter, not admin
    env.mock_all_auths();
    client.register(&imposter, &vk);
}

#[test]
fn is_registered_true_false() {
    let scenario = setup_and_prove();
    let env = Env::default();
    let admin = Address::generate(&env);

    let core_id = env.register(R14Core, ());
    let client = R14CoreClient::new(&env, &core_id);
    client.init(&admin);

    let fake_id = BytesN::from_array(&env, &[0xFFu8; 32]);
    assert!(!client.is_registered(&fake_id));

    let vk = build_soroban_vk(&env, &scenario.svk);
    env.mock_all_auths();
    let circuit_id = client.register(&admin, &vk);
    assert!(client.is_registered(&circuit_id));
}

#[test]
fn get_vk_returns_stored() {
    let scenario = setup_and_prove();
    let env = Env::default();
    let admin = Address::generate(&env);

    let core_id = env.register(R14Core, ());
    let client = R14CoreClient::new(&env, &core_id);
    client.init(&admin);

    let vk = build_soroban_vk(&env, &scenario.svk);
    env.mock_all_auths();
    let circuit_id = client.register(&admin, &vk);

    let stored_vk = client.get_vk(&circuit_id);
    // Compare alpha_g1 as a spot check
    assert_eq!(stored_vk.alpha_g1.to_bytes(), vk.alpha_g1.to_bytes());
    assert_eq!(stored_vk.ic.len(), vk.ic.len());
}
