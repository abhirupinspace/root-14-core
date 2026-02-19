use ark_bls12_381::Fr;
use ark_crypto_primitives::sponge::{
    constraints::CryptographicSpongeVar,
    poseidon::constraints::PoseidonSpongeVar,
};
use ark_r1cs_std::fields::fp::FpVar;
use ark_relations::r1cs::ConstraintSystemRef;
use r14_poseidon::poseidon_config;

pub fn poseidon_hash_var(
    cs: ConstraintSystemRef<Fr>,
    inputs: &[FpVar<Fr>],
) -> Result<FpVar<Fr>, ark_relations::r1cs::SynthesisError> {
    let config = poseidon_config();
    let mut sponge = PoseidonSpongeVar::new(cs, &config);
    sponge.absorb(&inputs)?;
    let out = sponge.squeeze_field_elements(1)?;
    Ok(out.into_iter().next().unwrap())
}

pub fn hash2_var(
    cs: ConstraintSystemRef<Fr>,
    a: &FpVar<Fr>,
    b: &FpVar<Fr>,
) -> Result<FpVar<Fr>, ark_relations::r1cs::SynthesisError> {
    poseidon_hash_var(cs, &[a.clone(), b.clone()])
}
