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
    /// Private witness
    x: Option<Fr>,
    /// Public input
    y: Fr,
}


impl DummyCircuit {
    
}

impl ConstraintSynthesizer<Fr> for DummyCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        use ark_relations::r1cs::Variable;

        // Allocate public input y
        let y_var = cs.new_input_variable(|| Ok(self.y))?;

        // Allocate private witness x
        let x_var = cs.new_witness_variable(|| {
            self.x.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Allocate intermediate variable x_squared
        let x_squared_var = cs.new_witness_variable(|| {
            let x = self.x.ok_or(SynthesisError::AssignmentMissing)?;
            Ok(x * x)
        })?;

        // Constraint 1: x_squared = x * x
        cs.enforce_constraint(
            lc!() + x_var,
            lc!() + x_var,
            lc!() + x_squared_var,
        )?;

        // Constraint 2: y = x_squared + 5
        // In R1CS: A * B = C, we use: 1 * (x_squared + 5) = y
        // Rearranged: x_squared * 1 = y - 5
        let five = Fr::from(5u64);
        cs.enforce_constraint(
            lc!() + x_squared_var + (five, Variable::One),
            lc!() + Variable::One,
            lc!() + y_var,
        )?;

        Ok(())
    }
}

/// Serialize G1 point to hex string (uncompressed format for Soroban)
fn serialize_g1(point: &G1Affine) -> String {
    let mut bytes = Vec::new();
    point.serialize_uncompressed(&mut bytes).unwrap();
    assert_eq!(bytes.len(), 96, "G1 uncompressed should be 96 bytes");
    hex::encode(&bytes)
}

/// Serialize G2 point to hex string (uncompressed format for Soroban)
fn serialize_g2(point: &G2Affine) -> String {
    let mut bytes = Vec::new();
    point.serialize_uncompressed(&mut bytes).unwrap();
    assert_eq!(bytes.len(), 192, "G2 uncompressed should be 192 bytes");
    hex::encode(&bytes)
}

/// Serialize Fr field element to hex string (big-endian for Soroban's Fr::from_bytes)
fn serialize_fr(fr: &Fr) -> String {
    let mut bytes = Vec::new();
    fr.serialize_compressed(&mut bytes).unwrap();
    // arkworks Fr uses generic LE serialization, Soroban Fr::from_bytes uses U256::from_be_bytes
    bytes.reverse();
    hex::encode(&bytes)
}

#[test]
fn generate_test_vectors() {
    let mut rng = thread_rng();

    println!("\n=== Phase 0: Generating Groth16 Test Vectors ===\n");

    // Circuit: y = x² + 5 with x=3, y=14
    let y = Fr::from(14u64);
    let x = Fr::from(3u64);

    // Verify circuit logic off-chain
    let x_squared = x * x;
    assert_eq!(x_squared, Fr::from(9u64));
    assert_eq!(x_squared + Fr::from(5u64), y);

    println!("Circuit: y = x² + 5");
    println!("Private witness: x = 3");
    println!("Public input: y = 14");
    println!("Verification: 3² + 5 = 9 + 5 = 14 ✓\n");

    // Setup phase (with x=None for circuit generation)
    println!("Running trusted setup...");
    let setup_circuit = DummyCircuit { x: None, y };
    let (pk, vk) = Groth16::<Bls12_381>::circuit_specific_setup(setup_circuit, &mut rng)
        .expect("Setup failed");
    println!("Setup complete\n");

    // Prove phase (with actual witness x=3)
    println!("Generating proof...");
    let prove_circuit = DummyCircuit { x: Some(x), y };
    let proof = Groth16::<Bls12_381>::prove(&pk, prove_circuit, &mut rng)
        .expect("Proving failed");
    println!("Proof generated\n");

    // Verify off-chain to ensure proof is valid
    println!("Verifying proof off-chain...");
    let public_inputs = vec![y];
    let valid = Groth16::<Bls12_381>::verify(&vk, &public_inputs, &proof)
        .expect("Verification failed");
    assert!(valid, "Proof verification failed!");
    println!("✓ Proof verified successfully off-chain\n");

    // Print test vectors for hardcoding
    println!("=== Test Vectors (paste into test_vectors.rs) ===\n");

    println!("// Verification Key");
    println!("pub const VK_ALPHA_G1: &str = \"{}\";", serialize_g1(&vk.alpha_g1));
    println!("pub const VK_BETA_G2: &str = \"{}\";", serialize_g2(&vk.beta_g2));
    println!("pub const VK_GAMMA_G2: &str = \"{}\";", serialize_g2(&vk.gamma_g2));
    println!("pub const VK_DELTA_G2: &str = \"{}\";", serialize_g2(&vk.delta_g2));
    println!("pub const VK_IC_0: &str = \"{}\";", serialize_g1(&vk.gamma_abc_g1[0]));
    println!("pub const VK_IC_1: &str = \"{}\";", serialize_g1(&vk.gamma_abc_g1[1]));
    println!();

    println!("// Proof");
    println!("pub const PROOF_A: &str = \"{}\";", serialize_g1(&proof.a));
    println!("pub const PROOF_B: &str = \"{}\";", serialize_g2(&proof.b));
    println!("pub const PROOF_C: &str = \"{}\";", serialize_g1(&proof.c));
    println!();

    println!("// Public Input");
    println!("pub const PUBLIC_INPUT: &str = \"{}\";", serialize_fr(&y));
    println!();

    println!("=== Verification Key Structure ===");
    println!("IC length: {} (matches 1 public input + 1)", vk.gamma_abc_g1.len());
    println!();

    println!("✓ Test vectors generated successfully");
    println!("✓ Copy the above constants to crates/r14-kernel/src/test_vectors.rs");
}
