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
    pub contract_id: String,
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
