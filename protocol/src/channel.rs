use anycall::sync_channel::{SyncClientAgent, SyncServiceProvider};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::mpsc;
use std::sync::mpsc::{sync_channel, Receiver, RecvError, SendError, Sender, SyncSender};
use thiserror::Error;

pub fn new_channel_pair<Provider, Coder>(
    provider: Provider,
    coder: Coder,
) -> (SenderChannel<Coder>, ReceiverChannel<Provider, Coder>)
where
    Provider: 'static + Send + Sync + SyncServiceProvider,
    Coder: 'static + Send + Sync + anycall::coder::Coder + Clone,
    Provider::Req: DeserializeOwned + Send,
    Provider::Resp: Serialize,
{
    let (sender, recevier) = mpsc::channel();
    (
        SenderChannel {
            channel: sender,
            coder: coder.clone(),
        },
        ReceiverChannel {
            channel: recevier,
            provider,
            coder,
        },
    )
}
pub struct SenderChannel<Coder>
where
    Coder: anycall::coder::Coder,
{
    channel: Sender<(Vec<u8>, SyncSender<Vec<u8>>)>,
    coder: Coder,
}
pub struct ReceiverChannel<Provider, Coder>
where
    Provider: 'static + Send + Sync + SyncServiceProvider,
    Coder: 'static + Send + Sync + anycall::coder::Coder,
    Provider::Req: DeserializeOwned + Send,
    Provider::Resp: Serialize,
{
    channel: Receiver<(Vec<u8>, SyncSender<Vec<u8>>)>,
    provider: Provider,
    coder: Coder,
}

#[derive(Error, Debug)]
pub enum ChannelErr<S, D> {
    #[error("{0}")]
    Ser(S),
    #[error("{0}")]
    Des(D),
    #[error("{0}")]
    SendReq(SendError<(Vec<u8>, SyncSender<Vec<u8>>)>),
    #[error("{0}")]
    SendResp(SendError<Vec<u8>>),
    #[error("{0}")]
    RecvResp(RecvError),
}

impl<Coder: anycall::coder::Coder> SyncClientAgent for SenderChannel<Coder> {
    type Err = ChannelErr<Coder::SerErr, Coder::DesErr>;

    fn call<Req: Serialize, Resp: DeserializeOwned>(
        &self,
        request_body: Req,
    ) -> Result<Resp, <Self as SyncClientAgent>::Err> {
        let (sync_sender, sync_receiver) = sync_channel(1);
        self.channel
            .send((
                self.coder.encode(request_body).map_err(ChannelErr::Ser)?,
                sync_sender,
            ))
            .map_err(ChannelErr::SendReq)?;
        let recv = sync_receiver.recv().map_err(ChannelErr::RecvResp)?;
        self.coder.decode(recv).map_err(ChannelErr::Des)
    }
}

impl<Provider, Coder> ReceiverChannel<Provider, Coder>
where
    Provider: 'static + Send + Sync + SyncServiceProvider<Ctx = ()>,
    Coder: 'static + Send + Sync + anycall::coder::Coder,
    Provider::Req: DeserializeOwned + Send,
    Provider::Resp: Serialize,
{
    pub fn pool_once(&self) -> Result<(), ChannelErr<Coder::SerErr, Coder::DesErr>> {
        let (req, resp_chan) = self.channel.recv().map_err(ChannelErr::RecvResp)?;
        let req = self
            .coder
            .decode::<Provider::Req>(req)
            .map_err(ChannelErr::Des)?;

        let resp = self.provider.serve((), req);
        let resp = self.coder.encode(resp).map_err(ChannelErr::Ser)?;
        resp_chan.send(resp).map_err(ChannelErr::SendResp)?;

        Ok(())
    }
}
