use std::sync::Arc;

use ark_bls12_381::Fr;
use ark_ff::{BigInteger, PrimeField, UniformRand};
use axum::body::Body;
use http_body_util::BodyExt;
use tokio::sync::RwLock;
use tower::ServiceExt;

use r14_indexer::api::{AppState, SharedState};
use r14_indexer::db::Db;
use r14_indexer::tree::{verify_proof, SparseMerkleTree};

fn fr_to_hex(fr: &Fr) -> String {
    format!("0x{}", hex::encode(fr.into_bigint().to_bytes_be()))
}

/// Build shared state from a temp DB path
fn make_state(db: Db, tree: SparseMerkleTree) -> SharedState {
    Arc::new(RwLock::new(AppState { tree, db }))
}

#[tokio::test]
async fn e2e_full_flow() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("test.db");

    // ── 1. Setup: insert 5 random leaves ───────────────────────────────
    let mut rng = ark_std::test_rng();
    let leaves: Vec<Fr> = (0..5).map(|_| Fr::rand(&mut rng)).collect();

    let db = Db::open(&db_path).unwrap();
    let mut tree = SparseMerkleTree::new();

    for (i, leaf) in leaves.iter().enumerate() {
        let idx = tree.insert(*leaf);
        assert_eq!(idx, i);
        db.insert_leaf(idx, *leaf, 100 + i as u64).unwrap();
    }

    let root_after_insert = tree.root();

    // ── 2. Proof verification ──────────────────────────────────────────
    for (i, leaf) in leaves.iter().enumerate() {
        let proof = tree.proof(i);
        assert!(
            verify_proof(*leaf, &proof, &root_after_insert),
            "proof failed for index {i}"
        );
    }

    // ── 3. HTTP endpoints ──────────────────────────────────────────────
    let state = make_state(db, tree);
    let app = r14_indexer::api::router(state.clone());

    // /v1/health → 200
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");

    // /v1/root → 200, matches tree root
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/v1/root")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let expected_root = fr_to_hex(&root_after_insert.0);
    assert_eq!(json["root"], expected_root);

    // /v1/proof/0 → 200, has siblings + indices
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/v1/proof/0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["siblings"].is_array());
    assert!(json["indices"].is_array());

    // /v1/proof/999 → 404
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/v1/proof/999")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    // /v1/leaf/{hex} → 200, correct index + block_height
    let leaf_hex = fr_to_hex(&leaves[2]);
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri(format!("/v1/leaf/{leaf_hex}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["index"], 2);
    assert_eq!(json["block_height"], 102);

    // /v1/leaf/{bogus} → 404
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/v1/leaf/0xdeadbeef00000000000000000000000000000000000000000000000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    // ── 4. Persistence: reopen DB, rebuild tree, same root ─────────────
    drop(state); // drop to release DB lock
    let db2 = Db::open(&db_path).unwrap();
    let loaded = db2.load_leaves().unwrap();
    assert_eq!(loaded.len(), 5);

    let mut tree2 = SparseMerkleTree::new();
    for leaf in &loaded {
        tree2.insert(*leaf);
    }
    assert_eq!(tree2.root(), root_after_insert);

    // ── 5. Cursor round-trip ───────────────────────────────────────────
    db2.save_cursor(42, Some("abc123")).unwrap();
    let cursor = db2.load_cursor().unwrap();
    assert_eq!(cursor, Some((42, Some("abc123".to_string()))));

    // overwrite
    db2.save_cursor(99, None).unwrap();
    let cursor = db2.load_cursor().unwrap();
    assert_eq!(cursor, Some((99, None)));
}
