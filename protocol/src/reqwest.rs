use anycall::async_channel::AsyncClientAgent;
use anycall::maybe_send::{MaybeSend, MaybeSendBoxFuture};
use arc_swap::ArcSwap;
use reqwest::header::HeaderMap;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;
use thiserror::Error;

pub struct ReqwestPost<Coder, Req, Resp>
where
    Coder: anycall::coder::Coder,
    Req: Serialize,
    Resp: DeserializeOwned,
{
    reqwest_client: Client,
    pub config: ArcSwap<ReqwestPostConfig>,
    coder: Coder,
    _req_resp: PhantomData<(Req, Resp)>,
}

#[derive(Debug)]
pub struct ReqwestPostConfig {
    pub api_url: String,
    pub header_map: HeaderMap,
}

impl<Coder, Req, Resp> ReqwestPost<Coder, Req, Resp>
where
    Coder: anycall::coder::Coder,
    Req: Serialize,
    Resp: DeserializeOwned,
{
    pub fn new(reqwest_client: Client, config: ArcSwap<ReqwestPostConfig>, coder: Coder) -> Self {
        Self {
            reqwest_client,
            config,
            coder,
            _req_resp: PhantomData,
        }
    }
}

#[derive(Error, Debug)]
pub enum HttpPostErr<S, D> {
    #[error("{0}")]
    Ser(S),
    #[error("{0}")]
    Des(D),
    #[error("{0}")]
    Rqe(reqwest::Error),
    #[error("HTTP {status}")]
    Status {
        status: reqwest::StatusCode,
        body: Vec<u8>,
    },
}

impl<Coder, Req, Resp> AsyncClientAgent for ReqwestPost<Coder, Req, Resp>
where
    Coder: anycall::coder::Coder + Sync,
    Coder::SerErr: MaybeSend,
    Coder::DesErr: MaybeSend,
    Req: Serialize,
    Resp: DeserializeOwned + MaybeSend,
{
    type Req = Req;
    type Resp = Resp;
    type Err = HttpPostErr<Coder::SerErr, Coder::DesErr>;

    fn call(
        &self,
        request_body: Self::Req,
    ) -> MaybeSendBoxFuture<'_, Result<Self::Resp, Self::Err>> {
        let body = match self.coder.encode(request_body).map_err(HttpPostErr::Ser) {
            Ok(body) => body,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let config = self.config.load_full();
        let req = self
            .reqwest_client
            .post(config.api_url.as_str())
            .headers(config.header_map.clone())
            .body(body)
            .send();
        let coder = &self.coder;
        Box::pin(async move {
            let response = req.await.map_err(HttpPostErr::Rqe)?;
            let status = response.status();
            let body = response.bytes().await.map_err(HttpPostErr::Rqe)?;
            if !status.is_success() {
                return Err(HttpPostErr::Status {
                    status,
                    body: body.to_vec(),
                });
            }
            coder.decode(body.as_ref()).map_err(HttpPostErr::Des)
        })
    }
}
