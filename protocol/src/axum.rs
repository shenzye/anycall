use anycall::async_channel::AsyncServiceProvider;
use axum::http::HeaderMap;
use futures::StreamExt;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::future::Future;
use std::sync::Arc;

pub struct AxumBodyHandler<Provider, Coder>
where
    Provider: 'static + Send + Sync + AsyncServiceProvider,
    Coder: 'static + Send + Sync + anycall::coder::Coder,
    Provider::Req: DeserializeOwned + Send,
    Provider::Resp: Serialize,
{
    inner: Arc<AxumBodyHandlerInner<Provider, Coder>>,
}

impl<Provider, Coder> AxumBodyHandler<Provider, Coder>
where
    Provider: 'static + Send + Sync + AsyncServiceProvider,
    Coder: 'static + Send + Sync + anycall::coder::Coder,
    Provider::Req: DeserializeOwned + Send,
    Provider::Resp: Serialize,
{
    pub fn new(provider: Provider, coder: Coder) -> Self {
        Self {
            inner: Arc::new(AxumBodyHandlerInner { provider, coder }),
        }
    }
}

struct AxumBodyHandlerInner<Provider, Coder>
where
    Provider: 'static + Send + Sync + AsyncServiceProvider,
    Coder: 'static + Send + Sync + anycall::coder::Coder,
    Provider::Req: DeserializeOwned + Send,
    Provider::Resp: Serialize,
{
    provider: Provider,
    coder: Coder,
}

impl<Provider, Coder> Clone for AxumBodyHandler<Provider, Coder>
where
    Provider: 'static + Send + Sync + AsyncServiceProvider,
    Coder: 'static + Send + Sync + anycall::coder::Coder,
    Provider::Req: DeserializeOwned + Send,
    Provider::Resp: Serialize,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

pub struct HttpContext {
    pub url: String,
    pub header_map: HeaderMap,
}
impl<'a> From<&'a axum::extract::Request> for HttpContext {
    fn from(value: &'a axum::extract::Request) -> Self {
        Self {
            url: value.uri().path().to_string(),
            header_map: value.headers().clone(),
        }
    }
}

impl From<HttpContext> for () {
    fn from(_: HttpContext) {}
}

impl<Provider, Coder> ::axum::handler::Handler<(), ()> for AxumBodyHandler<Provider, Coder>
where
    Provider: 'static + Send + Sync + AsyncServiceProvider,
    Coder: 'static + Send + Sync + anycall::coder::Coder,
    Provider::Req: DeserializeOwned + Send,
    Provider::Resp: Serialize,
    Provider::Ctx: From<HttpContext>,
    Coder::DesErr: Send,
    Coder::SerErr: Send,
{
    type Future =
        ::core::pin::Pin<Box<dyn Future<Output = ::axum::response::Response> + Send + 'static>>;
    fn call(self, req: ::axum::extract::Request, (): ()) -> Self::Future {
        let http_context = HttpContext::from(&req);
        let inner = self.inner;
        Box::pin(async move {
            let mut body_data = Vec::new();
            let mut stream = req.into_body().into_data_stream();
            let mut count = 0usize;
            while let Some(b) = stream.next().await {
                match b {
                    Ok(b) => {
                        count += b.len();
                        if count > 500_000_000 {
                            let mut resp =
                                ::axum::response::Response::new(::axum::body::Body::empty());
                            *resp.status_mut() = ::axum::http::StatusCode::PAYLOAD_TOO_LARGE;
                            return resp;
                        }
                        body_data.push(b);
                    }
                    Err(_) => {
                        let mut resp = ::axum::response::Response::new(::axum::body::Body::empty());
                        *resp.status_mut() = ::axum::http::StatusCode::BAD_REQUEST;
                        return resp;
                    }
                }
            }
            let mut body_vec = Vec::with_capacity(count);
            for b in body_data {
                body_vec.extend_from_slice(&b);
            }

            return if let Ok(req) = inner.coder.decode::<Provider::Req>(body_vec.as_slice()) {
                let resp = inner.provider.serve(http_context.into(), req).await;
                if let Ok(data) = inner.coder.encode(resp) {
                    ::axum::response::Response::new(::axum::body::Body::from(data))
                } else {
                    let mut resp = ::axum::response::Response::new(::axum::body::Body::empty());
                    *resp.status_mut() = ::axum::http::StatusCode::INTERNAL_SERVER_ERROR;
                    resp
                }
            } else {
                let mut resp = ::axum::response::Response::new(::axum::body::Body::empty());
                *resp.status_mut() = ::axum::http::StatusCode::BAD_REQUEST;
                resp
            };
        })
    }
}
