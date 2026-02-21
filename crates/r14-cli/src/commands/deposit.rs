use anyhow::Result;
use ark_ff::{BigInteger, PrimeField};
use r14_sdk::{commitment, Note};
use r14_sdk::wallet::{crypto_rng, fr_to_hex, hex_to_fr, load_wallet, save_wallet, NoteEntry};

use crate::output;

/// Convert Fr to raw hex (no 0x prefix) for stellar CLI BytesN<32>
fn fr_to_raw_hex(fr: &ark_bls12_381::Fr) -> String {
    let bytes = fr.into_bigint().to_bytes_be();
    hex::encode(bytes)
}

pub async fn run(value: u64, app_tag: u32, local_only: bool) -> Result<()> {
    let mut wallet = load_wallet()?;
    let owner = hex_to_fr(&wallet.owner_hash)?;

    let mut rng = crypto_rng();
    let note = Note::new(value, app_tag, owner, &mut rng);
    let cm = commitment(&note);

    let entry = NoteEntry {
        value: note.value,
        app_tag: note.app_tag,
        owner: fr_to_hex(&note.owner),
        nonce: fr_to_hex(&note.nonce),
        commitment: fr_to_hex(&cm),
        index: None,
        spent: false,
    };

    wallet.notes.push(entry);
    save_wallet(&wallet)?;

    let cm_hex_display = fr_to_hex(&cm);

    if local_only {
        if output::is_json() {
            output::json_output(serde_json::json!({
                "value": value,
                "app_tag": app_tag,
                "commitment": cm_hex_display,
                "on_chain": false,
            }));
        } else {
            output::success("note created (local)");
            output::label("value", &value.to_string());
            output::label("app_tag", &app_tag.to_string());
            output::label("commitment", &cm_hex_display);
            output::info("--local-only: skipping on-chain submission");
        }
        return Ok(());
    }

    // validation now in main.rs, but keep guard for direct calls
    if wallet.stellar_secret == "PLACEHOLDER" || wallet.transfer_contract_id == "PLACEHOLDER" {
        output::warn("stellar_secret or transfer_contract_id not set — skipping on-chain");
        if output::is_json() {
            output::json_output(serde_json::json!({
                "value": value,
                "app_tag": app_tag,
                "commitment": cm_hex_display,
                "on_chain": false,
            }));
        }
        return Ok(());
    }

    let cm_hex = fr_to_raw_hex(&cm);

    let sp = output::spinner("computing new merkle root...");
    let new_root_hex = r14_sdk::merkle::compute_new_root(&wallet.indexer_url, &[cm]).await?;
    sp.finish_and_clear();

    let sp = output::spinner("submitting deposit on-chain...");
    let result = r14_sdk::soroban::invoke_contract(
        &wallet.transfer_contract_id,
        "testnet",
        &wallet.stellar_secret,
        "deposit",
        &[("cm", &cm_hex), ("new_root", &new_root_hex)],
    )
    .await?;
    sp.finish_and_clear();

    if output::is_json() {
        output::json_output(serde_json::json!({
            "value": value,
            "app_tag": app_tag,
            "commitment": cm_hex_display,
            "on_chain": true,
            "result": result,
        }));
    } else {
        output::success("deposit submitted");
        output::label("value", &value.to_string());
        output::label("commitment", &cm_hex_display);
        output::label("tx", &result);
    }
    Ok(())
}
