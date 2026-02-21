// Copyright 2026 abhirupbanerjee
// Licensed under the Apache License, Version 2.0

//! Soroban contract invocation via the `stellar` CLI.
//!
//! Wraps the `stellar` binary for key derivation and contract calls.
//! Requires the [Stellar CLI](https://github.com/stellar/stellar-cli)
//! to be installed and available on `$PATH`.
//!
//! # Example
//!
//! ```rust,no_run
//! # async fn example() -> anyhow::Result<()> {
//! // derive public key from secret
//! let pubkey = r14_sdk::soroban::get_public_key("S_SECRET...").await?;
//!
//! // invoke a contract function
//! let result = r14_sdk::soroban::invoke_contract(
//!     "C_CONTRACT_ID",
//!     "testnet",
//!     "S_SECRET...",
//!     "deposit",
//!     &[("cm", "deadbeef..."), ("new_root", "cafebabe...")],
//! ).await?;
//! # Ok(())
//! # }
//! ```

use anyhow::{Context, Result};
use tokio::process::Command;

/// Get the public key (G...) for a Stellar secret key
pub async fn get_public_key(secret: &str) -> Result<String> {
    let output = Command::new("stellar")
        .arg("keys")
        .arg("address")
        .arg(secret)
        .output()
        .await
        .context("failed to run `stellar keys address`")?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("stellar keys address failed: {stderr}"))
    }
}

/// Invoke a Soroban contract function via the `stellar` CLI.
///
/// `args` is a list of (arg_name, value) pairs passed as `--arg_name value`.
pub async fn invoke_contract(
    contract_id: &str,
    network: &str,
    source_secret: &str,
    function: &str,
    args: &[(&str, &str)],
) -> Result<String> {
    let mut cmd = Command::new("stellar");
    cmd.arg("contract")
        .arg("invoke")
        .arg("--id")
        .arg(contract_id)
        .arg("--network")
        .arg(network)
        .arg("--source")
        .arg(source_secret)
        .arg("--")
        .arg(function);

    for (name, value) in args {
        cmd.arg(format!("--{name}"));
        cmd.arg(value);
    }

    let output = cmd
        .output()
        .await
        .context("failed to run `stellar` CLI — is it installed?")?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("stellar contract invoke failed: {stderr}"))
    }
}
