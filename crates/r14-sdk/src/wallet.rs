// Copyright 2026 abhirupbanerjee
// Licensed under the Apache License, Version 2.0

//! Wallet persistence and field-element ↔ hex conversion.
//!
//! Stores keys, notes, and config as JSON at `~/.r14/wallet.json`.
//!
//! # Hex format
//!
//! [`fr_to_hex`] produces `0x`-prefixed big-endian hex (66 chars).
//! [`hex_to_fr`] accepts both `0x`-prefixed and raw hex, and zero-pads
//! short inputs to 32 bytes.
//!
//! # Example
//!
//! ```rust,no_run
//! use r14_sdk::wallet::{load_wallet, save_wallet, fr_to_hex, hex_to_fr};
//!
//! # fn example() -> anyhow::Result<()> {
//! let mut w = load_wallet()?;
//! let owner_fr = hex_to_fr(&w.owner_hash)?;
//! // ... use owner_fr in note creation ...
//! save_wallet(&w)?;
//! # Ok(())
//! # }
//! ```

use anyhow::{Context, Result};
use ark_bls12_381::Fr;
use ark_ff::{BigInteger, PrimeField};
use ark_std::rand::{rngs::StdRng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub fn crypto_rng() -> StdRng {
    StdRng::seed_from_u64(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64,
    )
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WalletData {
    pub secret_key: String,
    pub owner_hash: String,
    pub stellar_secret: String,
    pub notes: Vec<NoteEntry>,
    pub indexer_url: String,
    pub rpc_url: String,
    pub core_contract_id: String,
    pub transfer_contract_id: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NoteEntry {
    pub value: u64,
    pub app_tag: u32,
    pub owner: String,
    pub nonce: String,
    pub commitment: String,
    pub index: Option<u64>,
    pub spent: bool,
}

pub fn wallet_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("cannot determine home directory")?;
    Ok(home.join(".r14").join("wallet.json"))
}

pub fn load_wallet() -> Result<WalletData> {
    let path = wallet_path()?;
    let data = fs::read_to_string(&path)
        .with_context(|| format!("cannot read wallet at {}", path.display()))?;
    serde_json::from_str(&data).context("invalid wallet JSON")
}

pub fn save_wallet(wallet: &WalletData) -> Result<()> {
    let path = wallet_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(wallet)?;
    fs::write(&path, json)?;
    Ok(())
}

pub fn fr_to_hex(fr: &Fr) -> String {
    let bigint = fr.into_bigint();
    let bytes = bigint.to_bytes_be();
    format!("0x{}", hex::encode(bytes))
}

pub fn hex_to_fr(s: &str) -> Result<Fr> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    let bytes = hex::decode(s).context("invalid hex")?;
    // pad to 32 bytes
    let mut padded = vec![0u8; 32 - bytes.len().min(32)];
    padded.extend_from_slice(&bytes[..bytes.len().min(32)]);
    let bigint = <Fr as PrimeField>::BigInt::try_from(
        ark_ff::BigInt::<4>::new({
            let mut limbs = [0u64; 4];
            // BE bytes -> LE limbs
            for (i, chunk) in padded.rchunks(8).enumerate() {
                if i >= 4 { break; }
                let mut buf = [0u8; 8];
                let start = 8 - chunk.len();
                buf[start..].copy_from_slice(chunk);
                limbs[i] = u64::from_be_bytes(buf);
            }
            limbs
        })
    ).unwrap();
    Fr::from_bigint(bigint).context("value not in field")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_ff::UniformRand;
    use ark_std::rand::{rngs::StdRng, SeedableRng};

    #[test]
    fn fr_hex_roundtrip() {
        let mut rng = StdRng::seed_from_u64(99);
        for _ in 0..10 {
            let original = Fr::rand(&mut rng);
            let hex = fr_to_hex(&original);
            let recovered = hex_to_fr(&hex).unwrap();
            assert_eq!(original, recovered);
        }
    }

    #[test]
    fn hex_to_fr_no_prefix() {
        let mut rng = StdRng::seed_from_u64(42);
        let val = Fr::rand(&mut rng);
        let hex_with = fr_to_hex(&val); // "0x..."
        let hex_without = hex_with.strip_prefix("0x").unwrap();
        assert_eq!(hex_to_fr(&hex_with).unwrap(), hex_to_fr(hex_without).unwrap());
    }

    #[test]
    fn hex_to_fr_zero() {
        let fr = hex_to_fr("0x0000000000000000000000000000000000000000000000000000000000000000").unwrap();
        assert_eq!(fr, Fr::from(0u64));
    }

    #[test]
    fn hex_to_fr_one() {
        let fr = hex_to_fr("0x0000000000000000000000000000000000000000000000000000000000000001").unwrap();
        assert_eq!(fr, Fr::from(1u64));
    }

    #[test]
    fn hex_to_fr_short_input() {
        // short hex (< 32 bytes) should be zero-padded
        let fr = hex_to_fr("01").unwrap();
        assert_eq!(fr, Fr::from(1u64));
    }

    #[test]
    fn fr_to_hex_has_0x_prefix() {
        let hex = fr_to_hex(&Fr::from(42u64));
        assert!(hex.starts_with("0x"));
        assert_eq!(hex.len(), 66); // "0x" + 64 hex chars
    }
}
