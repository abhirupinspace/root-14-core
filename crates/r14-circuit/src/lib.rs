pub mod merkle_gadget;
pub mod poseidon_gadget;
pub mod transfer;

use ark_bls12_381::{Bls12_381, Fr};
use ark_groth16::{Groth16, PreparedVerifyingKey, ProvingKey, VerifyingKey};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem};
use ark_snark::SNARK;
use ark_std::rand::{CryptoRng, RngCore};
use r14_types::{MerklePath, Note};

pub use transfer::TransferCircuit;

/// Public inputs for a transfer proof
pub struct PublicInputs {
    pub old_root: Fr,
    pub nullifier: Fr,
    pub out_commitment_0: Fr,
    pub out_commitment_1: Fr,
}

impl PublicInputs {
    pub fn to_vec(&self) -> Vec<Fr> {
        vec![self.old_root, self.nullifier, self.out_commitment_0, self.out_commitment_1]
    }
}

/// Run Groth16 trusted setup for the transfer circuit
pub fn setup<R: RngCore + CryptoRng>(rng: &mut R) -> (ProvingKey<Bls12_381>, VerifyingKey<Bls12_381>) {
    let circuit = TransferCircuit::empty();
    Groth16::<Bls12_381>::circuit_specific_setup(circuit, rng).expect("setup failed")
}

/// Generate a Groth16 proof for a private transfer
pub fn prove<R: RngCore + CryptoRng>(
    pk: &ProvingKey<Bls12_381>,
    secret_key: Fr,
    consumed_note: Note,
    merkle_path: MerklePath,
    created_notes: [Note; 2],
    rng: &mut R,
) -> (ark_groth16::Proof<Bls12_381>, PublicInputs) {
    // Compute public inputs natively
    let cm = r14_poseidon::commitment(&consumed_note);

    let mut current = cm;
    for i in 0..merkle_path.siblings.len() {
        if merkle_path.indices[i] {
            current = r14_poseidon::hash2(merkle_path.siblings[i], current);
        } else {
            current = r14_poseidon::hash2(current, merkle_path.siblings[i]);
        }
    }
    let old_root = current;

    let nullifier = r14_poseidon::poseidon_hash(&[secret_key, consumed_note.nonce]);
    let out_cm_0 = r14_poseidon::commitment(&created_notes[0]);
    let out_cm_1 = r14_poseidon::commitment(&created_notes[1]);

    let circuit = TransferCircuit {
        secret_key: Some(secret_key),
        consumed_note: Some(consumed_note),
        merkle_path: Some(merkle_path),
        created_notes: Some(created_notes),
    };

    let proof = Groth16::<Bls12_381>::prove(pk, circuit, rng).expect("proving failed");

    let public_inputs = PublicInputs {
        old_root,
        nullifier,
        out_commitment_0: out_cm_0,
        out_commitment_1: out_cm_1,
    };

    (proof, public_inputs)
}

/// Verify a proof off-chain
pub fn verify_offchain(
    vk: &VerifyingKey<Bls12_381>,
    proof: &ark_groth16::Proof<Bls12_381>,
    public_inputs: &PublicInputs,
) -> bool {
    let pvk = PreparedVerifyingKey::from(vk.clone());
    Groth16::<Bls12_381>::verify_with_processed_vk(&pvk, &public_inputs.to_vec(), proof)
        .unwrap_or(false)
}

/// Count constraints in the transfer circuit
pub fn constraint_count() -> usize {
    let cs = ConstraintSystem::<Fr>::new_ref();
    cs.set_optimization_goal(ark_relations::r1cs::OptimizationGoal::Constraints);
    cs.set_mode(ark_relations::r1cs::SynthesisMode::Setup);
    let circuit = TransferCircuit::empty();
    circuit.generate_constraints(cs.clone()).expect("constraint generation failed");
    cs.num_constraints()
}

// === Serialization for Soroban (delegated to r14-sdk) ===

pub use r14_sdk::serialize::{
    serialize_fr, serialize_g1, serialize_g2, serialize_vk_for_soroban, SerializedProof,
    SerializedVK,
};

