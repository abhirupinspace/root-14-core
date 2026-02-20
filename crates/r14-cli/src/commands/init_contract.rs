use anyhow::Result;
use ark_std::rand::{rngs::StdRng, SeedableRng};

use crate::wallet::load_wallet;

pub async fn run() -> Result<()> {
    let wallet = load_wallet()?;

    if wallet.stellar_secret == "PLACEHOLDER" || wallet.contract_id == "PLACEHOLDER" {
        anyhow::bail!("stellar_secret or contract_id not set in wallet.json");
    }

    // Deterministic setup — same seed=42 used everywhere
    let mut rng = StdRng::seed_from_u64(42);
    let (_pk, vk) = r14_circuit::setup(&mut rng);

    let svk = r14_circuit::serialize_vk_for_soroban(&vk);

    // Build VK JSON matching the Soroban contract's expected format
    let ic_rest: Vec<String> = svk.ic[1..].iter().map(|s| format!("\"{}\"", s)).collect();
    let vk_json = format!(
        r#"{{"alpha_g1":"{}","beta_g2":"{}","gamma_g2":"{}","delta_g2":"{}","ic_0":"{}","ic_rest":[{}]}}"#,
        svk.alpha_g1, svk.beta_g2, svk.gamma_g2, svk.delta_g2, svk.ic[0], ic_rest.join(",")
    );

    println!("initializing contract with VK...");
    println!("  contract: {}", wallet.contract_id);

    let result = crate::soroban::invoke_contract(
        &wallet.contract_id,
        "testnet",
        &wallet.stellar_secret,
        "init",
        &[("vk", &vk_json)],
    )
    .await?;

    println!("init complete: {result}");
    Ok(())
}
