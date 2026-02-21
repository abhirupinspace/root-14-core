use anyhow::{Context, Result};
use ark_bls12_381::Fr;
use r14_sdk::{commitment, MerklePath, Note};
use r14_sdk::wallet::{crypto_rng, fr_to_hex, hex_to_fr, load_wallet, save_wallet, NoteEntry};
use serde::Deserialize;

use ark_std::rand::{rngs::StdRng, SeedableRng};

use crate::output;

fn strip_0x(s: &str) -> String {
    s.strip_prefix("0x").unwrap_or(s).to_string()
}

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

    // prove — deterministic seed for setup so pk matches on-chain vk
    let sp = output::spinner("generating proof (this may take a few seconds)...");
    let setup_rng = &mut StdRng::seed_from_u64(42);
    let (pk, _vk) = r14_circuit::setup(setup_rng);
    let (proof, pi) = r14_circuit::prove(
        &pk,
        sk_fr,
        consumed.clone(),
        merkle_path,
        [note_0.clone(), note_1.clone()],
        &mut rng,
    );
    sp.finish_and_clear();

    let (serialized_proof, serialized_pi) =
        r14_circuit::serialize_proof_for_soroban(&proof, &pi);

    let cm_0 = commitment(&note_0);
    let cm_1 = commitment(&note_1);

    if dry_run {
        let dry_output = serde_json::json!({
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
        if output::is_json() {
            output::json_output(dry_output);
        } else {
            println!("{}", serde_json::to_string_pretty(&dry_output)?);
        }
        return Ok(());
    }

    // validation now in main.rs, but keep guard for direct calls
    if wallet.stellar_secret == "PLACEHOLDER" || wallet.transfer_contract_id == "PLACEHOLDER" {
        return Err(output::fail_with_hint(
            "stellar_secret or transfer_contract_id not set",
            "run `r14 config set <key> <value>`",
        ));
    }

    // Build proof JSON for Soroban contracttype Proof { a: G1Affine, b: G2Affine, c: G1Affine }
    let proof_json = format!(
        r#"{{"a":"{}","b":"{}","c":"{}"}}"#,
        serialized_proof.a, serialized_proof.b, serialized_proof.c
    );

    // Public inputs: old_root, nullifier, cm_0, cm_1 as hex (no 0x prefix)
    let old_root_hex = strip_0x(&serialized_pi[0]);
    let nullifier_hex = strip_0x(&serialized_pi[1]);
    let cm_0_hex = strip_0x(&serialized_pi[2]);
    let cm_1_hex = strip_0x(&serialized_pi[3]);

    let sp = output::spinner("computing new merkle root...");
    let new_root_hex = r14_sdk::merkle::compute_new_root(
        &wallet.indexer_url,
        &[cm_0, cm_1],
    )
    .await?;
    sp.finish_and_clear();

    let sp = output::spinner("submitting transfer on-chain...");
    let result = r14_sdk::soroban::invoke_contract(
        &wallet.transfer_contract_id,
        "testnet",
        &wallet.stellar_secret,
        "transfer",
        &[
            ("proof", &proof_json),
            ("old_root", &old_root_hex),
            ("nullifier", &nullifier_hex),
            ("cm_0", &cm_0_hex),
            ("cm_1", &cm_1_hex),
            ("new_root", &new_root_hex),
        ],
    )
    .await?;
    sp.finish_and_clear();

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

    if output::is_json() {
        output::json_output(serde_json::json!({
            "value": value,
            "recipient": recipient_hex,
            "nullifier": fr_to_hex(&pi.nullifier),
            "out_commitment_0": fr_to_hex(&cm_0),
            "out_commitment_1": fr_to_hex(&cm_1),
            "result": result,
        }));
    } else {
        output::success("transfer submitted");
        output::label("value", &value.to_string());
        output::label("nullifier", &fr_to_hex(&pi.nullifier));
        output::label("tx", &result);
    }
    Ok(())
}
