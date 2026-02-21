use ark_bls12_381::{Bls12_381, Fr};
use ark_groth16::{Groth16, PreparedVerifyingKey, ProvingKey, VerifyingKey};
use ark_r1cs_std::{alloc::AllocVar, boolean::Boolean, eq::EqGadget, fields::fp::FpVar};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, SynthesisError};
use ark_snark::SNARK;
use ark_std::rand::{CryptoRng, RngCore};
use r14_circuit::merkle_gadget::verify_merkle_path;
use r14_circuit::poseidon_gadget::poseidon_hash_var;
use r14_types::MERKLE_DEPTH;

/// "I know a leaf + path such that leaf is in the tree with the given root"
#[derive(Clone)]
pub struct MembershipCircuit {
    pub leaf_preimage: Option<Fr>,
    pub siblings: Option<Vec<Fr>>,
    pub indices: Option<Vec<bool>>,
}

impl MembershipCircuit {
    pub fn empty() -> Self {
        Self { leaf_preimage: None, siblings: None, indices: None }
    }
}

impl ConstraintSynthesizer<Fr> for MembershipCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        // Public inputs: root, leaf_commitment
        let root_pub = FpVar::new_input(cs.clone(), || {
            let leaf = self.leaf_preimage.ok_or(SynthesisError::AssignmentMissing)?;
            let siblings = self.siblings.as_ref().ok_or(SynthesisError::AssignmentMissing)?;
            let indices = self.indices.as_ref().ok_or(SynthesisError::AssignmentMissing)?;
            let cm = r14_poseidon::poseidon_hash(&[leaf]);
            let mut current = cm;
            for i in 0..siblings.len() {
                if indices[i] {
                    current = r14_poseidon::hash2(siblings[i], current);
                } else {
                    current = r14_poseidon::hash2(current, siblings[i]);
                }
            }
            Ok(current)
        })?;

        let leaf_cm_pub = FpVar::new_input(cs.clone(), || {
            let leaf = self.leaf_preimage.ok_or(SynthesisError::AssignmentMissing)?;
            Ok(r14_poseidon::poseidon_hash(&[leaf]))
        })?;

        // Witnesses
        let leaf_var = FpVar::new_witness(cs.clone(), || {
            self.leaf_preimage.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let mut path_vars: Vec<(FpVar<Fr>, Boolean<Fr>)> = Vec::with_capacity(MERKLE_DEPTH);
        for i in 0..MERKLE_DEPTH {
            let sibling = FpVar::new_witness(cs.clone(), || {
                let siblings = self.siblings.as_ref().ok_or(SynthesisError::AssignmentMissing)?;
                Ok(siblings[i])
            })?;
            let index_bit = Boolean::new_witness(cs.clone(), || {
                let indices = self.indices.as_ref().ok_or(SynthesisError::AssignmentMissing)?;
                Ok(indices[i])
            })?;
            path_vars.push((sibling, index_bit));
        }

        // Constraint 1: poseidon(leaf_preimage) == leaf_commitment
        let computed_cm = poseidon_hash_var(cs.clone(), &[leaf_var])?;
        computed_cm.enforce_equal(&leaf_cm_pub)?;

        // Constraint 2: merkle path from leaf_commitment to root
        verify_merkle_path(cs, &computed_cm, &path_vars, &root_pub)?;

        Ok(())
    }
}

pub struct PublicInputs {
    pub root: Fr,
    pub leaf_commitment: Fr,
}

impl PublicInputs {
    pub fn to_vec(&self) -> Vec<Fr> {
        vec![self.root, self.leaf_commitment]
    }
}

pub fn setup<R: RngCore + CryptoRng>(rng: &mut R) -> (ProvingKey<Bls12_381>, VerifyingKey<Bls12_381>) {
    let circuit = MembershipCircuit::empty();
    Groth16::<Bls12_381>::circuit_specific_setup(circuit, rng).expect("setup failed")
}

pub fn prove<R: RngCore + CryptoRng>(
    pk: &ProvingKey<Bls12_381>,
    leaf_preimage: Fr,
    siblings: Vec<Fr>,
    indices: Vec<bool>,
    rng: &mut R,
) -> (ark_groth16::Proof<Bls12_381>, PublicInputs) {
    let leaf_commitment = r14_poseidon::poseidon_hash(&[leaf_preimage]);
    let mut current = leaf_commitment;
    for i in 0..siblings.len() {
        if indices[i] {
            current = r14_poseidon::hash2(siblings[i], current);
        } else {
            current = r14_poseidon::hash2(current, siblings[i]);
        }
    }
    let root = current;

    let circuit = MembershipCircuit {
        leaf_preimage: Some(leaf_preimage),
        siblings: Some(siblings),
        indices: Some(indices),
    };
    let proof = Groth16::<Bls12_381>::prove(pk, circuit, rng).expect("proving failed");
    (proof, PublicInputs { root, leaf_commitment })
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
    let circuit = MembershipCircuit::empty();
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

    fn dummy_path(rng: &mut impl RngCore) -> (Vec<Fr>, Vec<bool>) {
        let siblings: Vec<Fr> = (0..MERKLE_DEPTH).map(|_| Fr::rand(rng)).collect();
        let indices: Vec<bool> = (0..MERKLE_DEPTH).map(|i| i % 2 == 0).collect();
        (siblings, indices)
    }

    #[test]
    fn test_valid_membership() {
        let mut rng = test_rng();
        let leaf = Fr::rand(&mut rng);
        let (siblings, indices) = dummy_path(&mut rng);

        let (pk, vk) = setup(&mut rng);
        let (proof, pi) = prove(&pk, leaf, siblings, indices, &mut rng);
        assert!(verify_offchain(&vk, &proof, &pi));
    }

    #[test]
    fn test_wrong_root() {
        let mut rng = test_rng();
        let leaf = Fr::rand(&mut rng);
        let (siblings, indices) = dummy_path(&mut rng);

        let (pk, vk) = setup(&mut rng);
        let (proof, mut pi) = prove(&pk, leaf, siblings, indices, &mut rng);
        pi.root = Fr::rand(&mut rng);
        assert!(!verify_offchain(&vk, &proof, &pi), "should fail: wrong root");
    }

    #[test]
    fn test_wrong_leaf() {
        let mut rng = test_rng();
        let leaf = Fr::rand(&mut rng);
        let wrong_leaf = Fr::rand(&mut rng);
        let (siblings, indices) = dummy_path(&mut rng);

        let (pk, vk) = setup(&mut rng);
        let (proof, _) = prove(&pk, wrong_leaf, siblings, indices, &mut rng);
        // Verify against commitment of the real leaf
        let pi = PublicInputs {
            root: Fr::rand(&mut rng), // doesn't matter, commitment mismatch first
            leaf_commitment: r14_poseidon::poseidon_hash(&[leaf]),
        };
        assert!(!verify_offchain(&vk, &proof, &pi), "should fail: wrong leaf");
    }

    #[test]
    fn test_membership_constraint_count() {
        let count = constraint_count();
        println!("Membership circuit constraints: {count}");
        assert!(count > 2000, "too few: {count}");
        assert!(count < 10000, "too many: {count}");
    }
}
