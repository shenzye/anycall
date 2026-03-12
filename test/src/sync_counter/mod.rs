#[anycall::service]
pub trait SyncCounter {
    fn sum(&self, a: i32, b: i32) -> i32;
}

struct SyncCounterServerImpl;

impl SyncCounterSyncService for SyncCounterServerImpl {
    type Ctx = ();
    fn sum(&self, _ctx: (), a: i32, b: i32) -> i32 {
        a + b
    }
}

#[cfg(not(target_family = "wasm"))]
#[cfg(test)]
mod native_tests {
    use super::SyncCounterServerImpl;
    use crate::sync_counter::{SyncCounterClient, SyncCounterSyncClient, SyncCounterSyncService};
    use anycall::coder::CborCoder;
    use anycall_protocol::channel::{ChannelErr, new_channel_pair};

    #[test]
    fn test_counter() {
        let (sender, receiver) =
            new_channel_pair(SyncCounterServerImpl.into_sync_provider(), CborCoder);
        let handle = std::thread::spawn(move || {
            receiver.pool_once().unwrap();
            receiver.pool_once().unwrap();
        });

        let client = SyncCounterClient::new(sender);
        assert_eq!(client.sum(1, 1).unwrap(), 2);
        assert_eq!(client.sum(2, 3).unwrap(), 5);
        handle.join().unwrap();
    }

    #[test]
    fn test_counter_when_receiver_is_dropped() {
        let (sender, receiver) =
            new_channel_pair(SyncCounterServerImpl.into_sync_provider(), CborCoder);
        drop(receiver);

        let client = SyncCounterClient::new(sender);
        assert!(matches!(client.sum(1, 1), Err(ChannelErr::SendReq(_))));
    }
}
