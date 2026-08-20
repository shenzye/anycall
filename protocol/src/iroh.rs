//! QUIC peer transport over [iroh](https://docs.rs/iroh).
//!
//! Native and `wasm32-unknown-unknown`. Stream tasks are spawned with
//! [`n0_future::task::spawn`] (Tokio natively, `spawn_local` in the browser).
use anycall::async_channel::{AsyncClientAgent, AsyncServiceProvider};
use anycall::maybe_send::{MaybeSend, MaybeSendBoxFuture};
use arc_swap::{ArcSwap, ArcSwapOption};
use iroh::endpoint::{
    ClosedStream, ConnectError, Connection, ConnectionError, ReadToEndError, RecvStream,
    SendStream, WriteError,
};
use iroh::protocol::{AcceptError, ProtocolHandler};
use iroh::{Endpoint, EndpointAddr, EndpointId};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::sync::Arc;
use thiserror::Error;

pub const ALPN: &[u8] = b"anycall/iroh/1";

const MAX_PAYLOAD: usize = 500_000_000;

pub struct IrohContext {
    pub remote_id: EndpointId,
}

impl From<IrohContext> for () {
    fn from(_: IrohContext) {}
}

pub struct IrohHandler<Provider, Coder>
where
    Provider: 'static + Send + Sync + AsyncServiceProvider,
    Coder: 'static + Send + Sync + anycall::coder::Coder,
    Provider::Req: DeserializeOwned + Send,
    Provider::Resp: Serialize,
{
    inner: Arc<IrohHandlerInner<Provider, Coder>>,
}

impl<Provider, Coder> IrohHandler<Provider, Coder>
where
    Provider: 'static + Send + Sync + AsyncServiceProvider,
    Coder: 'static + Send + Sync + anycall::coder::Coder,
    Provider::Req: DeserializeOwned + Send,
    Provider::Resp: Serialize,
{
    pub fn new(provider: Provider, coder: Coder) -> Self {
        Self {
            inner: Arc::new(IrohHandlerInner { provider, coder }),
        }
    }
}

struct IrohHandlerInner<Provider, Coder>
where
    Provider: 'static + Send + Sync + AsyncServiceProvider,
    Coder: 'static + Send + Sync + anycall::coder::Coder,
    Provider::Req: DeserializeOwned + Send,
    Provider::Resp: Serialize,
{
    provider: Provider,
    coder: Coder,
}

impl<Provider, Coder> Clone for IrohHandler<Provider, Coder>
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

impl<Provider, Coder> Debug for IrohHandler<Provider, Coder>
where
    Provider: 'static + Send + Sync + AsyncServiceProvider,
    Coder: 'static + Send + Sync + anycall::coder::Coder,
    Provider::Req: DeserializeOwned + Send,
    Provider::Resp: Serialize,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IrohHandler").finish_non_exhaustive()
    }
}

impl<Provider, Coder> ProtocolHandler for IrohHandler<Provider, Coder>
where
    Provider: 'static + Send + Sync + AsyncServiceProvider,
    Coder: 'static + Send + Sync + anycall::coder::Coder,
    Provider::Req: DeserializeOwned + Send,
    Provider::Resp: Serialize,
    Provider::Ctx: From<IrohContext>,
{
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let remote_id = connection.remote_id();
        loop {
            let (send, recv) = match connection.accept_bi().await {
                Ok(stream) => stream,
                Err(_) => return Ok(()),
            };
            let inner = self.inner.clone();
            n0_future::task::spawn(async move {
                let _ = serve_stream(&inner, IrohContext { remote_id }, send, recv).await;
            });
        }
    }
}

async fn serve_stream<Provider, Coder>(
    inner: &IrohHandlerInner<Provider, Coder>,
    context: IrohContext,
    mut send: SendStream,
    mut recv: RecvStream,
) -> Result<(), AcceptError>
where
    Provider: 'static + Send + Sync + AsyncServiceProvider,
    Coder: 'static + Send + Sync + anycall::coder::Coder,
    Provider::Req: DeserializeOwned + Send,
    Provider::Resp: Serialize,
    Provider::Ctx: From<IrohContext>,
{
    let data = recv
        .read_to_end(MAX_PAYLOAD)
        .await
        .map_err(AcceptError::from_err)?;
    let Ok(req) = inner.coder.decode::<Provider::Req>(data.as_slice()) else {
        return Ok(());
    };
    let resp = inner.provider.serve(context.into(), req).await;
    let Ok(data) = inner.coder.encode(resp) else {
        return Ok(());
    };
    send.write_all(&data).await.map_err(AcceptError::from_err)?;
    send.finish()?;
    Ok(())
}

