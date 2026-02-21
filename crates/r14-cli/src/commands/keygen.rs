use anyhow::Result;
use r14_types::SecretKey;

use crate::output;
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
        core_contract_id: "PLACEHOLDER".into(),
        transfer_contract_id: "PLACEHOLDER".into(),
    };

    save_wallet(&wallet)?;

    if output::is_json() {
        output::json_output(serde_json::json!({
            "wallet_path": path.display().to_string(),
            "owner_hash": wallet.owner_hash,
        }));
    } else {
        output::success(&format!("wallet created at {}", path.display()));
        output::label("owner_hash", &wallet.owner_hash);
        output::warn("run `r14 config set stellar_secret <SECRET>` to configure");
    }
    Ok(())
}
