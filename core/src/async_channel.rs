use serde::Serialize;
use serde::de::DeserializeOwned;
pub trait AsyncClientAgent {
    type Err;
    fn call<Req: Serialize, Resp: DeserializeOwned>(
        &self,
        request_body: Req,
    ) -> impl std::future::Future<Output = Result<Resp, <Self as AsyncClientAgent>::Err>>;
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
