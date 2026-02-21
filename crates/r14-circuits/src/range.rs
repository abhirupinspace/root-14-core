use ark_bls12_381::{Bls12_381, Fr};
use ark_ff::{AdditiveGroup, PrimeField};
use ark_groth16::{Groth16, PreparedVerifyingKey, ProvingKey, VerifyingKey};
use ark_r1cs_std::{alloc::AllocVar, boolean::Boolean, eq::EqGadget, fields::fp::FpVar, fields::FieldVar};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, SynthesisError};
use ark_snark::SNARK;
use ark_std::rand::{CryptoRng, RngCore};
use r14_circuit::poseidon_gadget::poseidon_hash_var;

const RANGE_BITS: usize = 64;

/// "I know `x` committed as `cm = Poseidon(x, nonce)` such that `min <= x <= max`"
#[derive(Clone)]
pub struct RangeCircuit {
    pub x: Option<Fr>,
    pub nonce: Option<Fr>,
    pub min: Option<Fr>,
    pub max: Option<Fr>,
}

impl RangeCircuit {
    pub fn empty() -> Self {
        Self { x: None, nonce: None, min: None, max: None }
    }
}

/// Decompose `val` into `RANGE_BITS` Boolean witnesses and constrain reconstruction.
fn enforce_range_bits(
    cs: ConstraintSystemRef<Fr>,
    val: &FpVar<Fr>,
    native_val: Option<u64>,
) -> Result<(), SynthesisError> {
    let mut bits: Vec<Boolean<Fr>> = Vec::with_capacity(RANGE_BITS);
    for i in 0..RANGE_BITS {
        let bit = Boolean::new_witness(cs.clone(), || {
            let v = native_val.ok_or(SynthesisError::AssignmentMissing)?;
            Ok((v >> i) & 1 == 1)
        })?;
        bits.push(bit);
    }

    // Reconstruct: sum = Σ bit_i * 2^i
    let mut sum = FpVar::zero();
    let mut coeff = Fr::from(1u64);
    for bit in &bits {
        let bit_fp = FpVar::from(bit.clone());
        sum += bit_fp * coeff;
        coeff.double_in_place();
    }

    sum.enforce_equal(val)?;
    Ok(())
}

