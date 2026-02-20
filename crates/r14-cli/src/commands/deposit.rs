use anyhow::Result;
use r14_types::Note;

use crate::wallet::{crypto_rng, fr_to_hex, hex_to_fr, load_wallet, save_wallet, NoteEntry};

pub fn run(value: u64, app_tag: u32) -> Result<()> {
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
    println!("\nsubmit this commitment on-chain to finalize the deposit");
    Ok(())
}
