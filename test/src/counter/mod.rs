use std::convert::Infallible;

#[anycall::service]
pub trait Counter {
    fn sum(&self, a: i32, b: i32) -> i32;
    async fn sum_async(&self, a: i32, b: i32) -> i32;
}

#[cfg(not(target_family = "wasm"))]
#[cfg(test)]
mod native_tests {
    #[tokio::test]
    async fn test_counter() {
        use crate::counter::CounterAsyncService;
        use crate::counter::{CounterAsyncClient, CounterClient, CounterServerImpl};
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
}
struct CounterNative;
impl CounterAsyncClient for CounterNative {
    type Err = Infallible;
    async fn sum(&self, a: i32, b: i32) -> Result<i32, Infallible> {
        Ok(a + b)
    }
    async fn sum_async(&self, a: i32, b: i32) -> Result<i32, Infallible> {
        Ok(a + b)
    }
}

struct CounterServerImpl;
impl CounterAsyncService for CounterServerImpl {
    type Ctx = ();
    async fn sum(&self, _ctx: (), a: i32, b: i32) -> i32 {
        a + b
    }

    async fn sum_async(&self, _ctx: (), a: i32, b: i32) -> i32 {
        a + b
    }
}
