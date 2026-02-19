extern crate alloc;

use alloc::vec::Vec;
use ark_bls12_381::Fr;

pub const MERKLE_DEPTH: usize = 20;

#[derive(Clone, Debug)]
pub struct MerklePath {
    pub siblings: Vec<Fr>,
    pub indices: Vec<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MerkleRoot(pub Fr);
