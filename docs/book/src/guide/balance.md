# Balance & Sync

Check your private balance and sync note status with the indexer.

## Check balance

```rust
use r14_sdk::wallet;

let w = wallet::load_wallet()?;
let unspent: Vec<_> = w.notes.iter().filter(|n| !n.spent).collect();
let balance: u64 = unspent.iter().map(|n| n.value).sum();

println!("balance: {}", balance);
for (i, n) in unspent.iter().enumerate() {
    let status = match n.index {
        Some(idx) => format!("on-chain (idx={})", idx),
        None => "local-only".to_string(),
    };
    println!("  [{}] value={} app_tag={} {}", i, n.value, n.app_tag, status);
}
```

## Sync with indexer

Notes created locally don't have an on-chain leaf index until the indexer confirms the deposit event. Query the indexer to update:

```rust
let client = reqwest::Client::new();

for note in w.notes.iter_mut().filter(|n| !n.spent && n.index.is_none()) {
    let cm_hex = note.commitment.strip_prefix("0x").unwrap_or(&note.commitment);
    let url = format!("{}/v1/leaf/{}", w.indexer_url, cm_hex);

    if let Ok(resp) = client.get(&url).send().await {
        if resp.status().is_success() {
            if let Ok(leaf) = resp.json::<serde_json::Value>().await {
                note.index = leaf["index"].as_u64();
            }
        }
    }
}

wallet::save_wallet(&w)?;
```

## Note states

| State | `index` | `spent` | Meaning |
|-------|---------|---------|---------|
| Local-only | `None` | `false` | Created but not yet confirmed on-chain |
| On-chain | `Some(n)` | `false` | Confirmed, spendable |
| Spent | any | `true` | Consumed in a transfer |
