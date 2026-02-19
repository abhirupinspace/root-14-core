use ark_bls12_381::Fr;
use ark_crypto_primitives::sponge::{
    poseidon::{PoseidonConfig, PoseidonSponge},
    CryptographicSponge, FieldBasedCryptographicSponge,
};
use ark_ff::PrimeField;
use r14_types::{Note, Nullifier, OwnerHash, SecretKey};

const RATE: usize = 2;
const FULL_ROUNDS: usize = 8;
const PARTIAL_ROUNDS: usize = 31;
const ALPHA: u64 = 17;

pub fn poseidon_config() -> PoseidonConfig<Fr> {
    let (ark, mds) =
        ark_crypto_primitives::sponge::poseidon::find_poseidon_ark_and_mds::<Fr>(
            Fr::MODULUS_BIT_SIZE as u64,
            RATE,
            FULL_ROUNDS as u64,
            PARTIAL_ROUNDS as u64,
            0,
        );
    PoseidonConfig::new(FULL_ROUNDS, PARTIAL_ROUNDS, ALPHA, mds, ark, RATE, 1)
}

pub fn poseidon_hash(inputs: &[Fr]) -> Fr {
    let config = poseidon_config();
    let mut sponge = PoseidonSponge::new(&config);
    sponge.absorb(&inputs);
    sponge.squeeze_native_field_elements(1)[0]
}

pub fn hash2(a: Fr, b: Fr) -> Fr {
    poseidon_hash(&[a, b])
}

pub fn commitment(note: &Note) -> Fr {
    poseidon_hash(&[
        Fr::from(note.value),
        Fr::from(note.app_tag as u64),
        note.owner,
        note.nonce,
    ])
}

pub fn nullifier(sk: &SecretKey, nonce: &Fr) -> Nullifier {
    Nullifier::from_fr(hash2(sk.0, *nonce))
}

pub fn owner_hash(sk: &SecretKey) -> OwnerHash {
    OwnerHash(poseidon_hash(&[sk.0]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_ff::UniformRand;
    use ark_std::test_rng;

    #[test]
    fn test_hash2_deterministic() {
        let mut rng = test_rng();
        let a = Fr::rand(&mut rng);
        let b = Fr::rand(&mut rng);
        assert_eq!(hash2(a, b), hash2(a, b));
    }

    #[test]
    fn test_hash2_order_matters() {
        let mut rng = test_rng();
        let a = Fr::rand(&mut rng);
        let b = Fr::rand(&mut rng);
        assert_ne!(hash2(a, b), hash2(b, a));
    }

    #[test]
    fn test_commitment_deterministic() {
        let mut rng = test_rng();
        let owner = Fr::rand(&mut rng);
        let nonce = Fr::rand(&mut rng);
        let note = Note::with_nonce(1000, 1, owner, nonce);
        assert_eq!(commitment(&note), commitment(&note));
    }

    #[test]
    fn test_nullifier_deterministic() {
        let mut rng = test_rng();
        let sk = SecretKey::random(&mut rng);
        let nonce = Fr::rand(&mut rng);
        assert_eq!(nullifier(&sk, &nonce), nullifier(&sk, &nonce));
    }

    #[test]
    fn test_different_nonces_different_nullifiers() {
        let mut rng = test_rng();
        let sk = SecretKey::random(&mut rng);
        let n1 = Fr::rand(&mut rng);
        let n2 = Fr::rand(&mut rng);
        assert_ne!(nullifier(&sk, &n1), nullifier(&sk, &n2));
    }

    #[test]
    fn test_owner_hash_deterministic() {
        let mut rng = test_rng();
        let sk = SecretKey::random(&mut rng);
        assert_eq!(owner_hash(&sk), owner_hash(&sk));
    }
}
