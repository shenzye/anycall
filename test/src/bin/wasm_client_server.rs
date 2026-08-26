#[cfg(target_family = "wasm")]
fn main() {}

#[cfg(not(target_family = "wasm"))]
#[tokio::main]
async fn main() {
    native::run().await;
}

#[cfg(not(target_family = "wasm"))]
mod native {
    use std::time::Duration;

    use anycall::coder::CborCoder;
    use anycall_protocol::axum::AxumBodyHandler;
    use anycall_protocol::iroh::{ALPN, IrohHandler};
    use anycall_test::WASM_CLIENT_TEST_PORT;
    use anycall_test::counter::{CounterAsyncService, CounterServerImpl};
    use axum::Json;
    use axum::Router;
    use axum::body::Body;
    use axum::extract::Request;
    use axum::http::{HeaderName, HeaderValue, Method, StatusCode, header};
    use axum::middleware::{self, Next};
    use axum::response::Response;
    use axum::routing::{get, post};
    use iroh::Endpoint;
    use iroh::RelayMode;
    use iroh::endpoint::presets;
    use iroh::protocol::Router as IrohRouter;

    const ALLOW_PRIVATE_NETWORK: HeaderName =
        HeaderName::from_static("access-control-allow-private-network");

    pub async fn run() {
        let port = std::env::var("ANYCALL_WASM_TEST_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(WASM_CLIENT_TEST_PORT);

        let endpoint = Endpoint::builder(presets::N0)
            .relay_mode(RelayMode::Staging)
            .bind()
            .await
            .expect("bind iroh endpoint");
        let iroh_router = IrohRouter::builder(endpoint.clone())
            .accept(
                ALPN,
                IrohHandler::new(CounterServerImpl.into_provider(), CborCoder),
            )
            .spawn();
        tokio::time::timeout(Duration::from_secs(45), endpoint.online())
            .await
            .expect("iroh endpoint online timed out");
        let iroh_addr = endpoint.addr();
        eprintln!("wasm-client-server iroh addr: {iroh_addr:?}");

        let app = Router::new()
            .route("/health", get(|| async { "ok" }))
            .route(
                "/iroh-addr",
                get({
                    let iroh_addr = iroh_addr.clone();
                    move || {
                        let iroh_addr = iroh_addr.clone();
                        async move { Json(iroh_addr) }
                    }
                }),
            )
            .route(
                "/rpc",
                post(AxumBodyHandler::new(
                    CounterServerImpl.into_provider(),
                    CborCoder,
                )),
            )
            .layer(middleware::from_fn(cors));

        let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
            .await
            .expect("bind wasm-client-server");
        eprintln!("wasm-client-server listening on http://127.0.0.1:{port}");
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve wasm-client-server");
        iroh_router.shutdown().await.expect("shutdown iroh router");
    }

    async fn cors(req: Request, next: Next) -> Response {
        if req.method() == Method::OPTIONS {
            let mut response = Response::new(Body::empty());
            *response.status_mut() = StatusCode::NO_CONTENT;
            apply_cors(response.headers_mut());
            return response;
        }
        let mut response = next.run(req).await;
        apply_cors(response.headers_mut());
        response
    }

    fn apply_cors(headers: &mut axum::http::HeaderMap) {
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_ORIGIN,
            HeaderValue::from_static("*"),
        );
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_METHODS,
            HeaderValue::from_static("GET, POST, OPTIONS"),
        );
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            HeaderValue::from_static("*"),
        );
        headers.insert(ALLOW_PRIVATE_NETWORK, HeaderValue::from_static("true"));
    }
}