pub struct IrohConnect<Coder, Req, Resp>
where
    Coder: anycall::coder::Coder,
    Req: Serialize,
    Resp: DeserializeOwned,
{
    endpoint: Endpoint,
    pub config: ArcSwap<IrohConnectConfig>,
    connection: ArcSwapOption<CachedConnection>,
    coder: Coder,
    _req_resp: PhantomData<fn() -> (Req, Resp)>,
}

struct CachedConnection {
    endpoint_id: EndpointId,
    alpn: Vec<u8>,
    connection: Connection,
}

#[derive(Debug, Clone)]
pub struct IrohConnectConfig {
    pub addr: EndpointAddr,
    pub alpn: Vec<u8>,
}

impl<Coder, Req, Resp> IrohConnect<Coder, Req, Resp>
where
    Coder: anycall::coder::Coder,
    Req: Serialize,
    Resp: DeserializeOwned,
{
    pub fn new(endpoint: Endpoint, config: ArcSwap<IrohConnectConfig>, coder: Coder) -> Self {
        Self {
            endpoint,
            config,
            connection: ArcSwapOption::empty(),
            coder,
            _req_resp: PhantomData,
        }
    }

    async fn connection(&self) -> Result<Connection, iroh::endpoint::ConnectError> {
        let config = self.config.load_full();
        if let Some(cached) = self.connection.load_full() {
            if cached.endpoint_id == config.addr.id
                && cached.alpn == config.alpn
                && cached.connection.close_reason().is_none()
            {
                return Ok(cached.connection.clone());
            }
        }
        let connection = self
            .endpoint
            .connect(config.addr.clone(), &config.alpn)
            .await?;
        self.connection.store(Some(Arc::new(CachedConnection {
            endpoint_id: config.addr.id,
            alpn: config.alpn.clone(),
            connection: connection.clone(),
        })));
        Ok(connection)
    }
}

#[derive(Error, Debug)]
pub enum IrohConnectErr<S, D> {
    #[error("{0}")]
    Ser(S),
    #[error("{0}")]
    Des(D),
    #[error("{0}")]
    Connect(ConnectError),
    #[error("{0}")]
    Connection(ConnectionError),
    #[error("{0}")]
    Write(WriteError),
    #[error("{0}")]
    Finish(ClosedStream),
    #[error("{0}")]
    Read(ReadToEndError),
}

impl<Coder, Req, Resp> AsyncClientAgent for IrohConnect<Coder, Req, Resp>
where
    Coder: anycall::coder::Coder + Sync,
    Coder::SerErr: MaybeSend,
    Coder::DesErr: MaybeSend,
    Req: Serialize,
    Resp: DeserializeOwned + MaybeSend,
{
    type Req = Req;
    type Resp = Resp;
    type Err = IrohConnectErr<Coder::SerErr, Coder::DesErr>;

    fn call(
        &self,
        request_body: Self::Req,
    ) -> MaybeSendBoxFuture<'_, Result<Self::Resp, Self::Err>> {
        let body = match self.coder.encode(request_body).map_err(IrohConnectErr::Ser) {
            Ok(body) => body,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let coder = &self.coder;
        Box::pin(async move {
            let connection = self.connection().await.map_err(IrohConnectErr::Connect)?;
            let result = async {
                let (mut send, mut recv) = connection
                    .open_bi()
                    .await
                    .map_err(IrohConnectErr::Connection)?;
                send.write_all(&body).await.map_err(IrohConnectErr::Write)?;
                send.finish().map_err(IrohConnectErr::Finish)?;
                let resp = recv
                    .read_to_end(MAX_PAYLOAD)
                    .await
                    .map_err(IrohConnectErr::Read)?;
                coder.decode(resp.as_slice()).map_err(IrohConnectErr::Des)
            }
            .await;
            if result.is_err() && connection.close_reason().is_some() {
                self.connection.store(None);
            }
            result
        })
    }
}
