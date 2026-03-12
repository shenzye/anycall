use anycall::async_channel::AsyncClientAgent;
use reqwest::Client;
use serde::Serialize;
use serde::de::DeserializeOwned;
use thiserror::Error;

pub struct ReqwestPost<Coder>
where
    Coder: anycall::coder::Coder,
{
    reqwest_client: Client,
    api_url: String,
    coder: Coder,
}

impl<Coder> ReqwestPost<Coder>
where
    Coder: anycall::coder::Coder,
{
    pub fn new(reqwest_client: Client, api_url: String, coder: Coder) -> Self {
        Self {
            reqwest_client,
            api_url,
            coder,
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

impl<Coder> AsyncClientAgent for ReqwestPost<Coder>
where
    Coder: anycall::coder::Coder,
{
    type Err = HttpPostErr<Coder::SerErr, Coder::DesErr>;

    async fn call<Req: Serialize, Resp: DeserializeOwned>(
        &self,
        request_body: Req,
    ) -> Result<Resp, Self::Err> {
        let b = self
            .reqwest_client
            .post(&self.api_url)
            .body(self.coder.encode(request_body).map_err(HttpPostErr::Ser)?)
            .send()
            .await
            .map_err(HttpPostErr::Rqe)?
            .bytes()
            .await
            .map_err(HttpPostErr::Rqe)?;

        self.coder.decode(b.as_ref()).map_err(HttpPostErr::Des)
    }
}
