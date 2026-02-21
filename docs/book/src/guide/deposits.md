# Deposits

Create a private note and submit it on-chain.

## Step 1: Create the note

```rust
use r14_sdk::{Note, commitment};
use r14_sdk::wallet::{self, fr_to_hex, hex_to_fr, NoteEntry};

let mut w = wallet::load_wallet()?;
let owner = hex_to_fr(&w.owner_hash)?;

let mut rng = wallet::crypto_rng();
let note = Note::new(1_000, 1, owner, &mut rng);
let cm = commitment(&note);
```

**Parameters:**
- `1_000` — note value (amount)
- `1` — app tag (application identifier)
- `owner` — your owner hash (recipient of the note)

## Step 2: Save to wallet

```rust
w.notes.push(NoteEntry {
    value: note.value,
    app_tag: note.app_tag,
    owner: fr_to_hex(&note.owner),
    nonce: fr_to_hex(&note.nonce),
    commitment: fr_to_hex(&cm),
    index: None,     // set after on-chain confirmation
    spent: false,
});
wallet::save_wallet(&w)?;
```

At this point the note exists locally. If you stop here (local-only deposit), you have a record but nothing on-chain.

## Step 3: Submit on-chain

```rust
use r14_sdk::{merkle, soroban};

// compute the new merkle root with this commitment included
let new_root = merkle::compute_new_root(&w.indexer_url, &[cm]).await?;

// strip 0x prefix — Soroban expects raw hex for BytesN<32>
let cm_hex = fr_to_hex(&cm)[2..].to_string();

let result = soroban::invoke_contract(
    &w.transfer_contract_id,
    "testnet",
    &w.stellar_secret,
    "deposit",
    &[("cm", &cm_hex), ("new_root", &new_root)],
).await?;
```

## What happens on-chain

1. The `r14-transfer` contract adds the commitment to its Merkle tree
2. The root is updated to include the new leaf
3. A `deposit` event is emitted
4. The indexer picks up the event and updates its local tree

## After deposit

Sync with the indexer to get the note's on-chain leaf index (needed for spending later):

```rust
// the indexer assigns a leaf index once it sees the deposit event
// use the balance command or manually query GET /v1/leaf/{cm_hex}
```

Once `note.index` is set, the note is spendable via a private transfer.