impl ConstraintSynthesizer<Fr> for RangeCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        // Public inputs: min, max, commitment
        let min_pub = FpVar::new_input(cs.clone(), || {
            self.min.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let max_pub = FpVar::new_input(cs.clone(), || {
            self.max.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let cm_pub = FpVar::new_input(cs.clone(), || {
            let x = self.x.ok_or(SynthesisError::AssignmentMissing)?;
            let nonce = self.nonce.ok_or(SynthesisError::AssignmentMissing)?;
            Ok(r14_poseidon::poseidon_hash(&[x, nonce]))
        })?;

        // Witnesses
        let x_var = FpVar::new_witness(cs.clone(), || {
            self.x.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let nonce_var = FpVar::new_witness(cs.clone(), || {
            self.nonce.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Constraint 1: poseidon(x, nonce) == commitment
        let computed_cm = poseidon_hash_var(cs.clone(), &[x_var.clone(), nonce_var])?;
        computed_cm.enforce_equal(&cm_pub)?;

        // Compute native values for bit decomposition
        let x_minus_min_native = match (self.x, self.min) {
            (Some(x), Some(min)) => {
                let x_big = x.into_bigint();
                let min_big = min.into_bigint();
                // x and min are small (fit in u64), so subtraction is direct
                let x_u64 = x_big.as_ref()[0];
                let min_u64 = min_big.as_ref()[0];
                Some(x_u64.wrapping_sub(min_u64))
            }
            _ => None,
        };

        let max_minus_x_native = match (self.x, self.max) {
            (Some(x), Some(max)) => {
                let x_big = x.into_bigint();
                let max_big = max.into_bigint();
                let x_u64 = x_big.as_ref()[0];
                let max_u64 = max_big.as_ref()[0];
                Some(max_u64.wrapping_sub(x_u64))
            }
            _ => None,
        };

        // Constraint 2: (x - min) decomposes into 64 bits
        let x_minus_min = &x_var - &min_pub;
        enforce_range_bits(cs.clone(), &x_minus_min, x_minus_min_native)?;

        // Constraint 3: (max - x) decomposes into 64 bits
        let max_minus_x = &max_pub - &x_var;
        enforce_range_bits(cs, &max_minus_x, max_minus_x_native)?;

        Ok(())
    }
}

pub struct PublicInputs {
    pub min: Fr,
    pub max: Fr,
    pub commitment: Fr,
}

impl PublicInputs {
    pub fn to_vec(&self) -> Vec<Fr> {
        vec![self.min, self.max, self.commitment]
    }
}

pub fn setup<R: RngCore + CryptoRng>(rng: &mut R) -> (ProvingKey<Bls12_381>, VerifyingKey<Bls12_381>) {
    let circuit = RangeCircuit::empty();
    Groth16::<Bls12_381>::circuit_specific_setup(circuit, rng).expect("setup failed")
}

pub fn prove<R: RngCore + CryptoRng>(
    pk: &ProvingKey<Bls12_381>,
    x: u64,
    nonce: Fr,
    min: u64,
    max: u64,
    rng: &mut R,
) -> (ark_groth16::Proof<Bls12_381>, PublicInputs) {
    let x_fr = Fr::from(x);
    let min_fr = Fr::from(min);
    let max_fr = Fr::from(max);
    let commitment = r14_poseidon::poseidon_hash(&[x_fr, nonce]);

    let circuit = RangeCircuit {
        x: Some(x_fr),
        nonce: Some(nonce),
        min: Some(min_fr),
        max: Some(max_fr),
    };
    let proof = Groth16::<Bls12_381>::prove(pk, circuit, rng).expect("proving failed");
    (proof, PublicInputs { min: min_fr, max: max_fr, commitment })
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
    let circuit = RangeCircuit::empty();
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
    fn test_valid_range() {
        let mut rng = test_rng();
        let nonce = Fr::rand(&mut rng);
        let (pk, vk) = setup(&mut rng);
        let (proof, pi) = prove(&pk, 500, nonce, 100, 1000, &mut rng);
        assert!(verify_offchain(&vk, &proof, &pi));
    }

    #[test]
    fn test_range_at_boundaries() {
        let mut rng = test_rng();
        let nonce = Fr::rand(&mut rng);
        let (pk, vk) = setup(&mut rng);

        // x == min
        let (proof, pi) = prove(&pk, 100, nonce, 100, 1000, &mut rng);
        assert!(verify_offchain(&vk, &proof, &pi), "x == min should pass");

        // x == max
        let nonce2 = Fr::rand(&mut rng);
        let (proof, pi) = prove(&pk, 1000, nonce2, 100, 1000, &mut rng);
        assert!(verify_offchain(&vk, &proof, &pi), "x == max should pass");
    }

    #[test]
    fn test_out_of_range() {
        let mut rng = test_rng();
        let nonce = Fr::rand(&mut rng);

        // x=50, min=100 → x-min underflows → can't decompose in 64 bits
        let circuit = RangeCircuit {
            x: Some(Fr::from(50u64)),
            nonce: Some(nonce),
            min: Some(Fr::from(100u64)),
            max: Some(Fr::from(1000u64)),
        };
        let cs = ConstraintSystem::<Fr>::new_ref();
        // This will panic or produce unsatisfied constraints because
        // 50 - 100 in the field is a huge number that can't be 64-bit decomposed
        let result = circuit.generate_constraints(cs.clone());
        if result.is_ok() {
            assert!(!cs.is_satisfied().unwrap(), "should fail: x < min");
        }
    }

    #[test]
    fn test_wrong_commitment() {
        let mut rng = test_rng();
        let nonce = Fr::rand(&mut rng);
        let (pk, vk) = setup(&mut rng);
        let (proof, mut pi) = prove(&pk, 500, nonce, 100, 1000, &mut rng);
        pi.commitment = Fr::rand(&mut rng);
        assert!(!verify_offchain(&vk, &proof, &pi), "should fail: wrong commitment");
    }

    #[test]
    fn test_range_constraint_count() {
        let count = constraint_count();
        println!("Range circuit constraints: {count}");
        assert!(count > 200, "too few: {count}");
        assert!(count < 1000, "too many: {count}");
    }
}
