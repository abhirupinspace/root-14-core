use anyhow::{Context, Result};
use ark_bls12_381::Fr;
use r14_types::{MerklePath, Note};
use serde::Deserialize;

use crate::wallet::{crypto_rng, fr_to_hex, hex_to_fr, load_wallet, save_wallet, NoteEntry};

#[derive(Deserialize)]
struct ProofResponse {
    siblings: Vec<String>,
    indices: Vec<bool>,
}

#[derive(Deserialize)]
struct RootResponse {
    #[allow(dead_code)]
    root: String,
}

pub async fn run(value: u64, recipient_hex: &str, dry_run: bool) -> Result<()> {
    let mut wallet = load_wallet()?;
    let sk_fr = hex_to_fr(&wallet.secret_key)?;
    let owner_fr = hex_to_fr(&wallet.owner_hash)?;
    let recipient_fr = hex_to_fr(recipient_hex)?;

    // find unspent note with sufficient value and on-chain index
    let note_idx = wallet
        .notes
        .iter()
        .position(|n| !n.spent && n.value >= value && n.index.is_some())
        .context("no unspent on-chain note with sufficient value")?;

    let entry = &wallet.notes[note_idx];
    let consumed = Note::with_nonce(
        entry.value,
        entry.app_tag,
        hex_to_fr(&entry.owner)?,
        hex_to_fr(&entry.nonce)?,
    );
    let leaf_index = entry.index.unwrap();
    let app_tag = entry.app_tag;
    let consumed_value = entry.value;

    let client = reqwest::Client::new();

    // fetch merkle proof
    let proof_url = format!("{}/v1/proof/{}", wallet.indexer_url, leaf_index);
    let proof_resp: ProofResponse = client
        .get(&proof_url)
        .send()
        .await?
        .json()
        .await
        .context("failed to parse merkle proof")?;

    let siblings: Vec<Fr> = proof_resp
        .siblings
        .iter()
        .map(|s| hex_to_fr(s))
        .collect::<Result<_>>()?;
    let merkle_path = MerklePath {
        siblings,
        indices: proof_resp.indices,
    };

    // fetch root (for verification context)
    let root_url = format!("{}/v1/root", wallet.indexer_url);
    let _root_resp: RootResponse = client
        .get(&root_url)
        .send()
        .await?
        .json()
        .await
        .context("failed to parse root")?;

    // build output notes
    let mut rng = crypto_rng();
    let change = consumed_value - value;
    let note_0 = Note::new(value, app_tag, recipient_fr, &mut rng);
    let note_1 = Note::new(change, app_tag, owner_fr, &mut rng);

    // prove
    println!("generating proof (this may take a few seconds)...");
    let (pk, _vk) = r14_circuit::setup(&mut rng);
    let (proof, pi) = r14_circuit::prove(
        &pk,
        sk_fr,
        consumed.clone(),
        merkle_path,
        [note_0.clone(), note_1.clone()],
        &mut rng,
    );

    let (serialized_proof, serialized_pi) =
        r14_circuit::serialize_proof_for_soroban(&proof, &pi);

    let cm_0 = r14_poseidon::commitment(&note_0);
    let cm_1 = r14_poseidon::commitment(&note_1);

    if dry_run {
        let output = serde_json::json!({
            "proof": {
                "a": serialized_proof.a,
                "b": serialized_proof.b,
                "c": serialized_proof.c,
            },
            "public_inputs": serialized_pi,
            "nullifier": fr_to_hex(&pi.nullifier),
            "out_commitment_0": fr_to_hex(&cm_0),
            "out_commitment_1": fr_to_hex(&cm_1),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        // TODO: submit to Soroban via JSON-RPC (simulateTransaction + sendTransaction)
        println!("soroban submission not yet implemented — use --dry-run");
        println!("nullifier:        {}", fr_to_hex(&pi.nullifier));
        println!("out_commitment_0: {}", fr_to_hex(&cm_0));
        println!("out_commitment_1: {}", fr_to_hex(&cm_1));
    }

    // update wallet: mark consumed as spent, add output notes
    wallet.notes[note_idx].spent = true;

    wallet.notes.push(NoteEntry {
        value: note_0.value,
        app_tag: note_0.app_tag,
        owner: fr_to_hex(&note_0.owner),
        nonce: fr_to_hex(&note_0.nonce),
        commitment: fr_to_hex(&cm_0),
        index: None,
        spent: false,
    });

    wallet.notes.push(NoteEntry {
        value: note_1.value,
        app_tag: note_1.app_tag,
        owner: fr_to_hex(&note_1.owner),
        nonce: fr_to_hex(&note_1.nonce),
        commitment: fr_to_hex(&cm_1),
        index: None,
        spent: false,
    });

    save_wallet(&wallet)?;
    println!("wallet updated");
    Ok(())
}
