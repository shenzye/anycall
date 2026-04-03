use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::pin::Pin;

pub trait AsyncClientAgent {
    type Req: Serialize;
    type Resp: DeserializeOwned;
    type Err;
    fn call(
        &self,
        request_body: Self::Req,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Resp, Self::Err>> + '_>>;
}

pub trait AsyncServiceProvider {
    type Req;
    type Resp;
    type Ctx;
    fn serve(
        &self,
        context: <Self as AsyncServiceProvider>::Ctx,
        request_body: <Self as AsyncServiceProvider>::Req,
    ) -> ::std::pin::Pin<
        ::std::boxed::Box<
            dyn ::std::future::Future<Output = <Self as AsyncServiceProvider>::Resp> + Send + '_,
        >,
    >;
}
