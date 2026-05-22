use super::CounterServerImpl;
use anycall::async_channel::AsyncClientAgent;
use anycall::maybe_send::MaybeSendBoxFuture;
use anycall::wrapper::async_client_agent_box::AsyncClientAgentExt;
use anycall_protocol::balance::{Balance, BalanceBuildError};
use std::convert::Infallible;

use crate::counter::{CounterAsyncClient, CounterRequest, CounterResponse};

struct OffsetAgent {
    offset: i32,
}

#[derive(Debug, PartialEq, Eq)]
struct OffsetErrA;

#[derive(Debug, PartialEq, Eq)]
struct OffsetErrB;

#[derive(Debug, PartialEq, Eq)]
enum MixedAgentErr {
    A(OffsetErrA),
    B(OffsetErrB),
}

impl From<OffsetErrA> for MixedAgentErr {
    fn from(err: OffsetErrA) -> Self {
        Self::A(err)
    }
}

impl From<OffsetErrB> for MixedAgentErr {
    fn from(err: OffsetErrB) -> Self {
        Self::B(err)
    }
}

struct OffsetAgentA;

impl AsyncClientAgent for OffsetAgentA {
    type Req = CounterRequest;
    type Resp = CounterResponse;
    type Err = OffsetErrA;

    fn call(
        &self,
        request_body: Self::Req,
    ) -> MaybeSendBoxFuture<'_, Result<Self::Resp, Self::Err>> {
        Box::pin(async move {
            let response = match request_body {
                CounterRequest::Sum { a, b } => {
                    if a == -1 {
                        return Err(OffsetErrA);
                    }
                    CounterResponse::Sum(a + b)
                }
                CounterRequest::SumAsync { a, b } => {
                    if a == -1 {
                        return Err(OffsetErrA);
                    }
                    CounterResponse::SumAsync(a + b)
                }
            };
            Ok(response)
        })
    }
}

struct OffsetAgentB;

impl AsyncClientAgent for OffsetAgentB {
    type Req = CounterRequest;
    type Resp = CounterResponse;
    type Err = OffsetErrB;

    fn call(
        &self,
        request_body: Self::Req,
    ) -> MaybeSendBoxFuture<'_, Result<Self::Resp, Self::Err>> {
        Box::pin(async move {
            let response = match request_body {
                CounterRequest::Sum { a, b } => {
                    if a == -2 {
                        return Err(OffsetErrB);
                    }
                    CounterResponse::Sum(a + b + 100)
                }
                CounterRequest::SumAsync { a, b } => {
                    if a == -2 {
                        return Err(OffsetErrB);
                    }
                    CounterResponse::SumAsync(a + b + 100)
                }
            };
            Ok(response)
        })
    }
}

impl AsyncClientAgent for OffsetAgent {
    type Req = CounterRequest;
    type Resp = CounterResponse;
    type Err = Infallible;

    fn call(
        &self,
        request_body: Self::Req,
    ) -> MaybeSendBoxFuture<'_, Result<Self::Resp, Self::Err>> {
        let offset = self.offset;
        Box::pin(async move {
            let response = match request_body {
                CounterRequest::Sum { a, b } => CounterResponse::Sum(a + b + offset),
                CounterRequest::SumAsync { a, b } => CounterResponse::SumAsync(a + b + offset),
            };
            Ok(response)
        })
    }
}

#[tokio::test]
async fn test_counter() {
    use crate::counter::CounterAsyncService;
    use crate::counter::{CounterAsyncClient, CounterClient};
    use anycall::coder::CborCoder;
    use anycall_protocol::axum::AxumBodyHandler;
    use anycall_protocol::reqwest::ReqwestPost;
    use axum::Router;
    use axum::routing::post;

    let handle = tokio::spawn(async {
        let app = Router::new()
            .route(
                "/test",
                post(AxumBodyHandler::new(
                    CounterServerImpl.into_provider(),
                    CborCoder,
                )),
            )
            .into_make_service();

        let listener = tokio::net::TcpListener::bind("127.0.0.1:9527")
            .await
            .unwrap();
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    let a = CounterClient::new(ReqwestPost::new(
        reqwest::Client::new(),
        "http://127.0.0.1:9527/test".to_string(),
        CborCoder,
    ));
    assert_eq!(a.sum(1, 1).await.unwrap(), 2);
    assert_eq!(a.sum_async(1, 1).await.unwrap(), 2);
    handle.abort();
}

#[tokio::test]
async fn test_counter_balance_round_robin() {
    use crate::counter::CounterClient;

    let client = CounterClient::new(
        Balance::builder()
            .push(OffsetAgent { offset: 0 })
            .push(OffsetAgent { offset: 100 })
            .build()
            .unwrap(),
    );

    assert_eq!(client.sum(1, 1).await.unwrap(), 2);
    assert_eq!(client.sum(1, 1).await.unwrap(), 102);
    assert_eq!(client.sum_async(1, 1).await.unwrap(), 2);
    assert_eq!(client.sum_async(1, 1).await.unwrap(), 102);
}

#[test]
fn test_counter_balance_builder_rejects_empty_agents() {
    let result = Balance::<CounterRequest, CounterResponse, Infallible>::builder().build();
    assert!(matches!(result, Err(BalanceBuildError::EmptyAgents)));
}

#[tokio::test]
async fn test_counter_balance_round_robin_with_mixed_err_agents() {
    use crate::counter::CounterClient;

    let client = CounterClient::new(
        Balance::builder()
            .push(OffsetAgentA.map_err::<MixedAgentErr>())
            .push(OffsetAgentB.map_err::<MixedAgentErr>())
            .build()
            .unwrap(),
    );

    assert_eq!(client.sum(1, 1).await.unwrap(), 2);
    assert_eq!(client.sum(1, 1).await.unwrap(), 102);
    assert_eq!(client.sum_async(1, 1).await.unwrap(), 2);
    assert_eq!(client.sum_async(1, 1).await.unwrap(), 102);
}

#[tokio::test]
async fn test_counter_balance_mixed_err_map_err() {
    use crate::counter::CounterClient;

    let client = CounterClient::new(
        Balance::builder()
            .push(OffsetAgentA.map_err::<MixedAgentErr>())
            .push(OffsetAgentB.map_err::<MixedAgentErr>())
            .build()
            .unwrap(),
    );

    assert!(matches!(client.sum(-1, 1).await.unwrap_err(), MixedAgentErr::A(_)));
    assert!(matches!(client.sum(-2, 1).await.unwrap_err(), MixedAgentErr::B(_)));
    assert!(matches!(
        client.sum_async(-1, 1).await.unwrap_err(),
        MixedAgentErr::A(_)
    ));
    assert!(matches!(
        client.sum_async(-2, 1).await.unwrap_err(),
        MixedAgentErr::B(_)
    ));
}
