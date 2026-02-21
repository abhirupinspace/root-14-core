use ark_bls12_381::Fr;
use ark_ff::AdditiveGroup;
use r14_poseidon::hash2;
use r14_types::{MerklePath, MerkleRoot, MERKLE_DEPTH};

pub struct SparseMerkleTree {
    leaves: Vec<Fr>,
    zeros: Vec<Fr>,
}

impl SparseMerkleTree {
    pub fn new() -> Self {
        let mut zeros = vec![Fr::ZERO; MERKLE_DEPTH + 1];
        for i in 1..=MERKLE_DEPTH {
            zeros[i] = hash2(zeros[i - 1], zeros[i - 1]);
        }
        Self {
            leaves: Vec::new(),
            zeros,
        }
    }

    pub fn insert(&mut self, leaf: Fr) -> usize {
        let idx = self.leaves.len();
        self.leaves.push(leaf);
        idx
    }

    pub fn next_index(&self) -> usize {
        self.leaves.len()
    }

    pub fn leaves(&self) -> &[Fr] {
        &self.leaves
    }

    pub fn root(&self) -> MerkleRoot {
        if self.leaves.is_empty() {
            return MerkleRoot(self.zeros[MERKLE_DEPTH]);
        }
        let mut layer: Vec<Fr> = self.leaves.clone();
        for level in 0..MERKLE_DEPTH {
            let mut next = Vec::with_capacity((layer.len() + 1) / 2);
            let zero = self.zeros[level];
            let mut i = 0;
            while i < layer.len() {
                let left = layer[i];
                let right = if i + 1 < layer.len() {
                    layer[i + 1]
                } else {
                    zero
                };
                next.push(hash2(left, right));
                i += 2;
            }
            layer = next;
        }
        MerkleRoot(layer[0])
    }

    pub fn proof(&self, index: usize) -> MerklePath {
        assert!(index < self.leaves.len(), "index out of bounds");
        let mut siblings = Vec::with_capacity(MERKLE_DEPTH);
        let mut indices = Vec::with_capacity(MERKLE_DEPTH);
        let mut layer: Vec<Fr> = self.leaves.clone();
        let mut idx = index;

        for level in 0..MERKLE_DEPTH {
            let zero = self.zeros[level];
            let is_right = idx & 1 == 1;
            indices.push(is_right);

            let sibling_idx = if is_right { idx - 1 } else { idx + 1 };
            let sibling = if sibling_idx < layer.len() {
                layer[sibling_idx]
            } else {
                zero
            };
            siblings.push(sibling);

            // build next layer
            let mut next = Vec::with_capacity((layer.len() + 1) / 2);
            let mut i = 0;
            while i < layer.len() {
                let left = layer[i];
                let right = if i + 1 < layer.len() {
                    layer[i + 1]
                } else {
                    zero
                };
                next.push(hash2(left, right));
                i += 2;
            }
            layer = next;
            idx /= 2;
        }

        MerklePath { siblings, indices }
    }
}

/// Verify a Merkle proof against a root (used in tests + API consumers)
pub fn verify_proof(leaf: Fr, path: &MerklePath, root: &MerkleRoot) -> bool {
    let mut current = leaf;
    for i in 0..path.siblings.len() {
        if path.indices[i] {
            current = hash2(path.siblings[i], current);
        } else {
            current = hash2(current, path.siblings[i]);
        }
    }
    current == root.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_ff::UniformRand;

    #[test]
    fn empty_root_deterministic() {
        let t1 = SparseMerkleTree::new();
        let t2 = SparseMerkleTree::new();
        assert_eq!(t1.root().0, t2.root().0);
    }

    #[test]
    fn single_insert_changes_root() {
        let mut tree = SparseMerkleTree::new();
        let empty_root = tree.root();
        let mut rng = ark_std::test_rng();
        tree.insert(Fr::rand(&mut rng));
        assert_ne!(tree.root().0, empty_root.0);
    }

    #[test]
    fn proof_verifies() {
        let mut tree = SparseMerkleTree::new();
        let mut rng = ark_std::test_rng();
        let leaf = Fr::rand(&mut rng);
        tree.insert(leaf);
        tree.insert(Fr::rand(&mut rng));
        tree.insert(Fr::rand(&mut rng));

        let proof = tree.proof(0);
        let root = tree.root();
        assert!(verify_proof(leaf, &proof, &root));
    }

    #[test]
    fn rebuild_consistency() {
        let mut rng = ark_std::test_rng();
        let leaves: Vec<Fr> = (0..5).map(|_| Fr::rand(&mut rng)).collect();

        let mut t1 = SparseMerkleTree::new();
        for l in &leaves {
            t1.insert(*l);
        }

        let mut t2 = SparseMerkleTree::new();
        for l in &leaves {
            t2.insert(*l);
        }

        assert_eq!(t1.root().0, t2.root().0);
    }

    #[test]
    fn all_proofs_verify() {
        let mut tree = SparseMerkleTree::new();
        let mut rng = ark_std::test_rng();
        let leaves: Vec<Fr> = (0..8).map(|_| Fr::rand(&mut rng)).collect();
        for l in &leaves {
            tree.insert(*l);
        }
        let root = tree.root();
        for (i, leaf) in leaves.iter().enumerate() {
            let proof = tree.proof(i);
            assert!(verify_proof(*leaf, &proof, &root), "proof failed for index {i}");
        }
    }
}
