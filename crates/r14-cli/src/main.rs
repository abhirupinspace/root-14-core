mod commands;
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
    /// Create a note (local-only for hackathon)
    Deposit {
        /// Note value
        value: u64,
        /// Application tag
        #[arg(long, default_value_t = 1)]
        app_tag: u32,
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
    /// Show balance and sync with indexer
    Balance,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::Keygen => commands::keygen::run()?,
        Cmd::Deposit { value, app_tag } => commands::deposit::run(value, app_tag)?,
        Cmd::Transfer { value, recipient, dry_run } => {
            commands::transfer::run(value, &recipient, dry_run).await?
        }
        Cmd::Balance => commands::balance::run().await?,
    }
    Ok(())
}
