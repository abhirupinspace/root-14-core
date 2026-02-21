# Keygen

Generate a secret key, derive the owner hash, and persist a new wallet.

```rust
use r14_sdk::{SecretKey, owner_hash};
use r14_sdk::wallet::{self, fr_to_hex, WalletData};

fn create_wallet() -> anyhow::Result<()> {
    let mut rng = wallet::crypto_rng();
    let sk = SecretKey::random(&mut rng);
    let owner = owner_hash(&sk);

    let w = WalletData {
        secret_key: fr_to_hex(&sk.0),
        owner_hash: fr_to_hex(&owner.0),
        stellar_secret: "S_YOUR_STELLAR_SECRET".into(),
        notes: vec![],
        indexer_url: "http://localhost:3000".into(),
        rpc_url: "https://soroban-testnet.stellar.org:443".into(),
        core_contract_id: "C_CORE_CONTRACT_ID".into(),
        transfer_contract_id: "C_TRANSFER_CONTRACT_ID".into(),
    };

    wallet::save_wallet(&w)?;
    println!("owner_hash: {}", w.owner_hash);
    Ok(())
}
```

## What gets stored

The wallet is saved as JSON at `~/.r14/wallet.json`:

```json
{
  "secret_key": "0x...",
  "owner_hash": "0x...",
  "stellar_secret": "S_YOUR_STELLAR_SECRET",
  "notes": [],
  "indexer_url": "http://localhost:3000",
  "rpc_url": "https://soroban-testnet.stellar.org:443",
  "core_contract_id": "C_CORE_CONTRACT_ID",
  "transfer_contract_id": "C_TRANSFER_CONTRACT_ID"
}
```

## Sharing your identity

Share your `owner_hash` with anyone who wants to send you private notes. It is a one-way Poseidon hash of your secret key — it cannot be reversed.

Never share your `secret_key` or `stellar_secret`.
