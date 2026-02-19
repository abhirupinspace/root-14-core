use ark_bls12_381::Fr;
use ark_r1cs_std::{boolean::Boolean, fields::fp::FpVar, prelude::EqGadget};
use ark_relations::r1cs::{ConstraintSystemRef, SynthesisError};

use crate::poseidon_gadget::hash2_var;

/// Verify a Merkle path in-circuit.
/// `path` is a slice of (sibling, index_bit) where index_bit=true means leaf is on the right.
pub fn verify_merkle_path(
    cs: ConstraintSystemRef<Fr>,
    leaf: &FpVar<Fr>,
    path: &[(FpVar<Fr>, Boolean<Fr>)],
    root: &FpVar<Fr>,
) -> Result<(), SynthesisError> {
    let mut current = leaf.clone();

    for (sibling, is_right) in path {
        // if is_right: hash(sibling, current), else: hash(current, sibling)
        let left = is_right.select(sibling, &current)?;
        let right = is_right.select(&current, sibling)?;
        current = hash2_var(cs.clone(), &left, &right)?;
    }

    current.enforce_equal(root)?;
    Ok(())
}
