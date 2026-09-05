use serde::de::DeserializeOwned;
use serde::Serialize;

pub trait SyncClientAgent {
    type Err;
    fn call<Req: Serialize, Resp: DeserializeOwned>(
        &self,
        request_body: Req,
    ) -> Result<Resp, <Self as SyncClientAgent>::Err>;
}
pub trait SyncServiceProvider {
    type Req;
    type Resp;
    type Ctx;
    fn serve(
        &self,
        context: <Self as SyncServiceProvider>::Ctx,
        request_body: <Self as SyncServiceProvider>::Req,
    ) -> <Self as SyncServiceProvider>::Resp;
}
