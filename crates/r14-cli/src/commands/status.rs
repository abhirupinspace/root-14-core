use anyhow::Result;
use colored::Colorize;

use crate::output;
use crate::wallet::{load_wallet, wallet_path};

pub async fn run() -> Result<()> {
    let path = wallet_path()?;
    let wallet_loaded = path.exists();

    if !wallet_loaded {
        if output::is_json() {
            output::json_output(serde_json::json!({
                "wallet_loaded": false,
                "contracts_configured": false,
                "indexer_reachable": false,
                "notes_total": 0,
                "notes_synced": 0,
            }));
        } else {
            output::label("wallet", &"not found".red().to_string());
            output::info("run `r14 keygen` to create a wallet");
        }
        return Ok(());
    }

    let wallet = load_wallet()?;

    let contracts_configured = wallet.stellar_secret != "PLACEHOLDER"
        && wallet.core_contract_id != "PLACEHOLDER"
        && wallet.transfer_contract_id != "PLACEHOLDER";

    // ping indexer
    let indexer_reachable = reqwest::Client::new()
        .get(format!("{}/v1/root", wallet.indexer_url))
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);

    let unspent: Vec<_> = wallet.notes.iter().filter(|n| !n.spent).collect();
    let notes_total = unspent.len();
    let notes_synced = unspent.iter().filter(|n| n.index.is_some()).count();

    if output::is_json() {
        output::json_output(serde_json::json!({
            "wallet_loaded": true,
            "contracts_configured": contracts_configured,
            "indexer_reachable": indexer_reachable,
            "notes_total": notes_total,
            "notes_synced": notes_synced,
        }));
    } else {
        output::label("wallet", &"loaded".green().to_string());
        let contracts_str = if contracts_configured {
            "configured".green().to_string()
        } else {
            "missing PLACEHOLDERs".yellow().to_string()
        };
        output::label("contracts", &contracts_str);
        let indexer_str = if indexer_reachable {
            format!("{} ({})", "reachable".green(), wallet.indexer_url)
        } else {
            format!("{} ({})", "unreachable".red(), wallet.indexer_url)
        };
        output::label("indexer", &indexer_str);
        output::label("notes", &format!("{notes_total} total, {notes_synced} synced"));
    }

    Ok(())
}
