use actix::prelude::*;
use actix::dev::{MessageResponse, ResponseChannel};
use std::marker::PhantomData;
use serde::{Serialize, de::DeserializeOwned};
use tokio::sync::oneshot;
use log::{error};
use actix_raft::{
    AppData,
    AppError,
    AppDataResponse,
    messages,
};

use crate::network::{Node};
use crate::raft::MemRaft;
use crate::server;

pub trait RemoteMessage: Message + Send + Serialize + DeserializeOwned
where Self::Result: Send + Serialize + DeserializeOwned {
    fn type_id() -> &'static str;
}

/// DistributeMessage(Message)
pub struct SendRemoteMessage<M>(pub M)
where M: RemoteMessage + 'static,
      M::Result: Send + Serialize + DeserializeOwned;

impl<M> Message for SendRemoteMessage<M>
where M: RemoteMessage + 'static,
      M::Result: Send + Serialize + DeserializeOwned
{
    type Result = M::Result;
}

pub struct RemoteMessageResult<M>
    where M: RemoteMessage + 'static,
          M::Result: Send + Serialize + DeserializeOwned
{
    pub rx: oneshot::Receiver<String>,
    pub m: PhantomData<M>,
}

impl<M> MessageResponse<Node, SendRemoteMessage<M>> for RemoteMessageResult<M>
where
      M: RemoteMessage + 'static,
      M::Result: Send + Serialize + DeserializeOwned
{
    fn handle<R: ResponseChannel<SendRemoteMessage<M>>>(self, _: &mut Context<Node>, tx: Option<R>) {
        Arbiter::spawn(
            self.rx
                .map_err(|e| error!("{:?}", e))
                .and_then(move |msg| {
                    // Raft node has not been initialized yet
                    if msg == "" {
                        return Err(());
                    }

                    let msg = serde_json::from_slice::<M::Result>(msg.as_ref()).unwrap();
                    if let Some(tx) = tx {
                        let _ = tx.send(msg);
                    }
                    Ok(())
                })
        );
    }
}

#[derive(Message, Clone)]
pub struct RegisterHandler(pub Addr<MemRaft>);

/// Impl RemoteMessage for RaftNetwork messages
impl<D: AppData> RemoteMessage for messages::AppendEntriesRequest<D> {
    fn type_id() -> &'static str { "AppendEntriesRequest" }
}

impl RemoteMessage for messages::VoteRequest {
    fn type_id() -> &'static str { "VoteRequest" }
}

impl RemoteMessage for messages::InstallSnapshotRequest {
    fn type_id() -> &'static str { "InstallSnapshotRequest" }
}

impl<D: AppData, R: AppDataResponse, E: AppError> RemoteMessage for messages::ClientPayload<D, R, E> {
    fn type_id() -> &'static str { "ClientPayload" }
}

/// Impl RemoteMessage for Application Messages
impl RemoteMessage for server::Join {
    fn type_id() -> &'static str { "Join" }
}

impl RemoteMessage for server::SendRoom {
    fn type_id() -> &'static str { "SendRoom" }
}

impl RemoteMessage for server::SendRecipient {
    fn type_id() -> &'static str { "SendRecipient" }
}

impl RemoteMessage for server::CreateRoom {
    fn type_id() -> &'static str { "CreateRoom" }
}

impl RemoteMessage for server::GetMembers {
    fn type_id() -> &'static str { "GetMembers" }
}
