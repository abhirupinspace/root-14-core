use anyhow::Result;
use ark_std::rand::{rngs::StdRng, SeedableRng};

use crate::output;
use r14_sdk::wallet::load_wallet;

pub async fn run() -> Result<()> {
    let wallet = load_wallet()?;

    // validation now in main.rs, but keep guard for direct calls
    if wallet.stellar_secret == "PLACEHOLDER"
        || wallet.core_contract_id == "PLACEHOLDER"
        || wallet.transfer_contract_id == "PLACEHOLDER"
    {
        return Err(output::fail_with_hint(
            "stellar_secret, core_contract_id, or transfer_contract_id not set",
            "run `r14 config set <key> <value>`",
        ));
    }

    // Deterministic setup — same seed=42 used everywhere
    let sp = output::spinner("setting up circuit...");
    let mut rng = StdRng::seed_from_u64(42);
    let (_pk, vk) = r14_circuit::setup(&mut rng);
    sp.finish_and_clear();

    let svk = r14_circuit::serialize_vk_for_soroban(&vk);

    // Build VK JSON matching the Soroban contract's unified IC format
    let ic_entries: Vec<String> = svk.ic.iter().map(|s| format!("\"{}\"", s)).collect();
    let vk_json = format!(
        r#"{{"alpha_g1":"{}","beta_g2":"{}","gamma_g2":"{}","delta_g2":"{}","ic":[{}]}}"#,
        svk.alpha_g1, svk.beta_g2, svk.gamma_g2, svk.delta_g2, ic_entries.join(",")
    );

    // Derive caller address from stellar secret
    let caller_address = r14_sdk::soroban::get_public_key(&wallet.stellar_secret).await?;

    // Step 1: Register VK on r14-core
    let sp = output::spinner("registering VK on r14-core...");
    let circuit_id = r14_sdk::soroban::invoke_contract(
        &wallet.core_contract_id,
        "testnet",
        &wallet.stellar_secret,
        "register",
        &[("caller", &caller_address), ("vk", &vk_json)],
    )
    .await?;
    sp.finish_and_clear();

    output::info(&format!("VK registered, circuit_id: {circuit_id}"));

    // Step 2: Initialize r14-transfer with core address, circuit_id, empty root
    let empty_root_hex = r14_sdk::merkle::empty_root_hex();

    let sp = output::spinner("initializing r14-transfer...");
    let result = r14_sdk::soroban::invoke_contract(
        &wallet.transfer_contract_id,
        "testnet",
        &wallet.stellar_secret,
        "init",
        &[
            ("core_contract", &wallet.core_contract_id),
            ("circuit_id", &circuit_id),
            ("empty_root", &empty_root_hex),
        ],
    )
    .await?;
    sp.finish_and_clear();

    if output::is_json() {
        output::json_output(serde_json::json!({
            "circuit_id": circuit_id,
            "result": result,
        }));
    } else {
        output::success("init complete");
        output::label("circuit_id", &circuit_id);
        output::label("result", &result);
    }
    Ok(())
}
