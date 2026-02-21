// Copyright 2026 abhirupbanerjee
// Licensed under the Apache License, Version 2.0

//! Integration test: proves r14-sdk is usable as an external dependency.
//! All imports go through `r14_sdk::` — no internal crate paths.

use ark_ff::UniformRand;
use ark_std::rand::{rngs::StdRng, SeedableRng};

fn rng() -> StdRng {
    StdRng::seed_from_u64(123)
}

// ── re-exports from r14-types / r14-poseidon ──

#[test]
fn reexported_types_accessible() {
    let mut rng = rng();
    let sk = r14_sdk::SecretKey::random(&mut rng);
    let owner = r14_sdk::owner_hash(&sk);

    let note = r14_sdk::Note::new(100, 1, owner.0, &mut rng);
    let cm = r14_sdk::commitment(&note);

    // commitment is a field element, should survive hex roundtrip
    let hex = r14_sdk::wallet::fr_to_hex(&cm);
    let recovered = r14_sdk::wallet::hex_to_fr(&hex).unwrap();
    assert_eq!(cm, recovered);
}

#[test]
fn reexported_hash2() {
    let a = ark_bls12_381::Fr::from(1u64);
    let b = ark_bls12_381::Fr::from(2u64);
    let h = r14_sdk::hash2(a, b);
    // deterministic
    assert_eq!(h, r14_sdk::hash2(a, b));
    // non-trivial
    assert_ne!(h, a);
    assert_ne!(h, b);
}

#[test]
fn merkle_depth_constant() {
    assert!(r14_sdk::MERKLE_DEPTH > 0);
}

// ── wallet module ──

#[test]
fn wallet_types_constructible() {
    let wallet = r14_sdk::wallet::WalletData {
        secret_key: "0xdead".into(),
        owner_hash: "0xbeef".into(),
        stellar_secret: "S_TEST".into(),
        notes: vec![r14_sdk::wallet::NoteEntry {
            value: 500,
            app_tag: 1,
            owner: "0xaa".into(),
            nonce: "0xbb".into(),
            commitment: "0xcc".into(),
            index: Some(0),
            spent: false,
        }],
        indexer_url: "http://localhost:3000".into(),
        rpc_url: "https://example.com".into(),
        core_contract_id: "C_CORE".into(),
        transfer_contract_id: "C_TRANSFER".into(),
    };
    assert_eq!(wallet.notes.len(), 1);
    assert_eq!(wallet.notes[0].value, 500);
}

#[test]
fn wallet_path_resolves() {
    // should not panic — just returns a PathBuf
    let path = r14_sdk::wallet::wallet_path().unwrap();
    assert!(path.ends_with("wallet.json"));
}

#[test]
fn crypto_rng_works() {
    let mut rng = r14_sdk::wallet::crypto_rng();
    let a = ark_bls12_381::Fr::rand(&mut rng);
    let b = ark_bls12_381::Fr::rand(&mut rng);
    assert_ne!(a, b);
}

// ── merkle module ──

#[test]
fn merkle_empty_root_via_sdk() {
    let hex = r14_sdk::merkle::empty_root_hex();
    assert_eq!(hex.len(), 64);
}

#[test]
fn merkle_compute_root_via_sdk() {
    let mut rng = rng();
    let leaf = ark_bls12_381::Fr::rand(&mut rng);
    let root = r14_sdk::merkle::compute_root_from_leaves(&[leaf]);
    assert_eq!(root.len(), 64);
    assert_ne!(root, r14_sdk::merkle::empty_root_hex());
}

// ── serialize module ──

#[test]
fn serialize_fr_via_sdk() {
    let fr = ark_bls12_381::Fr::from(42u64);
    let hex = r14_sdk::serialize::serialize_fr(&fr);
    assert_eq!(hex.len(), 64);
}

// ── full workflow: keygen → note → commitment → merkle root ──

#[test]
fn end_to_end_note_to_root() {
    let mut rng = rng();

    // keygen
    let sk = r14_sdk::SecretKey::random(&mut rng);
    let owner = r14_sdk::owner_hash(&sk);

    // create two notes
    let note_a = r14_sdk::Note::new(1000, 1, owner.0, &mut rng);
    let note_b = r14_sdk::Note::new(500, 1, owner.0, &mut rng);
    let cm_a = r14_sdk::commitment(&note_a);
    let cm_b = r14_sdk::commitment(&note_b);

    // compute merkle root over both commitments
    let root = r14_sdk::merkle::compute_root_from_leaves(&[cm_a, cm_b]);
    assert_eq!(root.len(), 64);

    // nullifier derivable
    let nul = r14_sdk::nullifier(&sk, &note_a.nonce);
    let nul_hex = r14_sdk::wallet::fr_to_hex(&nul.0);
    assert!(nul_hex.starts_with("0x"));
}
