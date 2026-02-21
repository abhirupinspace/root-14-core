// Copyright 2026 abhirupbanerjee
// Licensed under the Apache License, Version 2.0

//! Off-chain proof generator for Phase 0 feasibility spike
//! Circuit: y = x² + 5
//! Public input: y = 14
//! Private witness: x = 3

use ark_bls12_381::{Bls12_381, Fr, G1Affine, G2Affine};
use ark_groth16::Groth16;
use ark_relations::{
    lc,
    r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError},
};
use ark_serialize::CanonicalSerialize;
use ark_snark::SNARK;
use ark_std::rand::thread_rng;

/// Dummy circuit: y = x² + 5
#[derive(Clone)]
struct DummyCircuit {
    x: Option<Fr>,
    y: Fr,
}

impl ConstraintSynthesizer<Fr> for DummyCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        use ark_relations::r1cs::Variable;

        let y_var = cs.new_input_variable(|| Ok(self.y))?;
        let x_var = cs.new_witness_variable(|| {
            self.x.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let x_squared_var = cs.new_witness_variable(|| {
            let x = self.x.ok_or(SynthesisError::AssignmentMissing)?;
            Ok(x * x)
        })?;

        cs.enforce_constraint(
            lc!() + x_var,
            lc!() + x_var,
            lc!() + x_squared_var,
        )?;

        let five = Fr::from(5u64);
        cs.enforce_constraint(
            lc!() + x_squared_var + (five, Variable::One),
            lc!() + Variable::One,
            lc!() + y_var,
        )?;

        Ok(())
    }
}

fn serialize_g1(point: &G1Affine) -> String {
    let mut bytes = Vec::new();
    point.serialize_uncompressed(&mut bytes).unwrap();
    assert_eq!(bytes.len(), 96, "G1 uncompressed should be 96 bytes");
    hex::encode(&bytes)
}

fn serialize_g2(point: &G2Affine) -> String {
    let mut bytes = Vec::new();
    point.serialize_uncompressed(&mut bytes).unwrap();
    assert_eq!(bytes.len(), 192, "G2 uncompressed should be 192 bytes");
    hex::encode(&bytes)
}

fn serialize_fr(fr: &Fr) -> String {
    let mut bytes = Vec::new();
    fr.serialize_compressed(&mut bytes).unwrap();
    bytes.reverse();
    hex::encode(&bytes)
}

#[test]
fn generate_test_vectors() {
    let mut rng = thread_rng();

    let y = Fr::from(14u64);
    let x = Fr::from(3u64);

    let x_squared = x * x;
    assert_eq!(x_squared, Fr::from(9u64));
    assert_eq!(x_squared + Fr::from(5u64), y);

    let setup_circuit = DummyCircuit { x: None, y };
    let (pk, vk) = Groth16::<Bls12_381>::circuit_specific_setup(setup_circuit, &mut rng)
        .expect("Setup failed");

    let prove_circuit = DummyCircuit { x: Some(x), y };
    let proof = Groth16::<Bls12_381>::prove(&pk, prove_circuit, &mut rng)
        .expect("Proving failed");

    let public_inputs = vec![y];
    let valid = Groth16::<Bls12_381>::verify(&vk, &public_inputs, &proof)
        .expect("Verification failed");
    assert!(valid, "Proof verification failed!");

    println!("=== Test Vectors ===");
    println!("VK_ALPHA_G1: {}", serialize_g1(&vk.alpha_g1));
    println!("VK_IC length: {}", vk.gamma_abc_g1.len());
    println!("PUBLIC_INPUT: {}", serialize_fr(&y));
}
