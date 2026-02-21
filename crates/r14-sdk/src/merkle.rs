// Copyright 2026 abhirupbanerjee
// Licensed under the Apache License, Version 2.0

//! Merkle tree computation for Root14's sparse Merkle tree.
//!
//! Provides both offline root computation from a leaf list and
//! indexer-backed root computation that fetches existing leaves
//! over HTTP before appending new commitments.
//!
//! The tree uses Poseidon `hash2` with depth [`MERKLE_DEPTH`]
//! and zero-valued empty leaves.
//!
//! # Example
//!
//! ```rust
//! use r14_sdk::merkle::{empty_root_hex, compute_root_from_leaves};
//!
//! // empty tree
//! let root = empty_root_hex();
//! assert_eq!(root.len(), 64); // 32 bytes, no 0x prefix
//!
//! // with leaves
//! # use ark_bls12_381::Fr;
//! let root = compute_root_from_leaves(&[Fr::from(1u64), Fr::from(2u64)]);
//! ```

use anyhow::{Context, Result};
use ark_bls12_381::Fr;
use ark_ff::AdditiveGroup;
use r14_poseidon::hash2;
use r14_types::MERKLE_DEPTH;

use crate::wallet::hex_to_fr;

/// Compute the empty Merkle root: hash2(0,0) iterated MERKLE_DEPTH times
pub fn empty_root() -> Fr {
    let mut h = Fr::ZERO;
    for _ in 0..MERKLE_DEPTH {
        h = hash2(h, h);
    }
    h
}

/// Compute the Merkle root from a list of leaves (mirrors indexer's SparseMerkleTree::root)
fn compute_root(leaves: &[Fr]) -> Fr {
    if leaves.is_empty() {
        return empty_root();
    }

    // Precompute zero hashes per level
    let mut zeros = vec![Fr::ZERO; MERKLE_DEPTH + 1];
    for i in 1..=MERKLE_DEPTH {
        zeros[i] = hash2(zeros[i - 1], zeros[i - 1]);
    }

    let mut layer: Vec<Fr> = leaves.to_vec();
    for level in 0..MERKLE_DEPTH {
        let mut next = Vec::with_capacity((layer.len() + 1) / 2);
        let zero = zeros[level];
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
    layer[0]
}

/// Fetch leaves from indexer, append new commitments, return the new root as raw hex
pub async fn compute_new_root(
    indexer_url: &str,
    new_commitments: &[Fr],
) -> Result<String> {
    let client = reqwest::Client::new();
    let url = format!("{}/v1/leaves", indexer_url);

    let resp: serde_json::Value = client
        .get(&url)
        .send()
        .await?
        .json()
        .await
        .context("failed to fetch leaves from indexer")?;

    let leaf_hexes = resp["leaves"]
        .as_array()
        .context("invalid leaves response")?;

    let mut leaves: Vec<Fr> = leaf_hexes
        .iter()
        .map(|v| hex_to_fr(v.as_str().unwrap_or("")))
        .collect::<Result<_>>()?;

    for cm in new_commitments {
        leaves.push(*cm);
    }

    let root = compute_root(&leaves);
    Ok(fr_to_raw_hex(&root))
}

fn fr_to_raw_hex(fr: &Fr) -> String {
    crate::wallet::fr_to_raw_hex(fr)
}

/// Compute root from leaves and return as raw hex (no 0x prefix)
pub fn compute_root_from_leaves(leaves: &[Fr]) -> String {
    fr_to_raw_hex(&compute_root(leaves))
}

/// Empty root as raw hex (no 0x prefix)
pub fn empty_root_hex() -> String {
    fr_to_raw_hex(&empty_root())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_ff::UniformRand;
    use ark_std::rand::{rngs::StdRng, SeedableRng};

    #[test]
    fn empty_root_deterministic() {
        let r1 = empty_root();
        let r2 = empty_root();
        assert_eq!(r1, r2);
    }

    #[test]
    fn empty_root_hex_is_64_chars() {
        let hex = empty_root_hex();
        assert_eq!(hex.len(), 64);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn empty_root_matches_hex() {
        let root = empty_root();
        let hex = empty_root_hex();
        assert_eq!(hex, fr_to_raw_hex(&root));
    }

    #[test]
    fn compute_root_empty_equals_empty_root() {
        let from_fn = empty_root_hex();
        let from_leaves = compute_root_from_leaves(&[]);
        assert_eq!(from_fn, from_leaves);
    }

    #[test]
    fn single_leaf_root() {
        let mut rng = StdRng::seed_from_u64(77);
        let leaf = Fr::rand(&mut rng);
        let root = compute_root_from_leaves(&[leaf]);
        // should differ from empty root
        assert_ne!(root, empty_root_hex());
        // deterministic
        assert_eq!(root, compute_root_from_leaves(&[leaf]));
    }

    #[test]
    fn two_leaves_root() {
        let mut rng = StdRng::seed_from_u64(88);
        let a = Fr::rand(&mut rng);
        let b = Fr::rand(&mut rng);
        let root_ab = compute_root_from_leaves(&[a, b]);
        let root_ba = compute_root_from_leaves(&[b, a]);
        // order matters
        assert_ne!(root_ab, root_ba);
    }

    #[test]
    fn root_changes_with_extra_leaf() {
        let mut rng = StdRng::seed_from_u64(99);
        let a = Fr::rand(&mut rng);
        let b = Fr::rand(&mut rng);
        let root_1 = compute_root_from_leaves(&[a]);
        let root_2 = compute_root_from_leaves(&[a, b]);
        assert_ne!(root_1, root_2);
    }
}
