use ark_bls12_381::Fr;
use ark_ff::UniformRand;
use ark_std::rand::Rng;

#[derive(Clone, Debug)]
pub struct Note {
    pub value: u64,
    pub app_tag: u32,
    pub owner: Fr,
    pub nonce: Fr,
}

impl Note {
    pub fn new<R: Rng>(value: u64, app_tag: u32, owner: Fr, rng: &mut R) -> Self {
        Self {
            value,
            app_tag,
            owner,
            nonce: Fr::rand(rng),
        }
    }

    pub fn with_nonce(value: u64, app_tag: u32, owner: Fr, nonce: Fr) -> Self {
        Self {
            value,
            app_tag,
            owner,
            nonce,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_std::test_rng;

    #[test]
    fn test_note_creation() {
        let mut rng = test_rng();
        let owner = Fr::rand(&mut rng);
        let n1 = Note::new(1000, 1, owner, &mut rng);
        let n2 = Note::new(1000, 1, owner, &mut rng);
        // Random nonces should differ
        assert_ne!(n1.nonce, n2.nonce);
        assert_eq!(n1.value, 1000);
        assert_eq!(n1.app_tag, 1);
    }
}
