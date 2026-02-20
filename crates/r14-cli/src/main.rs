mod commands;
pub mod soroban;
mod wallet;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "r14", about = "Private transfer CLI for Stellar")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::Keygen => commands::keygen::run()?,
        Cmd::Deposit { value, app_tag, local_only } => {
            commands::deposit::run(value, app_tag, local_only).await?
        }
        Cmd::Transfer { value, recipient, dry_run } => {
            commands::transfer::run(value, &recipient, dry_run).await?
        }
        Cmd::InitContract => commands::init_contract::run().await?,
        Cmd::Balance => commands::balance::run().await?,
    }
    Ok(())
}
