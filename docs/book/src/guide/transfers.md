# Private Transfers

Execute a private transfer with a ZK proof. This requires both `r14-sdk` and `r14-circuit`.

## Overview

A private transfer consumes one note and creates two output notes (recipient + change). The ZK proof ensures value conservation and ownership without revealing amounts.

## Step 1: Load wallet and find a spendable note

```rust
use r14_sdk::{Note, MerklePath, commitment};
use r14_sdk::wallet::{crypto_rng, fr_to_hex, hex_to_fr, load_wallet, save_wallet, NoteEntry};

let mut w = load_wallet()?;
let sk_fr = hex_to_fr(&w.secret_key)?;
let owner_fr = hex_to_fr(&w.owner_hash)?;

// find an unspent note with sufficient value and an on-chain index
let entry = w.notes.iter()
    .find(|n| !n.spent && n.value >= amount && n.index.is_some())
    .expect("no spendable note with sufficient value");

let consumed = Note::with_nonce(
    entry.value, entry.app_tag,
    hex_to_fr(&entry.owner)?, hex_to_fr(&entry.nonce)?,
);
let leaf_index = entry.index.unwrap();
```

## Step 2: Fetch Merkle proof from indexer

```rust
let client = reqwest::Client::new();
let proof_url = format!("{}/v1/proof/{}", w.indexer_url, leaf_index);

// response: { "siblings": ["0x...", ...], "indices": [true, false, ...] }
let resp: serde_json::Value = client.get(&proof_url).send().await?.json().await?;

let siblings: Vec<ark_bls12_381::Fr> = resp["siblings"]
    .as_array().unwrap()
    .iter()
    .map(|s| hex_to_fr(s.as_str().unwrap()))
    .collect::<anyhow::Result<_>>()?;

let indices: Vec<bool> = resp["indices"]
    .as_array().unwrap()
    .iter()
    .map(|v| v.as_bool().unwrap())
    .collect();

let merkle_path = MerklePath { siblings, indices };
```

## Step 3: Build output notes

```rust
let mut rng = crypto_rng();
let change = entry.value - amount;

// note for recipient
let note_out = Note::new(amount, entry.app_tag, recipient_owner_fr, &mut rng);
// change note back to sender
let note_change = Note::new(change, entry.app_tag, owner_fr, &mut rng);
```

## Step 4: Generate ZK proof

```rust
use ark_std::rand::{rngs::StdRng, SeedableRng};

// deterministic setup — same seed=42 must match what was used during contract init
let setup_rng = &mut StdRng::seed_from_u64(42);
let (pk, _vk) = r14_circuit::setup(setup_rng);

let (proof, pi) = r14_circuit::prove(
    &pk, sk_fr, consumed, merkle_path,
    [note_out.clone(), note_change.clone()],
    &mut rng,
);
```

> **Important**: The setup seed (42) must match what was used during `r14 init-contract`. Using a different seed produces a different proving/verifying key pair and proofs will fail verification.

## Step 5: Serialize and submit

```rust
let (sp, spi) = r14_circuit::serialize_proof_for_soroban(&proof, &pi);
let cm_0 = commitment(&note_out);
let cm_1 = commitment(&note_change);

let new_root = r14_sdk::merkle::compute_new_root(
    &w.indexer_url, &[cm_0, cm_1],
).await?;

let proof_json = format!(
    r#"{{"a":"{}","b":"{}","c":"{}"}}"#,
    sp.a, sp.b, sp.c
);

r14_sdk::soroban::invoke_contract(
    &w.transfer_contract_id, "testnet", &w.stellar_secret,
    "transfer",
    &[
        ("proof", &proof_json),
        ("old_root", &spi[0]),
        ("nullifier", &spi[1]),
        ("cm_0", &spi[2]),
        ("cm_1", &spi[3]),
        ("new_root", &new_root),
    ],
).await?;
```

## Step 6: Update wallet

```rust
// mark consumed note as spent
// (find the index of the note you consumed)
w.notes[consumed_idx].spent = true;

// add output notes
w.notes.push(NoteEntry {
    value: note_out.value,
    app_tag: note_out.app_tag,
    owner: fr_to_hex(&note_out.owner),
    nonce: fr_to_hex(&note_out.nonce),
    commitment: fr_to_hex(&cm_0),
    index: None,
    spent: false,
});

w.notes.push(NoteEntry {
    value: note_change.value,
    app_tag: note_change.app_tag,
    owner: fr_to_hex(&note_change.owner),
    nonce: fr_to_hex(&note_change.nonce),
    commitment: fr_to_hex(&cm_1),
    index: None,
    spent: false,
});

save_wallet(&w)?;
```

## What the ZK proof guarantees

The circuit enforces all of these without revealing any private data:

1. Prover knows `sk` matching the consumed note's `owner`
2. The consumed note exists in the Merkle tree (valid path to `old_root`)
3. `consumed.value == output_0.value + output_1.value`
4. All notes share the same `app_tag`
5. Nullifier is correctly derived from `sk` and `nonce`
6. Output commitments are correctly computed
