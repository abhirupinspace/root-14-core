use base64::{engine::general_purpose::STANDARD as B64, Engine};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use stellar_xdr::curr::{Limits, ReadXdr, ScVal};

#[derive(Debug)]
pub struct TransferEvent {
    pub nullifier: [u8; 32],
    pub cm_0: [u8; 32],
    pub cm_1: [u8; 32],
    pub ledger: u64,
}

#[derive(Debug)]
pub struct DepositEvent {
    pub cm: [u8; 32],
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
    latest_ledger: u64,
}

#[derive(Deserialize)]
struct RpcEvent {
    ledger: u64,
    value: String,
    id: Option<String>,
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

pub struct DepositPollResult {
    pub events: Vec<DepositEvent>,
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

fn build_topic_filter(contract_id: &str, topic_name: &str) -> serde_json::Value {
    // Build XDR manually: Soroban runtime uses SCV_SYMBOL = tag 14 (0x0e)
    // but stellar-xdr 25.0.0 encodes Symbol as tag 13. Hardcode the correct
    // wire format to match what the chain actually emits.
    let name_bytes = topic_name.as_bytes();
    let mut buf = Vec::new();
    buf.extend_from_slice(&14u32.to_be_bytes()); // SCV_SYMBOL tag on chain
    buf.extend_from_slice(&(name_bytes.len() as u32).to_be_bytes());
    buf.extend_from_slice(name_bytes);
    // XDR strings are padded to 4-byte boundary
    let pad = (4 - (name_bytes.len() % 4)) % 4;
    buf.extend(std::iter::repeat(0u8).take(pad));
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
    let filters = build_topic_filter(contract_id, "transfer");

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

    let mut events = Vec::new();
    let mut last_cursor = None;

    for ev in &result.events {
        last_cursor = ev.id.clone();
        match parse_transfer_value(&ev.value, ev.ledger) {
            Ok(te) => events.push(te),
            Err(e) => eprintln!("skip event parse: {e}"),
        }
    }

    Ok(PollResult {
        events,
        latest_ledger: result.latest_ledger,
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

pub async fn poll_deposit_events(
    client: &Client,
    rpc_url: &str,
    contract_id: &str,
    start_ledger: u64,
    cursor: Option<&str>,
) -> anyhow::Result<DepositPollResult> {
    let filters = build_topic_filter(contract_id, "deposit");

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
        id: 2,
        method: "getEvents",
        params,
    };

    let resp: JsonRpcResponse<GetEventsResult> =
        client.post(rpc_url).json(&req).send().await?.json().await?;

    let result = match resp.result {
        Some(r) => r,
        None => return Err(anyhow::anyhow!("getEvents(deposit) error: {:?}", resp.error)),
    };

    let mut events = Vec::new();
    let mut last_cursor = None;

    for ev in &result.events {
        last_cursor = ev.id.clone();
        match parse_deposit_value(&ev.value, ev.ledger) {
            Ok(de) => events.push(de),
            Err(e) => eprintln!("skip deposit event parse: {e}"),
        }
    }

    Ok(DepositPollResult {
        events,
        latest_ledger: result.latest_ledger,
        cursor: last_cursor,
    })
}

fn parse_deposit_value(value_b64: &str, ledger: u64) -> anyhow::Result<DepositEvent> {
    let xdr_bytes = B64.decode(value_b64)?;
    let sc_val = ScVal::from_xdr(&xdr_bytes, Limits::none())?;

    // deposit event value is a single-element tuple: (cm,)
    match sc_val {
        ScVal::Vec(Some(vec)) if vec.len() == 1 => {
            let cm = extract_bytes32(&vec[0], "cm")?;
            Ok(DepositEvent { cm, ledger })
        }
        _ => Err(anyhow::anyhow!("unexpected deposit event value shape: {sc_val:?}")),
    }
}
