use base64::{engine::general_purpose::STANDARD as B64, Engine};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use stellar_xdr::curr::{Limits, ReadXdr, ScVal, WriteXdr};

#[derive(Debug)]
pub struct TransferEvent {
    pub nullifier: [u8; 32],
    pub cm_0: [u8; 32],
    pub cm_1: [u8; 32],
    pub ledger: u64,
}

#[derive(Serialize)]
struct JsonRpcRequest<'a> {
    jsonrpc: &'a str,
    id: u64,
    method: &'a str,
    params: serde_json::Value,
}

#[derive(Deserialize)]
struct JsonRpcResponse<T> {
    result: Option<T>,
    error: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct GetEventsResult {
    events: Vec<RpcEvent>,
    #[serde(rename = "latestLedger")]
    latest_ledger: String,
}

#[derive(Deserialize)]
struct RpcEvent {
    #[serde(rename = "ledger")]
    ledger: String,
    value: String,
    #[serde(rename = "pagingToken")]
    paging_token: Option<String>,
}

#[derive(Deserialize)]
struct GetLatestLedgerResult {
    sequence: u64,
}

pub struct PollResult {
    pub events: Vec<TransferEvent>,
    pub latest_ledger: u64,
    pub cursor: Option<String>,
}

pub async fn get_latest_ledger(client: &Client, rpc_url: &str) -> anyhow::Result<u64> {
    let req = JsonRpcRequest {
        jsonrpc: "2.0",
        id: 1,
        method: "getLatestLedger",
        params: serde_json::json!({}),
    };
    let resp: JsonRpcResponse<GetLatestLedgerResult> =
        client.post(rpc_url).json(&req).send().await?.json().await?;
    match resp.result {
        Some(r) => Ok(r.sequence),
        None => Err(anyhow::anyhow!("getLatestLedger error: {:?}", resp.error)),
    }
}

fn build_topic_filter(contract_id: &str) -> serde_json::Value {
    // Encode Symbol("transfer") as XDR -> base64
    let topic_xdr = ScVal::Symbol(stellar_xdr::curr::ScSymbol("transfer".try_into().unwrap()));
    let buf = topic_xdr.to_xdr(Limits::none()).unwrap();
    let topic_b64 = B64.encode(&buf);

    serde_json::json!([{
        "type": "contract",
        "contractIds": [contract_id],
        "topics": [[topic_b64]]
    }])
}

pub async fn poll_events(
    client: &Client,
    rpc_url: &str,
    contract_id: &str,
    start_ledger: u64,
    cursor: Option<&str>,
) -> anyhow::Result<PollResult> {
    let filters = build_topic_filter(contract_id);

    let mut params = serde_json::json!({
        "filters": filters,
        "pagination": { "limit": 100 }
    });

    if let Some(c) = cursor {
        params["pagination"]["cursor"] = serde_json::json!(c);
    } else {
        params["startLedger"] = serde_json::json!(start_ledger);
    }

    let req = JsonRpcRequest {
        jsonrpc: "2.0",
        id: 1,
        method: "getEvents",
        params,
    };

    let resp: JsonRpcResponse<GetEventsResult> =
        client.post(rpc_url).json(&req).send().await?.json().await?;

    let result = match resp.result {
        Some(r) => r,
        None => return Err(anyhow::anyhow!("getEvents error: {:?}", resp.error)),
    };

    let latest_ledger = result.latest_ledger.parse::<u64>()?;
    let mut events = Vec::new();
    let mut last_cursor = None;

    for ev in &result.events {
        last_cursor = ev.paging_token.clone();
        let ledger = ev.ledger.parse::<u64>()?;
        match parse_transfer_value(&ev.value, ledger) {
            Ok(te) => events.push(te),
            Err(e) => eprintln!("skip event parse: {e}"),
        }
    }

    Ok(PollResult {
        events,
        latest_ledger,
        cursor: last_cursor,
    })
}

fn parse_transfer_value(value_b64: &str, ledger: u64) -> anyhow::Result<TransferEvent> {
    let xdr_bytes = B64.decode(value_b64)?;
    let sc_val = ScVal::from_xdr(&xdr_bytes, Limits::none())?;

    match sc_val {
        ScVal::Vec(Some(vec)) if vec.len() == 3 => {
            let nullifier = extract_bytes32(&vec[0], "nullifier")?;
            let cm_0 = extract_bytes32(&vec[1], "cm_0")?;
            let cm_1 = extract_bytes32(&vec[2], "cm_1")?;
            Ok(TransferEvent {
                nullifier,
                cm_0,
                cm_1,
                ledger,
            })
        }
        _ => Err(anyhow::anyhow!("unexpected event value shape: {sc_val:?}")),
    }
}

fn extract_bytes32(val: &ScVal, name: &str) -> anyhow::Result<[u8; 32]> {
    match val {
        ScVal::Bytes(b) => {
            let slice: &[u8] = b.as_ref();
            slice
                .try_into()
                .map_err(|_| anyhow::anyhow!("{name}: expected 32 bytes, got {}", slice.len()))
        }
        _ => Err(anyhow::anyhow!("{name}: expected Bytes, got {val:?}")),
    }
}
