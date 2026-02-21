use anyhow::{Context, Result};
use colored::Colorize;
use serde::Deserialize;

use crate::output;
use r14_sdk::wallet::{load_wallet, save_wallet};

#[derive(Deserialize)]
struct LeafResponse {
    index: u64,
    #[allow(dead_code)]
    block_height: u64,
}

pub async fn run() -> Result<()> {
    let mut wallet = load_wallet()?;
    let client = reqwest::Client::new();

    let sp = output::spinner("syncing notes with indexer...");

    // sync unspent notes with indexer
    for note in wallet.notes.iter_mut().filter(|n| !n.spent) {
        if note.index.is_some() {
            continue;
        }
        let cm_hex = note.commitment.strip_prefix("0x").unwrap_or(&note.commitment);
        let url = format!("{}/v1/leaf/{}", wallet.indexer_url, cm_hex);
        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(leaf) = resp.json::<LeafResponse>().await {
                    note.index = Some(leaf.index);
                }
            }
            _ => {} // indexer unreachable or commitment not on-chain yet
        }
    }

    sp.finish_and_clear();
    save_wallet(&wallet).context("failed to save wallet after sync")?;

    // display
    let unspent: Vec<_> = wallet.notes.iter().filter(|n| !n.spent).collect();
    let total: u64 = unspent.iter().map(|n| n.value).sum();

    if output::is_json() {
        let notes_json: Vec<_> = unspent
            .iter()
            .map(|n| {
                serde_json::json!({
                    "value": n.value,
                    "app_tag": n.app_tag,
                    "commitment": n.commitment,
                    "index": n.index,
                    "status": if n.index.is_some() { "on-chain" } else { "local-only" },
                })
            })
            .collect();
        output::json_output(serde_json::json!({
            "balance": total,
            "notes": notes_json,
        }));
    } else {
        output::label("balance", &total.to_string());
        if !unspent.is_empty() {
            output::info("\nunspent notes:");
            for (i, n) in unspent.iter().enumerate() {
                let status = match n.index {
                    Some(idx) => format!("{} (idx={})", "on-chain".green(), idx),
                    None => "local-only".yellow().to_string(),
                };
                output::info(&format!("  [{}] value={} app_tag={} {}", i, n.value, n.app_tag, status));
            }
        }
    }

    Ok(())
}