/// Convenience wrapper that accepts PublicInputs (calls r14_sdk internally)
pub fn serialize_proof_for_soroban(
    proof: &ark_groth16::Proof<Bls12_381>,
    public_inputs: &PublicInputs,
) -> (SerializedProof, Vec<String>) {
    r14_sdk::serialize::serialize_proof_for_soroban(proof, &public_inputs.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_ff::UniformRand;
    use ark_relations::r1cs::ConstraintSynthesizer;
    use ark_std::rand::{rngs::StdRng, SeedableRng};
    use r14_types::{MerklePath, Note, SecretKey, MERKLE_DEPTH};

    fn test_rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    fn build_dummy_merkle_path(rng: &mut impl RngCore) -> MerklePath {
        let siblings: Vec<Fr> = (0..MERKLE_DEPTH).map(|_| Fr::rand(rng)).collect();
        let indices: Vec<bool> = (0..MERKLE_DEPTH).map(|i| i % 2 == 0).collect();
        MerklePath { siblings, indices }
    }

    fn test_scenario(rng: &mut impl RngCore) -> (Fr, Note, MerklePath, [Note; 2]) {
        let sk = SecretKey::random(rng);
        let owner = r14_poseidon::owner_hash(&sk);
        let consumed = Note::new(1000, 1, owner.0, rng);
        let path = build_dummy_merkle_path(rng);

        let recipient_sk = SecretKey::random(rng);
        let recipient_owner = r14_poseidon::owner_hash(&recipient_sk);
        let note_0 = Note::new(700, 1, recipient_owner.0, rng);
        let note_1 = Note::new(300, 1, owner.0, rng); // change back to sender

        (sk.0, consumed, path, [note_0, note_1])
    }

    #[test]
    fn test_valid_transfer() {
        let mut rng = test_rng();
        let (sk, consumed, path, created) = test_scenario(&mut rng);

        let (pk, vk) = setup(&mut rng);
        let (proof, pi) = prove(&pk, sk, consumed, path, created, &mut rng);
        assert!(verify_offchain(&vk, &proof, &pi));
    }

    #[test]
    fn test_wrong_secret_key() {
        let mut rng = test_rng();
        let (_, consumed, path, created) = test_scenario(&mut rng);
        let wrong_sk = Fr::rand(&mut rng); // wrong key

        let circuit = TransferCircuit {
            secret_key: Some(wrong_sk),
            consumed_note: Some(consumed),
            merkle_path: Some(path),
            created_notes: Some(created),
        };

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit.generate_constraints(cs.clone()).unwrap();
        assert!(!cs.is_satisfied().unwrap(), "should fail: wrong secret key");
    }

    #[test]
    fn test_wrong_merkle_path() {
        let mut rng = test_rng();
        let (sk, consumed, mut path, created) = test_scenario(&mut rng);
        // Corrupt one sibling
        path.siblings[0] = Fr::rand(&mut rng);

        // The circuit will compute a different root than what gets set as public input
        // We need to test at the proof level — the circuit itself always computes consistently
        // So instead: use prove() which computes root from the bad path, then tamper the root
        let (pk, vk) = setup(&mut rng);
        let (proof, mut pi) = prove(&pk, sk, consumed, path, created, &mut rng);
        // Tamper with root to simulate inclusion failure
        pi.old_root = Fr::rand(&mut rng);
        assert!(!verify_offchain(&vk, &proof, &pi), "should fail: wrong root");
    }

    #[test]
    fn test_value_mismatch() {
        let mut rng = test_rng();
        let sk = SecretKey::random(&mut rng);
        let owner = r14_poseidon::owner_hash(&sk);
        let consumed = Note::new(1000, 1, owner.0, &mut rng);
        let path = build_dummy_merkle_path(&mut rng);

        let recipient_sk = SecretKey::random(&mut rng);
        let recipient_owner = r14_poseidon::owner_hash(&recipient_sk);
        // Values don't sum to 1000
        let note_0 = Note::new(600, 1, recipient_owner.0, &mut rng);
        let note_1 = Note::new(300, 1, owner.0, &mut rng);

        let circuit = TransferCircuit {
            secret_key: Some(sk.0),
            consumed_note: Some(consumed),
            merkle_path: Some(path),
            created_notes: Some([note_0, note_1]),
        };

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit.generate_constraints(cs.clone()).unwrap();
        assert!(!cs.is_satisfied().unwrap(), "should fail: value mismatch");
    }

    #[test]
    fn test_constraint_count() {
        let count = constraint_count();
        println!("Transfer circuit constraint count: {}", count);
        assert!(count < 20_000, "constraint count {} exceeds 20K limit", count);
        assert!(count > 1_000, "constraint count {} suspiciously low", count);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut rng = test_rng();
        let (sk, consumed, path, created) = test_scenario(&mut rng);

        let (pk, vk) = setup(&mut rng);
        let (proof, pi) = prove(&pk, sk, consumed, path, created, &mut rng);

        let svk = serialize_vk_for_soroban(&vk);
        let (sp, spi) = serialize_proof_for_soroban(&proof, &pi);

        // IC length = 5 (1 constant + 4 public inputs)
        assert_eq!(svk.ic.len(), 5, "IC length should be 5 for 4 public inputs");

        // G1 = 96 bytes = 192 hex chars
        assert_eq!(svk.alpha_g1.len(), 192);
        assert_eq!(sp.a.len(), 192);
        assert_eq!(sp.c.len(), 192);
        for ic in &svk.ic {
            assert_eq!(ic.len(), 192);
        }

        // G2 = 192 bytes = 384 hex chars
        assert_eq!(svk.beta_g2.len(), 384);
        assert_eq!(sp.b.len(), 384);

        // Fr = 32 bytes = 64 hex chars
        assert_eq!(spi.len(), 4);
        for pi_hex in &spi {
            assert_eq!(pi_hex.len(), 64);
        }
    }

    #[test]
    fn test_app_tag_mismatch() {
        let mut rng = test_rng();
        let sk = SecretKey::random(&mut rng);
        let owner = r14_poseidon::owner_hash(&sk);
        let consumed = Note::new(1000, 1, owner.0, &mut rng);
        let path = build_dummy_merkle_path(&mut rng);

        let recipient_sk = SecretKey::random(&mut rng);
        let recipient_owner = r14_poseidon::owner_hash(&recipient_sk);
        // app_tag mismatch: consumed=1, created=2
        let note_0 = Note::new(700, 2, recipient_owner.0, &mut rng);
        let note_1 = Note::new(300, 1, owner.0, &mut rng);

        let circuit = TransferCircuit {
            secret_key: Some(sk.0),
            consumed_note: Some(consumed),
            merkle_path: Some(path),
            created_notes: Some([note_0, note_1]),
        };

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit.generate_constraints(cs.clone()).unwrap();
        assert!(!cs.is_satisfied().unwrap(), "should fail: app tag mismatch");
    }
}
