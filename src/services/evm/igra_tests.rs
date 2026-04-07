//! Igra Mainnet (chain id 38833): public RPC disallows `eth_getAccount` — we fall back to
//! `eth_getCode` + `keccak256` after a failed multicall. Nodes that support `eth_getAccount` use
//! `codeHash` only (no bytecode on the wire).
//!
//! - Live RPC tests are `#[ignore]`; run: `cargo test -p token-api services::evm::igra -- --ignored`
//! - Offline: wiremock tests below

use super::*;
use alloy::{
    primitives::hex,
    providers::{MULTICALL3_ADDRESS, ProviderBuilder},
    sol_types::SolValue,
    transports::http::Http,
};
use serde_json::json;
use url::Url;
use wiremock::{
    Mock, MockServer, Request, ResponseTemplate,
    matchers::{method, path},
};

const IGRA_CHAIN_ID: ChainId = 38833;
const DEFAULT_IGRA_RPC: &str = "https://rpc.igralabs.com:8545";
/// Wrapped Igra Kaspa — verified via explorer token list.
const IGRA_WIKAS: &str = "0x17Ec7E1768c813E2a3a9b0f94A35605CA520C242";

/// Empty-account `codeHash` (same as `eth_getAccount` / MPT empty code).
const EMPTY_CODE_HASH: &str = "0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470";

fn igra_rpc_client() -> RpcClient {
    let url = std::env::var("IGRA_RPC_URL").unwrap_or_else(|_| DEFAULT_IGRA_RPC.to_string());
    let url = Url::parse(&url).expect("IGRA_RPC_URL or default must be a valid URL");
    let http = Http::new(url);
    RpcClient::new(http, false)
}

fn body_is_single_eth_chain_id(req: &Request) -> bool {
    let b = String::from_utf8_lossy(&req.body);
    b.contains("\"eth_chainId\"") && !b.trim_start().starts_with('[')
}

fn body_is_single_eth_get_code(req: &Request) -> bool {
    let b = String::from_utf8_lossy(&req.body);
    b.contains("\"eth_getCode\"") && !b.trim_start().starts_with('[')
}

fn body_is_single_eth_get_account(req: &Request) -> bool {
    let b = String::from_utf8_lossy(&req.body);
    b.contains("\"eth_getAccount\"") && !b.trim_start().starts_with('[')
}

/// `eth_call` to the canonical Multicall3 address (aggregate), not ERC20 `eth_call`s.
fn body_is_multicall3_aggregate_eth_call(req: &Request) -> bool {
    let b = String::from_utf8_lossy(&req.body).to_lowercase();
    b.contains("\"eth_call\"")
        && !b.trim_start().starts_with('[')
        && b.contains("ca11bde05977b3631167028862be2a173976ca11")
}

fn body_is_eth_call_with_input_prefix(prefix: &str) -> impl Fn(&Request) -> bool + '_ {
    let p = prefix.to_lowercase();
    move |req: &Request| {
        let b = String::from_utf8_lossy(&req.body).to_lowercase();
        b.contains("\"eth_call\"") && !b.trim_start().starts_with('[') && b.contains(p.as_str())
    }
}

fn jsonrpc_eth_result_template(req: &Request, result: String) -> ResponseTemplate {
    let id = serde_json::from_slice::<serde_json::Value>(&req.body)
        .ok()
        .and_then(|v| v.get("id").cloned())
        .unwrap_or(json!(0));
    ResponseTemplate::new(200).set_body_json(json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    }))
}

fn eth_call_hex(result_word: impl SolValue) -> String {
    let bytes = result_word.abi_encode();
    format!("0x{}", hex::encode(bytes))
}

/// Solidity `uint8` / `decimals()` return: value 18 ABI-encoded as a 32-byte word.
const ENCODED_DECIMALS_18: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000012";

