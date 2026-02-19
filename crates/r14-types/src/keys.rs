use ark_bls12_381::Fr;
use ark_ff::UniformRand;
use ark_std::rand::Rng;

#[derive(Clone, Debug)]
pub struct SecretKey(pub Fr);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnerHash(pub Fr);

impl SecretKey {
    pub fn random<R: Rng>(rng: &mut R) -> Self {
        Self(Fr::rand(rng))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_std::test_rng;

    #[test]
    fn test_secret_key_random() {
        let mut rng = test_rng();
        let sk1 = SecretKey::random(&mut rng);
        let sk2 = SecretKey::random(&mut rng);
        assert_ne!(sk1.0, sk2.0);
    }
}
