mod commands;
pub mod output;

use clap::{Parser, Subcommand};
use r14_sdk::wallet;

#[derive(Parser)]
#[command(name = "r14", about = "Private transfer CLI for Stellar")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
    /// Output as JSON (machine-readable)
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum Cmd {
    /// Generate a new keypair and create wallet
    Keygen,
    /// Create a note and submit deposit on-chain
    Deposit {
        /// Note value
        value: u64,
        /// Application tag
        #[arg(long, default_value_t = 1)]
        app_tag: u32,
        /// Skip on-chain submission, only create local note
        #[arg(long)]
        local_only: bool,
    },
    /// Private transfer with ZK proof
    Transfer {
        /// Amount to send
        value: u64,
        /// Recipient owner_hash (hex)
        recipient: String,
        /// Only generate proof, don't submit to Soroban
        #[arg(long)]
        dry_run: bool,
    },
    /// Initialize contract with verification key
    InitContract,
    /// Show balance and sync with indexer
    Balance,
    /// Compute merkle root for given commitments (offline, no indexer)
    ComputeRoot {
        /// Commitment hex values (no 0x prefix)
        commitments: Vec<String>,
    },
    /// Show wallet and indexer status
    Status,
    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Set a config value
    Set {
        /// Config key (rpc_url, indexer_url, core_contract_id, transfer_contract_id, stellar_secret)
        key: String,
        /// New value
        value: String,
    },
    /// Show current config
    Show,
}

fn validate_config(wallet: &wallet::WalletData) -> anyhow::Result<()> {
    let mut problems = vec![];
    if wallet.stellar_secret == "PLACEHOLDER" {
        problems.push("stellar_secret");
    }
    if wallet.core_contract_id == "PLACEHOLDER" {
        problems.push("core_contract_id");
    }
    if wallet.transfer_contract_id == "PLACEHOLDER" {
        problems.push("transfer_contract_id");
    }
    if !problems.is_empty() {
        return Err(output::fail_with_hint(
            &format!("unconfigured: {}", problems.join(", ")),
            "run `r14 config set <key> <value>` to configure",
        ));
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    output::set_json_mode(cli.json);

    match cli.command {
        Cmd::Keygen => commands::keygen::run()?,
        Cmd::Deposit { value, app_tag, local_only } => {
            if !local_only {
                let w = wallet::load_wallet()?;
                validate_config(&w)?;
            }
            commands::deposit::run(value, app_tag, local_only).await?
        }
        Cmd::Transfer { value, recipient, dry_run } => {
            if !dry_run {
                let w = wallet::load_wallet()?;
                validate_config(&w)?;
            }
            commands::transfer::run(value, &recipient, dry_run).await?
        }
        Cmd::InitContract => {
            let w = wallet::load_wallet()?;
            validate_config(&w)?;
            commands::init_contract::run().await?
        }
        Cmd::Balance => commands::balance::run().await?,
        Cmd::ComputeRoot { commitments } => {
            use r14_sdk::merkle;
            if commitments.is_empty() {
                println!("{}", merkle::empty_root_hex());
            } else {
                let leaves: Vec<ark_bls12_381::Fr> = commitments
                    .iter()
                    .map(|h| wallet::hex_to_fr(h))
                    .collect::<anyhow::Result<_>>()?;
                println!("{}", merkle::compute_root_from_leaves(&leaves));
            }
        }
        Cmd::Status => commands::status::run().await?,
        Cmd::Config { action } => match action {
            ConfigAction::Set { key, value } => commands::config::set(&key, &value)?,
            ConfigAction::Show => commands::config::show()?,
        },
    }
    Ok(())
}
