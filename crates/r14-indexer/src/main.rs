mod api;
mod db;
mod rpc;
mod tree;

use std::sync::Arc;
use std::time::Duration;

use ark_bls12_381::Fr;
use ark_ff::PrimeField;
use tokio::sync::RwLock;

use api::{AppState, SharedState};
use db::Db;
use tree::SparseMerkleTree;

// ── Config (hardcoded for hackathon MVP) ────────────────────────────
const RPC_URL: &str = "https://soroban-testnet.stellar.org:443";
const CONTRACT_ID: &str = "PLACEHOLDER_CONTRACT_ID";
const DB_PATH: &str = "r14-indexer.db";
const POLL_INTERVAL: Duration = Duration::from_secs(5);
const LISTEN_ADDR: &str = "0.0.0.0:3000";

#[tokio::main]
async fn main() {
    eprintln!("r14-indexer starting...");

    // 1. Open DB + create tables
    let db = Db::open(std::path::Path::new(DB_PATH)).expect("failed to open db");

    // 2. Rebuild tree from persisted leaves
    let mut tree = SparseMerkleTree::new();
    let leaves = db.load_leaves().expect("failed to load leaves");
    let leaf_count = leaves.len();
    for leaf in leaves {
        tree.insert(leaf);
    }
    eprintln!("rebuilt tree with {leaf_count} leaves, root={:?}", tree.root());

    // 3. Load sync cursor
    let cursor_state = db.load_cursor().expect("failed to load cursor");

    let state: SharedState = Arc::new(RwLock::new(AppState { tree, db }));

    // 4. Spawn poller
    let poller_state = state.clone();
    tokio::spawn(async move {
        poller_loop(poller_state, cursor_state).await;
    });

    // 5. Start HTTP server
    let router = api::router(state);
    let listener = tokio::net::TcpListener::bind(LISTEN_ADDR)
        .await
        .expect("failed to bind");
    eprintln!("listening on {LISTEN_ADDR}");
    axum::serve(listener, router).await.expect("server error");
}

async fn poller_loop(state: SharedState, initial_cursor: Option<(u64, Option<String>)>) {
    let client = reqwest::Client::new();

    let (mut start_ledger, mut cursor) = match initial_cursor {
        Some((ledger, c)) => (ledger, c),
        None => {
            // First run: get latest ledger as starting point
            match rpc::get_latest_ledger(&client, RPC_URL).await {
                Ok(seq) => {
                    eprintln!("no cursor, starting from ledger {seq}");
                    (seq, None)
                }
                Err(e) => {
                    eprintln!("failed to get latest ledger: {e}, retrying...");
                    tokio::time::sleep(POLL_INTERVAL).await;
                    return;
                }
            }
        }
    };

    loop {
        tokio::time::sleep(POLL_INTERVAL).await;

        let result = match rpc::poll_events(
            &client,
            RPC_URL,
            CONTRACT_ID,
            start_ledger,
            cursor.as_deref(),
        )
        .await
        {
            Ok(r) => r,
            Err(e) => {
                eprintln!("poll error: {e}");
                continue;
            }
        };

        if !result.events.is_empty() {
            let mut s = state.write().await;
            for ev in &result.events {
                let cm_0 = Fr::from_be_bytes_mod_order(&ev.cm_0);
                let cm_1 = Fr::from_be_bytes_mod_order(&ev.cm_1);

                let idx0 = s.tree.insert(cm_0);
                if let Err(e) = s.db.insert_leaf(idx0, cm_0, ev.ledger) {
                    eprintln!("db insert cm_0 error: {e}");
                }

                let idx1 = s.tree.insert(cm_1);
                if let Err(e) = s.db.insert_leaf(idx1, cm_1, ev.ledger) {
                    eprintln!("db insert cm_1 error: {e}");
                }
            }
            eprintln!(
                "indexed {} events, {} new leaves, root={:?}",
                result.events.len(),
                result.events.len() * 2,
                s.tree.root()
            );
        }

        start_ledger = result.latest_ledger;
        cursor = result.cursor.clone();

        // Persist cursor
        let s = state.read().await;
        if let Err(e) = s.db.save_cursor(start_ledger, cursor.as_deref()) {
            eprintln!("save cursor error: {e}");
        }
    }
}
