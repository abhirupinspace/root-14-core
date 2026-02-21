// Copyright 2026 abhirupbanerjee
// Licensed under the Apache License, Version 2.0

//! End-to-end integration test: off-chain prove → two-contract verify
//! Deploy r14-core + r14-transfer, register VK, then transfer

use r14_core::{R14Core, R14CoreClient, VerificationKey};
use r14_sdk::serialize::{serialize_proof_for_soroban, serialize_vk_for_soroban, SerializedProof, SerializedVK};
use r14_transfer::{Proof, R14Transfer, R14TransferClient};
use soroban_sdk::crypto::bls12_381::{G1Affine, G2Affine};
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

// ── Soroban type builders (unified IC) ──

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

// ── Test scenario ──

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

    assert!(r14_circuit::verify_offchain(&vk, &proof, &pi));

    let svk = serialize_vk_for_soroban(&vk);
    let (sp, spi) = serialize_proof_for_soroban(&proof, &pi.to_vec());

    TestScenario {
        proof: sp,
        public_inputs: spi,
        svk,
    }
}

/// Dummy empty root for tests (just 32 zero bytes — not a real Poseidon empty root)
fn test_empty_root(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[0xEEu8; 32])
}

/// Dummy new root for tests
fn test_new_root(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[0xAAu8; 32])
}

/// Deploy r14-core + r14-transfer, register VK, return transfer contract address.
/// Seeds the old_root from scenario into the root history via a deposit.
fn deploy_contracts(env: &Env, svk: &SerializedVK, old_root: &BytesN<32>) -> Address {
    let admin = Address::generate(env);

    // Deploy r14-core
    let core_id = env.register(R14Core, ());
    let core_client = R14CoreClient::new(env, &core_id);
    core_client.init(&admin);

    // Register transfer VK
    let vk = build_soroban_vk(env, svk);
    env.mock_all_auths();
    let circuit_id = core_client.register(&admin, &vk);

    // Deploy r14-transfer with empty root
    let transfer_id = env.register(R14Transfer, ());
    let transfer_client = R14TransferClient::new(env, &transfer_id);
    let empty_root = test_empty_root(env);
    transfer_client.init(&core_id, &circuit_id, &empty_root);

    // Deposit a dummy commitment to seed old_root into known roots
    let dummy_cm = BytesN::from_array(env, &[0x01u8; 32]);
    transfer_client.deposit(&dummy_cm, old_root);

    transfer_id
}

// ── Tests ──

#[test]
fn test_transfer_e2e() {
    let scenario = setup_and_prove();
    let env = Env::default();

    let old_root = hex_to_bytes32(&env, &scenario.public_inputs[0]);
    let transfer_addr = deploy_contracts(&env, &scenario.svk, &old_root);
    let client = R14TransferClient::new(&env, &transfer_addr);

    let proof = build_soroban_proof(&env, &scenario.proof);
    let nullifier = hex_to_bytes32(&env, &scenario.public_inputs[1]);
    let cm_0 = hex_to_bytes32(&env, &scenario.public_inputs[2]);
    let cm_1 = hex_to_bytes32(&env, &scenario.public_inputs[3]);
    let new_root = test_new_root(&env);

    let result = client.transfer(&proof, &old_root, &nullifier, &cm_0, &cm_1, &new_root);
    assert!(result);
}

#[test]
#[should_panic(expected = "nullifier already spent")]
fn test_double_spend_rejected() {
    let scenario = setup_and_prove();
    let env = Env::default();

    let old_root = hex_to_bytes32(&env, &scenario.public_inputs[0]);
    let transfer_addr = deploy_contracts(&env, &scenario.svk, &old_root);
    let client = R14TransferClient::new(&env, &transfer_addr);

    let proof = build_soroban_proof(&env, &scenario.proof);
    let nullifier = hex_to_bytes32(&env, &scenario.public_inputs[1]);
    let cm_0 = hex_to_bytes32(&env, &scenario.public_inputs[2]);
    let cm_1 = hex_to_bytes32(&env, &scenario.public_inputs[3]);
    let new_root = test_new_root(&env);

    client.transfer(&proof, &old_root, &nullifier, &cm_0, &cm_1, &new_root);
    // Second call with same nullifier should panic
    client.transfer(&proof, &old_root, &nullifier, &cm_0, &cm_1, &new_root);
}

