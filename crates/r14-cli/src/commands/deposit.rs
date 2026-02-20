use anyhow::Result;
use ark_ff::{BigInteger, PrimeField};
use r14_types::Note;

use crate::wallet::{crypto_rng, fr_to_hex, hex_to_fr, load_wallet, save_wallet, NoteEntry};

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
    let cm = r14_poseidon::commitment(&note);

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

    println!("note created (local)");
    println!("  value:      {}", value);
    println!("  app_tag:    {}", app_tag);
    println!("  commitment: {}", fr_to_hex(&cm));

    if local_only {
        println!("\n--local-only: skipping on-chain submission");
        return Ok(());
    }

    if wallet.stellar_secret == "PLACEHOLDER" || wallet.contract_id == "PLACEHOLDER" {
        println!("\nwarning: stellar_secret or contract_id not set in wallet.json");
        println!("skipping on-chain submission — set them and re-run without --local-only");
        return Ok(());
    }

    println!("\nsubmitting deposit on-chain...");
    let cm_hex = fr_to_raw_hex(&cm);
    let result = crate::soroban::invoke_contract(
        &wallet.contract_id,
        "testnet",
        &wallet.stellar_secret,
        "deposit",
        &[("cm", &cm_hex)],
    )
    .await?;

    println!("deposit submitted: {result}");
    Ok(())
}
