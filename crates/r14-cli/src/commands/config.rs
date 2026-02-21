use anyhow::Result;

use crate::output;
use crate::wallet::{load_wallet, save_wallet};

const ALLOWED_KEYS: &[&str] = &[
    "rpc_url",
    "indexer_url",
    "core_contract_id",
    "transfer_contract_id",
    "stellar_secret",
];

pub fn set(key: &str, value: &str) -> Result<()> {
    if !ALLOWED_KEYS.contains(&key) {
        return Err(output::fail_with_hint(
            &format!("unknown config key: {key}"),
            &format!("allowed keys: {}", ALLOWED_KEYS.join(", ")),
        ));
    }

    let mut wallet = load_wallet()?;
    match key {
        "rpc_url" => wallet.rpc_url = value.to_string(),
        "indexer_url" => wallet.indexer_url = value.to_string(),
        "core_contract_id" => wallet.core_contract_id = value.to_string(),
        "transfer_contract_id" => wallet.transfer_contract_id = value.to_string(),
        "stellar_secret" => wallet.stellar_secret = value.to_string(),
        _ => unreachable!(),
    }
    save_wallet(&wallet)?;

    if output::is_json() {
        output::json_output(serde_json::json!({ "key": key, "value": value }));
    } else {
        output::success(&format!("{key} updated"));
    }
    Ok(())
}

fn mask(s: &str) -> String {
    if s.len() <= 8 || s == "PLACEHOLDER" {
        return s.to_string();
    }
    format!("{}***{}", &s[..4], &s[s.len() - 4..])
}

pub fn show() -> Result<()> {
    let wallet = load_wallet()?;

    if output::is_json() {
        output::json_output(serde_json::json!({
            "secret_key": mask(&wallet.secret_key),
            "owner_hash": wallet.owner_hash,
            "stellar_secret": mask(&wallet.stellar_secret),
            "rpc_url": wallet.rpc_url,
            "indexer_url": wallet.indexer_url,
            "core_contract_id": wallet.core_contract_id,
            "transfer_contract_id": wallet.transfer_contract_id,
            "notes_count": wallet.notes.len(),
        }));
    } else {
        output::label("secret_key", &mask(&wallet.secret_key));
        output::label("owner_hash", &wallet.owner_hash);
        output::label("stellar_secret", &mask(&wallet.stellar_secret));
        output::label("rpc_url", &wallet.rpc_url);
        output::label("indexer_url", &wallet.indexer_url);
        output::label("core_contract_id", &wallet.core_contract_id);
        output::label("transfer_contract_id", &wallet.transfer_contract_id);
        output::label("notes", &wallet.notes.len().to_string());
    }
    Ok(())
}
