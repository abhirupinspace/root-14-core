use std::sync::Arc;

use ark_bls12_381::Fr;
use ark_ff::{BigInteger, PrimeField};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde_json::json;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

use crate::db::Db;
use crate::tree::SparseMerkleTree;

pub struct AppState {
    pub tree: SparseMerkleTree,
    pub db: Db,
}

pub type SharedState = Arc<RwLock<AppState>>;

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/v1/health", get(health))
        .route("/v1/root", get(get_root))
        .route("/v1/proof/{index}", get(get_proof))
        .route("/v1/leaf/{commitment}", get(get_leaf))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}

async fn get_root(State(state): State<SharedState>) -> impl IntoResponse {
    let s = state.read().await;
    let root = s.tree.root();
    let hex = fr_to_hex(&root.0);
    Json(json!({ "root": hex }))
}

async fn get_proof(
    State(state): State<SharedState>,
    Path(index): Path<usize>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let s = state.read().await;
    if index >= s.tree.next_index() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "index out of bounds" })),
        ));
    }
    let proof = s.tree.proof(index);
    let siblings: Vec<String> = proof.siblings.iter().map(fr_to_hex).collect();
    let indices: Vec<bool> = proof.indices;
    Ok(Json(json!({ "siblings": siblings, "indices": indices })))
}

async fn get_leaf(
    State(state): State<SharedState>,
    Path(commitment): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let bytes = hex::decode(commitment.strip_prefix("0x").unwrap_or(&commitment))
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "invalid hex" })),
            )
        })?;
    let fr = Fr::from_be_bytes_mod_order(&bytes);
    let s = state.read().await;
    match s.db.get_leaf_by_commitment(fr) {
        Ok(Some((idx, height))) => Ok(Json(json!({
            "index": idx,
            "block_height": height,
        }))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "commitment not found" })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )),
    }
}

fn fr_to_hex(fr: &Fr) -> String {
    format!("0x{}", hex::encode(fr.into_bigint().to_bytes_be()))
}
