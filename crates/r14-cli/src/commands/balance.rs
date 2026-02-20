use anyhow::{Context, Result};
use serde::Deserialize;

use crate::wallet::{load_wallet, save_wallet};

#[derive(Deserialize)]
struct LeafResponse {
    index: u64,
    #[allow(dead_code)]
    block_height: u64,
}

pub async fn run() -> Result<()> {
    let mut wallet = load_wallet()?;
    let client = reqwest::Client::new();

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

    save_wallet(&wallet).context("failed to save wallet after sync")?;

    // display
    let unspent: Vec<_> = wallet.notes.iter().filter(|n| !n.spent).collect();
    let total: u64 = unspent.iter().map(|n| n.value).sum();

    println!("balance: {}", total);
    if !unspent.is_empty() {
        println!("\nunspent notes:");
        for (i, n) in unspent.iter().enumerate() {
            let status = match n.index {
                Some(idx) => format!("on-chain (idx={})", idx),
                None => "local-only".into(),
            };
            println!("  [{}] value={} app_tag={} {}", i, n.value, n.app_tag, status);
        }
    }

    Ok(())
}