#[tokio::test]
async fn optimistic_multicall_then_eth_get_account_empty_codehash_uses_parallel_calls() {
    let mock = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_is_single_eth_chain_id)
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "id": 0u64,
            "result": format!("0x{:x}", IGRA_CHAIN_ID),
        })))
        .mount(&mock)
        .await;

    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_is_multicall3_aggregate_eth_call)
        .respond_with(|req: &Request| jsonrpc_eth_result_template(req, "0x".into()))
        .mount(&mock)
        .await;

    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_is_single_eth_get_account)
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "id": 2u64,
            "result": {
                "nonce": "0x0",
                "balance": "0x0",
                "storageRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
                "codeHash": EMPTY_CODE_HASH,
            },
        })))
        .mount(&mock)
        .await;

    let name_hex = eth_call_hex("Mock Igra Name".to_string());
    let symbol_hex = eth_call_hex("MIGRA".to_string());

    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_is_eth_call_with_input_prefix("06fdde03"))
        .respond_with(move |req: &Request| jsonrpc_eth_result_template(req, name_hex.clone()))
        .mount(&mock)
        .await;

    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_is_eth_call_with_input_prefix("95d89b41"))
        .respond_with(move |req: &Request| jsonrpc_eth_result_template(req, symbol_hex.clone()))
        .mount(&mock)
        .await;

    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_is_eth_call_with_input_prefix("313ce567"))
        .respond_with(move |req: &Request| {
            jsonrpc_eth_result_template(req, ENCODED_DECIMALS_18.into())
        })
        .mount(&mock)
        .await;

    let url: Url = mock.uri().parse().expect("wiremock uri");
    let http = Http::new(url);
    let rpc = RpcClient::new(http, true);

    let address = Address::repeat_byte(0x7e);
    let token = EvmTokenService::fetch_token(IGRA_CHAIN_ID, address, rpc)
        .await
        .expect("fetch_token after empty Multicall3 codeHash (parallel eth_call)");

    assert_eq!(token.name, "Mock Igra Name");
    assert_eq!(token.symbol, "MIGRA");
    assert_eq!(token.decimals, 18);
}

#[tokio::test]
async fn optimistic_multicall_then_get_account_error_falls_back_to_get_code_and_parallel_calls() {
    let mock = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_is_single_eth_chain_id)
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "id": 0u64,
            "result": format!("0x{:x}", IGRA_CHAIN_ID),
        })))
        .mount(&mock)
        .await;

    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_is_multicall3_aggregate_eth_call)
        .respond_with(|req: &Request| jsonrpc_eth_result_template(req, "0x".into()))
        .mount(&mock)
        .await;

    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_is_single_eth_get_account)
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "id": 2u64,
            "error": {"code": -32002, "message": "RPC method not allowed: eth_getAccount"},
        })))
        .mount(&mock)
        .await;

    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_is_single_eth_get_code)
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "id": 3u64,
            "result": "0x",
        })))
        .mount(&mock)
        .await;

    let name_hex = eth_call_hex("Fallback Path".to_string());
    let symbol_hex = eth_call_hex("FB".to_string());

    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_is_eth_call_with_input_prefix("06fdde03"))
        .respond_with(move |req: &Request| jsonrpc_eth_result_template(req, name_hex.clone()))
        .mount(&mock)
        .await;

    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_is_eth_call_with_input_prefix("95d89b41"))
        .respond_with(move |req: &Request| jsonrpc_eth_result_template(req, symbol_hex.clone()))
        .mount(&mock)
        .await;

    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_is_eth_call_with_input_prefix("313ce567"))
        .respond_with(move |req: &Request| {
            jsonrpc_eth_result_template(req, ENCODED_DECIMALS_18.into())
        })
        .mount(&mock)
        .await;

    let url: Url = mock.uri().parse().expect("wiremock uri");
    let http = Http::new(url);
    let rpc = RpcClient::new(http, true);

    let address = Address::repeat_byte(0xab);
    let token = EvmTokenService::fetch_token(IGRA_CHAIN_ID, address, rpc)
        .await
        .expect("fetch_token Igra-style getAccount deny + getCode");

    assert_eq!(token.name, "Fallback Path");
    assert_eq!(token.symbol, "FB");
    assert_eq!(token.decimals, 18);
}

#[tokio::test]
async fn igra_mainnet_chain_id_and_multicall3_slot_effective_empty() {
    let rpc = igra_rpc_client();
    let provider = ProviderBuilder::new().connect_client(rpc);
    assert_eq!(
        provider.get_chain_id().await.expect("chain id"),
        IGRA_CHAIN_ID as u64
    );
    assert!(
        provider
            .get_code_at(MULTICALL3_ADDRESS)
            .await
            .expect("eth_getCode")
            .is_empty(),
        "Igra has no Multicall3 bytecode at {MULTICALL3_ADDRESS:?} (canonical address)"
    );
}

#[tokio::test]
async fn igra_fetch_token_after_failed_multicall_uses_json_batch() {
    let rpc = igra_rpc_client();
    let address: Address = IGRA_WIKAS.parse().expect("WiKAS address");
    let token = EvmTokenService::fetch_token(IGRA_CHAIN_ID, address, rpc)
        .await
        .expect("fetch_token on Igra after optimistic multicall failure");
    assert_eq!(token.symbol, "WiKAS");
    assert_eq!(token.name, "Wrapped Igra Kaspa");
    assert_eq!(token.decimals, 18);
}
