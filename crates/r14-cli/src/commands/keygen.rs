use anyhow::Result;
use r14_types::SecretKey;

use crate::wallet::{crypto_rng, fr_to_hex, save_wallet, wallet_path, WalletData};

pub fn run() -> Result<()> {
    let path = wallet_path()?;
    if path.exists() {
        anyhow::bail!("wallet already exists at {}\ndelete it first to regenerate", path.display());
    }

    let mut rng = crypto_rng();
    let sk = SecretKey::random(&mut rng);
    let owner = r14_poseidon::owner_hash(&sk);

    let wallet = WalletData {
        secret_key: fr_to_hex(&sk.0),
        owner_hash: fr_to_hex(&owner.0),
        stellar_secret: "PLACEHOLDER".into(),
        notes: vec![],
        indexer_url: "http://localhost:3000".into(),
        rpc_url: "https://soroban-testnet.stellar.org:443".into(),
        contract_id: "PLACEHOLDER".into(),
    };

    save_wallet(&wallet)?;
    println!("wallet created at {}", path.display());
    println!("owner_hash: {}", wallet.owner_hash);
    println!("\nnote: edit wallet.json to set stellar_secret and contract_id");
    Ok(())
}
