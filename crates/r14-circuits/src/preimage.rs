use ark_bls12_381::{Bls12_381, Fr};
use ark_groth16::{Groth16, PreparedVerifyingKey, ProvingKey, VerifyingKey};
use ark_r1cs_std::{alloc::AllocVar, eq::EqGadget, fields::fp::FpVar};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, SynthesisError};
use ark_snark::SNARK;
use ark_std::rand::{CryptoRng, RngCore};
use r14_circuit::poseidon_gadget::poseidon_hash_var;

/// "I know `x` such that `Poseidon(x) == hash`"
#[derive(Clone)]
pub struct PreimageCircuit {
    pub preimage: Option<Fr>,
}

impl PreimageCircuit {
    pub fn empty() -> Self {
        Self { preimage: None }
    }
}

impl ConstraintSynthesizer<Fr> for PreimageCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        // Public input: hash
        let hash_pub = FpVar::new_input(cs.clone(), || {
            let x = self.preimage.ok_or(SynthesisError::AssignmentMissing)?;
            Ok(r14_poseidon::poseidon_hash(&[x]))
        })?;

        // Witness: preimage
        let preimage_var = FpVar::new_witness(cs.clone(), || {
            self.preimage.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Constraint: poseidon(preimage) == hash
        let computed = poseidon_hash_var(cs, &[preimage_var])?;
        computed.enforce_equal(&hash_pub)?;

        Ok(())
    }
}

pub struct PublicInputs {
    pub hash: Fr,
}

impl PublicInputs {
    pub fn to_vec(&self) -> Vec<Fr> {
        vec![self.hash]
    }
}

pub fn setup<R: RngCore + CryptoRng>(rng: &mut R) -> (ProvingKey<Bls12_381>, VerifyingKey<Bls12_381>) {
    let circuit = PreimageCircuit::empty();
    Groth16::<Bls12_381>::circuit_specific_setup(circuit, rng).expect("setup failed")
}

pub fn prove<R: RngCore + CryptoRng>(
    pk: &ProvingKey<Bls12_381>,
    preimage: Fr,
    rng: &mut R,
) -> (ark_groth16::Proof<Bls12_381>, PublicInputs) {
    let hash = r14_poseidon::poseidon_hash(&[preimage]);
    let circuit = PreimageCircuit { preimage: Some(preimage) };
    let proof = Groth16::<Bls12_381>::prove(pk, circuit, rng).expect("proving failed");
    (proof, PublicInputs { hash })
}

pub fn verify_offchain(
    vk: &VerifyingKey<Bls12_381>,
    proof: &ark_groth16::Proof<Bls12_381>,
    pi: &PublicInputs,
) -> bool {
    let pvk = PreparedVerifyingKey::from(vk.clone());
    Groth16::<Bls12_381>::verify_with_processed_vk(&pvk, &pi.to_vec(), proof).unwrap_or(false)
}

pub fn constraint_count() -> usize {
    let cs = ConstraintSystem::<Fr>::new_ref();
    cs.set_optimization_goal(ark_relations::r1cs::OptimizationGoal::Constraints);
    cs.set_mode(ark_relations::r1cs::SynthesisMode::Setup);
    let circuit = PreimageCircuit::empty();
    circuit.generate_constraints(cs.clone()).expect("constraint generation failed");
    cs.num_constraints()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_ff::UniformRand;
    use ark_std::rand::{rngs::StdRng, SeedableRng};

    fn test_rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    #[test]
    fn test_valid_preimage() {
        let mut rng = test_rng();
        let secret = Fr::rand(&mut rng);
        let (pk, vk) = setup(&mut rng);
        let (proof, pi) = prove(&pk, secret, &mut rng);
        assert!(verify_offchain(&vk, &proof, &pi));
    }

    #[test]
    fn test_wrong_preimage() {
        let mut rng = test_rng();
        let secret = Fr::rand(&mut rng);
        let wrong = Fr::rand(&mut rng);

        let (pk, vk) = setup(&mut rng);
        let (proof, _) = prove(&pk, wrong, &mut rng);
        // Verify against hash of `secret` (different from hash of `wrong`)
        let pi = PublicInputs { hash: r14_poseidon::poseidon_hash(&[secret]) };
        assert!(!verify_offchain(&vk, &proof, &pi), "should fail: wrong preimage");
    }

    #[test]
    fn test_preimage_constraint_count() {
        let count = constraint_count();
        println!("Preimage circuit constraints: {count}");
        assert!(count > 100, "too few: {count}");
        assert!(count < 1000, "too many: {count}");
    }
}
