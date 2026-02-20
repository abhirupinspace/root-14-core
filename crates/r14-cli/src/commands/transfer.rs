use anyhow::{Context, Result};
use ark_bls12_381::Fr;
use r14_types::{MerklePath, Note};
use serde::Deserialize;

use ark_std::rand::{rngs::StdRng, SeedableRng};

use crate::wallet::{crypto_rng, fr_to_hex, hex_to_fr, load_wallet, save_wallet, NoteEntry};

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
    println!("generating proof (this may take a few seconds)...");
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
        return Ok(());
    } else {
        if wallet.stellar_secret == "PLACEHOLDER" || wallet.contract_id == "PLACEHOLDER" {
            anyhow::bail!("stellar_secret or contract_id not set in wallet.json");
        }

        println!("submitting transfer on-chain...");

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

        let result = crate::soroban::invoke_contract(
            &wallet.contract_id,
            "testnet",
            &wallet.stellar_secret,
            "transfer",
            &[
                ("proof", &proof_json),
                ("old_root", &old_root_hex),
                ("nullifier", &nullifier_hex),
                ("cm_0", &cm_0_hex),
                ("cm_1", &cm_1_hex),
            ],
        )
        .await?;

        println!("transfer submitted: {result}");
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
