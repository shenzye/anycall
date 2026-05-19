use crate::async_channel::AsyncServiceProvider;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;

pub struct ContextMapper<P: AsyncServiceProvider, Mapper: (Fn(FromCtx) -> IntoCtx), FromCtx, IntoCtx>(
    P,
    Mapper,
    PhantomData<(FromCtx, IntoCtx)>,
);

impl<P: AsyncServiceProvider, Mapper: (Fn(FromCtx) -> IntoCtx), FromCtx, IntoCtx>
    ContextMapper<P, Mapper, FromCtx, IntoCtx>
{
    pub fn new(provider: P, mapper: Mapper) -> Self {
        Self(provider, mapper, PhantomData)
    }
}

impl<P: AsyncServiceProvider<Ctx = IntoCtx>, Mapper: (Fn(FromCtx) -> IntoCtx), FromCtx, IntoCtx>
    AsyncServiceProvider for ContextMapper<P, Mapper, FromCtx, IntoCtx>
{
    type Req = P::Req;
    type Resp = P::Resp;
    type Ctx = FromCtx;

    fn serve(
        &self,
        context: <Self as AsyncServiceProvider>::Ctx,
        request_body: <Self as AsyncServiceProvider>::Req,
    ) -> Pin<Box<dyn Future<Output = <Self as AsyncServiceProvider>::Resp> + Send + '_>> {
        self.0.serve(self.1(context), request_body)
    }
}

pub struct ContextAsyncMapper<P: AsyncServiceProvider, Mapper, FromCtx, IntoCtx>(
    P,
    Mapper,
    PhantomData<(FromCtx, IntoCtx)>,
);

impl<P: AsyncServiceProvider, Mapper, FromCtx, IntoCtx>
    ContextAsyncMapper<P, Mapper, FromCtx, IntoCtx>
{
    pub fn new(provider: P, mapper: Mapper) -> Self {
        Self(provider, mapper, PhantomData)
    }
}

impl<P, Mapper, Fut, FromCtx, IntoCtx> AsyncServiceProvider
    for ContextAsyncMapper<P, Mapper, FromCtx, IntoCtx>
where
    P: AsyncServiceProvider<Ctx = IntoCtx> + Sync,
    Mapper: Fn(FromCtx) -> Fut + Sync,
    Fut: Future<Output = IntoCtx> + Send,
    FromCtx: Send,
    P::Req: Send,
{
    type Req = P::Req;
    type Resp = P::Resp;
    type Ctx = FromCtx;

    fn serve(
        &self,
        context: <Self as AsyncServiceProvider>::Ctx,
        request_body: <Self as AsyncServiceProvider>::Req,
    ) -> Pin<Box<dyn Future<Output = <Self as AsyncServiceProvider>::Resp> + Send + '_>> {
        let provider = &self.0;
        let mapper = &self.1;
        Box::pin(async move {
            let context = mapper(context).await;
            provider.serve(context, request_body).await
        })
    }
}
