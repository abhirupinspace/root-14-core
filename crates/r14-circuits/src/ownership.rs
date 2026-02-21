use ark_bls12_381::{Bls12_381, Fr};
use ark_groth16::{Groth16, PreparedVerifyingKey, ProvingKey, VerifyingKey};
use ark_r1cs_std::{alloc::AllocVar, eq::EqGadget, fields::fp::FpVar};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, SynthesisError};
use ark_snark::SNARK;
use ark_std::rand::{CryptoRng, RngCore};
use r14_circuit::poseidon_gadget::poseidon_hash_var;

/// "I know `sk` such that `Poseidon(sk) == owner_hash`"
#[derive(Clone)]
pub struct OwnershipCircuit {
    pub secret_key: Option<Fr>,
}

impl OwnershipCircuit {
    pub fn empty() -> Self {
        Self { secret_key: None }
    }
}

impl ConstraintSynthesizer<Fr> for OwnershipCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let owner_hash_pub = FpVar::new_input(cs.clone(), || {
            let sk = self.secret_key.ok_or(SynthesisError::AssignmentMissing)?;
            Ok(r14_poseidon::poseidon_hash(&[sk]))
        })?;

        let sk_var = FpVar::new_witness(cs.clone(), || {
            self.secret_key.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let computed = poseidon_hash_var(cs, &[sk_var])?;
        computed.enforce_equal(&owner_hash_pub)?;

        Ok(())
    }
}

pub struct PublicInputs {
    pub owner_hash: Fr,
}

impl PublicInputs {
    pub fn to_vec(&self) -> Vec<Fr> {
        vec![self.owner_hash]
    }
}

pub fn setup<R: RngCore + CryptoRng>(rng: &mut R) -> (ProvingKey<Bls12_381>, VerifyingKey<Bls12_381>) {
    let circuit = OwnershipCircuit::empty();
    Groth16::<Bls12_381>::circuit_specific_setup(circuit, rng).expect("setup failed")
}

pub fn prove<R: RngCore + CryptoRng>(
    pk: &ProvingKey<Bls12_381>,
    secret_key: Fr,
    rng: &mut R,
) -> (ark_groth16::Proof<Bls12_381>, PublicInputs) {
    let owner_hash = r14_poseidon::poseidon_hash(&[secret_key]);
    let circuit = OwnershipCircuit { secret_key: Some(secret_key) };
    let proof = Groth16::<Bls12_381>::prove(pk, circuit, rng).expect("proving failed");
    (proof, PublicInputs { owner_hash })
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
    let circuit = OwnershipCircuit::empty();
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
    fn test_valid_ownership() {
        let mut rng = test_rng();
        let sk = Fr::rand(&mut rng);
        let (pk, vk) = setup(&mut rng);
        let (proof, pi) = prove(&pk, sk, &mut rng);
        assert!(verify_offchain(&vk, &proof, &pi));
    }

    #[test]
    fn test_wrong_secret_key() {
        let mut rng = test_rng();
        let real_sk = Fr::rand(&mut rng);
        let wrong_sk = Fr::rand(&mut rng);

        let (pk, vk) = setup(&mut rng);
        let (proof, _) = prove(&pk, wrong_sk, &mut rng);
        let pi = PublicInputs { owner_hash: r14_poseidon::poseidon_hash(&[real_sk]) };
        assert!(!verify_offchain(&vk, &proof, &pi), "should fail: wrong sk");
    }

    #[test]
    fn test_ownership_constraint_count() {
        let count = constraint_count();
        println!("Ownership circuit constraints: {count}");
        assert!(count > 100, "too few: {count}");
        assert!(count < 1000, "too many: {count}");
    }
}
