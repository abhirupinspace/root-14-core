use ark_bls12_381::Fr;
use ark_r1cs_std::{
    alloc::AllocVar, boolean::Boolean, eq::EqGadget, fields::fp::FpVar,
};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use r14_types::{MerklePath, Note, MERKLE_DEPTH};

use crate::merkle_gadget::verify_merkle_path;
use crate::poseidon_gadget::poseidon_hash_var;

#[derive(Clone)]
pub struct TransferCircuit {
    // Private witnesses
    pub secret_key: Option<Fr>,
    pub consumed_note: Option<Note>,
    pub merkle_path: Option<MerklePath>,
    pub created_notes: Option<[Note; 2]>,
}

impl TransferCircuit {
    /// Create a circuit with None witnesses (for setup)
    pub fn empty() -> Self {
        Self {
            secret_key: None,
            consumed_note: None,
            merkle_path: None,
            created_notes: None,
        }
    }
}

impl ConstraintSynthesizer<Fr> for TransferCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        // === Public inputs (4 Fr elements) ===
        // Order: old_root, nullifier, out_commitment_0, out_commitment_1
        let old_root_pub = FpVar::new_input(cs.clone(), || {
            let note = self.consumed_note.as_ref().ok_or(SynthesisError::AssignmentMissing)?;
            let path = self.merkle_path.as_ref().ok_or(SynthesisError::AssignmentMissing)?;
            // Compute root from path natively to get the public input value
            let cm = r14_poseidon::commitment(note);
            let mut current = cm;
            for i in 0..path.siblings.len() {
                if path.indices[i] {
                    current = r14_poseidon::hash2(path.siblings[i], current);
                } else {
                    current = r14_poseidon::hash2(current, path.siblings[i]);
                }
            }
            Ok(current)
        })?;

        let nullifier_pub = FpVar::new_input(cs.clone(), || {
            let sk = self.secret_key.ok_or(SynthesisError::AssignmentMissing)?;
            let note = self.consumed_note.as_ref().ok_or(SynthesisError::AssignmentMissing)?;
            Ok(r14_poseidon::poseidon_hash(&[sk, note.nonce]))
        })?;

        let out_cm_0_pub = FpVar::new_input(cs.clone(), || {
            let notes = self.created_notes.as_ref().ok_or(SynthesisError::AssignmentMissing)?;
            Ok(r14_poseidon::commitment(&notes[0]))
        })?;

        let out_cm_1_pub = FpVar::new_input(cs.clone(), || {
            let notes = self.created_notes.as_ref().ok_or(SynthesisError::AssignmentMissing)?;
            Ok(r14_poseidon::commitment(&notes[1]))
        })?;

        // === Private witnesses ===
        let sk_var = FpVar::new_witness(cs.clone(), || {
            self.secret_key.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let consumed_value = FpVar::new_witness(cs.clone(), || {
            let note = self.consumed_note.as_ref().ok_or(SynthesisError::AssignmentMissing)?;
            Ok(Fr::from(note.value))
        })?;

        let consumed_app_tag = FpVar::new_witness(cs.clone(), || {
            let note = self.consumed_note.as_ref().ok_or(SynthesisError::AssignmentMissing)?;
            Ok(Fr::from(note.app_tag as u64))
        })?;

        let consumed_owner = FpVar::new_witness(cs.clone(), || {
            let note = self.consumed_note.as_ref().ok_or(SynthesisError::AssignmentMissing)?;
            Ok(note.owner)
        })?;

        let consumed_nonce = FpVar::new_witness(cs.clone(), || {
            let note = self.consumed_note.as_ref().ok_or(SynthesisError::AssignmentMissing)?;
            Ok(note.nonce)
        })?;

        // Merkle path witnesses
        let mut path_vars: Vec<(FpVar<Fr>, Boolean<Fr>)> = Vec::with_capacity(MERKLE_DEPTH);
        for i in 0..MERKLE_DEPTH {
            let sibling = FpVar::new_witness(cs.clone(), || {
                let path = self.merkle_path.as_ref().ok_or(SynthesisError::AssignmentMissing)?;
                Ok(path.siblings[i])
            })?;
            let index_bit = Boolean::new_witness(cs.clone(), || {
                let path = self.merkle_path.as_ref().ok_or(SynthesisError::AssignmentMissing)?;
                Ok(path.indices[i])
            })?;
            path_vars.push((sibling, index_bit));
        }

        // Created note witnesses
        let mut created_values = Vec::with_capacity(2);
        let mut created_app_tags = Vec::with_capacity(2);
        let mut created_owners = Vec::with_capacity(2);
        let mut created_nonces = Vec::with_capacity(2);

        for i in 0..2 {
            created_values.push(FpVar::new_witness(cs.clone(), || {
                let notes = self.created_notes.as_ref().ok_or(SynthesisError::AssignmentMissing)?;
                Ok(Fr::from(notes[i].value))
            })?);
            created_app_tags.push(FpVar::new_witness(cs.clone(), || {
                let notes = self.created_notes.as_ref().ok_or(SynthesisError::AssignmentMissing)?;
                Ok(Fr::from(notes[i].app_tag as u64))
            })?);
            created_owners.push(FpVar::new_witness(cs.clone(), || {
                let notes = self.created_notes.as_ref().ok_or(SynthesisError::AssignmentMissing)?;
                Ok(notes[i].owner)
            })?);
            created_nonces.push(FpVar::new_witness(cs.clone(), || {
                let notes = self.created_notes.as_ref().ok_or(SynthesisError::AssignmentMissing)?;
                Ok(notes[i].nonce)
            })?);
        }

        // === Constraint 1: Ownership ===
        // owner_hash = poseidon(sk), enforce == consumed_note.owner
        let computed_owner = poseidon_hash_var(cs.clone(), &[sk_var.clone()])?;
        computed_owner.enforce_equal(&consumed_owner)?;

        // === Constraint 2: Consumed note commitment ===
        let consumed_cm = poseidon_hash_var(
            cs.clone(),
            &[consumed_value.clone(), consumed_app_tag.clone(), consumed_owner.clone(), consumed_nonce.clone()],
        )?;

        // === Constraint 3: Merkle inclusion ===
        verify_merkle_path(cs.clone(), &consumed_cm, &path_vars, &old_root_pub)?;

        // === Constraint 4: Nullifier ===
        let computed_nf = poseidon_hash_var(cs.clone(), &[sk_var.clone(), consumed_nonce.clone()])?;
        computed_nf.enforce_equal(&nullifier_pub)?;

        // === Constraint 5: Output commitments ===
        let computed_cm_0 = poseidon_hash_var(
            cs.clone(),
            &[created_values[0].clone(), created_app_tags[0].clone(), created_owners[0].clone(), created_nonces[0].clone()],
        )?;
        computed_cm_0.enforce_equal(&out_cm_0_pub)?;

        let computed_cm_1 = poseidon_hash_var(
            cs.clone(),
            &[created_values[1].clone(), created_app_tags[1].clone(), created_owners[1].clone(), created_nonces[1].clone()],
        )?;
        computed_cm_1.enforce_equal(&out_cm_1_pub)?;

        // === Constraint 6: Value conservation ===
        // consumed.value == created[0].value + created[1].value
        let sum = &created_values[0] + &created_values[1];
        consumed_value.enforce_equal(&sum)?;

        // === Constraint 7: App tag match ===
        consumed_app_tag.enforce_equal(&created_app_tags[0])?;
        consumed_app_tag.enforce_equal(&created_app_tags[1])?;

        Ok(())
    }
}
