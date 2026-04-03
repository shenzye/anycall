use anycall::async_channel::AsyncClientAgent;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use thiserror::Error;

pub struct ReqwestPost<Coder, Req, Resp>
where
    Coder: anycall::coder::Coder,
    Req: Serialize,
    Resp: DeserializeOwned,
{
    reqwest_client: Client,
    api_url: String,
    coder: Coder,
    _req_resp: PhantomData<(Req, Resp)>,
}

impl<Coder, Req, Resp> ReqwestPost<Coder, Req, Resp>
where
    Coder: anycall::coder::Coder,
    Req: Serialize,
    Resp: DeserializeOwned,
{
    pub fn new(reqwest_client: Client, api_url: String, coder: Coder) -> Self {
        Self {
            reqwest_client,
            api_url,
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
}

impl<Coder, Req, Resp> AsyncClientAgent for ReqwestPost<Coder, Req, Resp>
where
    Coder: anycall::coder::Coder,
    Req: Serialize,
    Resp: DeserializeOwned,
{
    type Req = Req;
    type Resp = Resp;
    type Err = HttpPostErr<Coder::SerErr, Coder::DesErr>;

    fn call(
        &self,
        request_body: Self::Req,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Resp, Self::Err>> + '_>> {
        let body = match self.coder.encode(request_body).map_err(HttpPostErr::Ser) {
            Ok(body) => body,
            Err(err) => return Box::pin(std::future::ready(Err(err))),
        };
        let req = self.reqwest_client.post(&self.api_url).body(body).send();
        Box::pin(async move {
            let b = req
                .await
                .map_err(HttpPostErr::Rqe)?
                .bytes()
                .await
                .map_err(HttpPostErr::Rqe)?;
            self.coder.decode(b.as_ref()).map_err(HttpPostErr::Des)
        })
    }
}
