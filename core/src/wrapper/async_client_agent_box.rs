use crate::async_channel::AsyncClientAgent;
use crate::maybe_send::MaybeSendBoxFuture;
use std::marker::PhantomData;

pub struct AsyncClientAgentBox<A, E>
where
    A: AsyncClientAgent,
    A::Err: Into<E>,
{
    inner: A,
    _marker: PhantomData<E>,
}

impl<A, E> AsyncClientAgentBox<A, E>
where
    A: AsyncClientAgent,
    A::Err: Into<E>,
{
    pub fn new(inner: A) -> Self {
        Self {
            inner,
            _marker: PhantomData,
        }
    }
}

impl<A, E> AsyncClientAgent for AsyncClientAgentBox<A, E>
where
    A: AsyncClientAgent,
    A::Err: Into<E>,
{
    type Req = A::Req;
    type Resp = A::Resp;
    type Err = E;

    fn call(
        &self,
        request_body: Self::Req,
    ) -> MaybeSendBoxFuture<'_, Result<Self::Resp, Self::Err>> {
        let future = self.inner.call(request_body);
        Box::pin(async move {
            match future.await {
                Ok(response) => Ok(response),
                Err(err) => Err(err.into()),
            }
        })
    }
}

pub trait AsyncClientAgentExt: AsyncClientAgent + Sized {
    fn map_err<E>(self) -> AsyncClientAgentBox<Self, E>
    where
        Self::Err: Into<E>,
    {
        AsyncClientAgentBox::new(self)
    }
}

impl<T> AsyncClientAgentExt for T where T: AsyncClientAgent + Sized {}
