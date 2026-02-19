use ark_bls12_381::Fr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Nullifier(pub Fr);

impl Nullifier {
    pub fn from_fr(fr: Fr) -> Self {
        Self(fr)
    }

    pub fn as_fr(&self) -> &Fr {
        &self.0
    }
}
