use super::{CounterAsyncClient, CounterClient, CounterRequest, CounterResponse};
use crate::WASM_CLIENT_TEST_PORT;
use anycall::coder::CborCoder;
use anycall_protocol::balance::Balance;
use anycall_protocol::iroh::{ALPN, IrohConnect, IrohConnectConfig};
use anycall_protocol::reqwest::{ReqwestPost, ReqwestPostConfig};
use arc_swap::ArcSwap;
use iroh::Endpoint;
use iroh::RelayMode;
use iroh::endpoint::presets;
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

fn rpc_url() -> String {
    format!("http://127.0.0.1:{WASM_CLIENT_TEST_PORT}/rpc")
}

fn reqwest_agent() -> ReqwestPost<CborCoder, CounterRequest, CounterResponse> {
    ReqwestPost::new(
        reqwest::Client::new(),
        ArcSwap::from_pointee(ReqwestPostConfig {
            api_url: rpc_url(),
            header_map: reqwest::header::HeaderMap::new(),
        }),
        CborCoder,
    )
}

#[wasm_bindgen_test]
async fn test_counter_reqwest() {
    let client = CounterClient::new(reqwest_agent());
    assert_eq!(client.sum(1, 1).await.unwrap(), 2);
    assert_eq!(client.sum_async(1, 1).await.unwrap(), 2);
}

#[wasm_bindgen_test]
async fn test_counter_balance_reqwest() {
    let client = CounterClient::new(
        Balance::builder()
            .push(reqwest_agent())
            .push(reqwest_agent())
            .build()
            .unwrap(),
    );
    assert_eq!(client.sum(1, 1).await.unwrap(), 2);
    assert_eq!(client.sum(1, 1).await.unwrap(), 2);
}

async fn fetch_iroh_addr() -> iroh::EndpointAddr {
    let url = format!("http://127.0.0.1:{WASM_CLIENT_TEST_PORT}/iroh-addr");
    let body = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .expect("get iroh-addr")
        .bytes()
        .await
        .expect("iroh-addr body");
    serde_json::from_slice(&body).expect("parse iroh-addr")
}

#[wasm_bindgen_test]
async fn test_counter_iroh() {
    let addr = fetch_iroh_addr().await;
    let endpoint = Endpoint::builder(presets::N0)
        .relay_mode(RelayMode::Staging)
        .bind()
        .await
        .expect("bind iroh client");
    endpoint.online().await;
    let client = CounterClient::new(
        IrohConnect::<CborCoder, CounterRequest, CounterResponse>::new(
            endpoint,
            ArcSwap::from_pointee(IrohConnectConfig {
                addr,
                alpn: ALPN.to_vec(),
            }),
            CborCoder,
        ),
    );
    assert_eq!(client.sum(1, 1).await.unwrap(), 2);
    assert_eq!(client.sum_async(1, 1).await.unwrap(), 2);
}
