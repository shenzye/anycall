use anycall::async_channel::AsyncClientAgent;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::AtomicUsize;
use thiserror::Error;

pub type BalanceAgent<Req, Resp, Err> =
    Box<dyn AsyncClientAgent<Req = Req, Resp = Resp, Err = Err>>;

pub struct Balance<Req, Resp, Err>
where
    Req: Serialize,
    Resp: DeserializeOwned,
{
    agents: Vec<BalanceAgent<Req, Resp, Err>>,
    index: AtomicUsize,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BalanceBuildError {
    #[error("balance requires at least one agent")]
    EmptyAgents,
}

pub struct BalanceBuilder<Req, Resp, Err>
where
    Req: Serialize,
    Resp: DeserializeOwned,
{
    agents: Vec<BalanceAgent<Req, Resp, Err>>,
}

impl<Req, Resp, Err> BalanceBuilder<Req, Resp, Err>
where
    Req: Serialize,
    Resp: DeserializeOwned,
{
    pub fn push<A>(mut self, agent: A) -> Self
    where
        A: AsyncClientAgent<Req = Req, Resp = Resp, Err = Err> + 'static,
    {
        self.agents.push(Box::new(agent));
        self
    }

    pub fn build(self) -> Result<Balance<Req, Resp, Err>, BalanceBuildError> {
        if self.agents.is_empty() {
            return Err(BalanceBuildError::EmptyAgents);
        }
        Ok(Balance {
            agents: self.agents,
            index: AtomicUsize::new(0),
        })
    }
}

impl<Req, Resp, Err> Balance<Req, Resp, Err>
where
    Req: Serialize,
    Resp: DeserializeOwned,
{
    pub fn builder() -> BalanceBuilder<Req, Resp, Err> {
        BalanceBuilder { agents: vec![] }
    }
}

impl<Req, Resp, Err> AsyncClientAgent for Balance<Req, Resp, Err>
where
    Req: Serialize,
    Resp: DeserializeOwned,
{
    type Req = Req;
    type Resp = Resp;
    type Err = Err;

    fn call(
        &self,
        request_body: Self::Req,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Resp, Self::Err>> + '_>> {
        let idx = self.index.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.agents[idx % self.agents.len()].call(request_body)
    }
}