#[test]
#[should_panic(expected = "proof verification failed")]
fn test_invalid_proof_rejected() {
    let scenario = setup_and_prove();
    let env = Env::default();

    let old_root = hex_to_bytes32(&env, &scenario.public_inputs[0]);
    let transfer_addr = deploy_contracts(&env, &scenario.svk, &old_root);
    let client = R14TransferClient::new(&env, &transfer_addr);

    let nullifier = hex_to_bytes32(&env, &scenario.public_inputs[1]);
    let cm_0 = hex_to_bytes32(&env, &scenario.public_inputs[2]);
    let cm_1 = hex_to_bytes32(&env, &scenario.public_inputs[3]);
    let new_root = test_new_root(&env);

    // Tamper proof: swap proof.a with IC[0] from VK
    let tampered_proof = Proof {
        a: hex_to_g1(&env, &scenario.svk.ic[0]),
        b: hex_to_g2(&env, &scenario.proof.b),
        c: hex_to_g1(&env, &scenario.proof.c),
    };

    client.transfer(&tampered_proof, &old_root, &nullifier, &cm_0, &cm_1, &new_root);
}

#[test]
#[should_panic(expected = "proof verification failed")]
fn test_wrong_nullifier_rejected() {
    let scenario = setup_and_prove();
    let env = Env::default();

    let old_root = hex_to_bytes32(&env, &scenario.public_inputs[0]);
    let transfer_addr = deploy_contracts(&env, &scenario.svk, &old_root);
    let client = R14TransferClient::new(&env, &transfer_addr);

    let proof = build_soroban_proof(&env, &scenario.proof);
    let cm_0 = hex_to_bytes32(&env, &scenario.public_inputs[2]);
    let cm_1 = hex_to_bytes32(&env, &scenario.public_inputs[3]);
    let new_root = test_new_root(&env);

    let wrong_nullifier = BytesN::from_array(&env, &[0xABu8; 32]);

    client.transfer(&proof, &old_root, &wrong_nullifier, &cm_0, &cm_1, &new_root);
}

#[test]
#[should_panic(expected = "unknown merkle root")]
fn test_unknown_root_rejected() {
    let scenario = setup_and_prove();
    let env = Env::default();

    let old_root = hex_to_bytes32(&env, &scenario.public_inputs[0]);
    let transfer_addr = deploy_contracts(&env, &scenario.svk, &old_root);
    let client = R14TransferClient::new(&env, &transfer_addr);

    let proof = build_soroban_proof(&env, &scenario.proof);
    let nullifier = hex_to_bytes32(&env, &scenario.public_inputs[1]);
    let cm_0 = hex_to_bytes32(&env, &scenario.public_inputs[2]);
    let cm_1 = hex_to_bytes32(&env, &scenario.public_inputs[3]);
    let new_root = test_new_root(&env);

    // Use a root that was never committed
    let fake_root = BytesN::from_array(&env, &[0xFFu8; 32]);
    client.transfer(&proof, &fake_root, &nullifier, &cm_0, &cm_1, &new_root);
}

#[test]
#[should_panic(expected = "zero commitment")]
fn test_zero_commitment_rejected() {
    let scenario = setup_and_prove();
    let env = Env::default();

    let old_root = hex_to_bytes32(&env, &scenario.public_inputs[0]);
    let transfer_addr = deploy_contracts(&env, &scenario.svk, &old_root);
    let client = R14TransferClient::new(&env, &transfer_addr);

    let zero_cm = BytesN::from_array(&env, &[0u8; 32]);
    let new_root = test_new_root(&env);
    client.deposit(&zero_cm, &new_root);
}
