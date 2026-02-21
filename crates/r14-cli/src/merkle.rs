use anyhow::{Context, Result};
use ark_bls12_381::Fr;
use ark_ff::{AdditiveGroup, BigInteger, PrimeField};
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

/// Fr to raw hex (no 0x prefix) for Soroban BytesN<32>
fn fr_to_raw_hex(fr: &Fr) -> String {
    hex::encode(fr.into_bigint().to_bytes_be())
}

/// Compute root from leaves and return as raw hex (no 0x prefix)
pub fn compute_root_from_leaves(leaves: &[Fr]) -> String {
    fr_to_raw_hex(&compute_root(leaves))
}

/// Empty root as raw hex (no 0x prefix)
pub fn empty_root_hex() -> String {
    fr_to_raw_hex(&empty_root())
}
