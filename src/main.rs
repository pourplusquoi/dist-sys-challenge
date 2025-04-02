use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, RwLock},
    time::Duration,
};

use async_trait::async_trait;
use maelstrom::{Node, Result, Runtime, done, protocol::Message};
use serde::Serialize;
use tokio::sync::OnceCell;
use tokio_context::context::Context;

pub(crate) mod req;
pub(crate) mod resp;

use crate::{
    req::{
        CommitOffsetsRequest, ListCommittedOffsetsRequest, PollRequest,
        ReplicateCommitOffsetsRequest, ReplicateSendRequest, Request, SendRequest,
    },
    resp::{ListCommittedOffsetsResponse, PollResponse, Response, SendResponse},
};

pub(crate) fn main() -> Result<()> {
    Runtime::init(try_main())
}

async fn try_main() -> Result<()> {
    let runtime = Runtime::new();
    let handler = Arc::new(Handler::default());
    runtime.with_handler(handler).run().await
}

#[derive(PartialEq, Eq)]
enum Role {
    Leader,
    Follower,
}

#[derive(Default)]
struct Handler {
    entries: RwLock<HashMap<String, Entry>>,
    once: OnceCell<(Role, String)>,
}

#[derive(Default)]
struct Entry {
    commited: u64,
    latest: u64,
    logs: BTreeMap<u64, u64>,
}

#[async_trait]
impl Node for Handler {
    async fn process(&self, runtime: Runtime, req: Message) -> Result<()> {
        match req.body.as_obj::<Request>() {
            Ok(r) => {
                let resp = match r {
                    Request::Send(req) => self.send(&runtime, req).await,
                    Request::Poll(req) => self.poll(req).await,
                    Request::CommitOffsets(req) => self.commit_offsets(&runtime, req).await,
                    Request::ListCommittedOffsets(req) => self.list_committed_offsets(req).await,
                    Request::ReplicateSend(req) => self.replicate_send(req).await,
                    Request::ReplicateCommitOffsets(req) => {
                        self.replicate_commit_offsets(req).await
                    }
                }?;
                runtime.reply(req, resp).await
            }
            _ => done(runtime, req),
        }
    }
}

impl Handler {
    async fn send(&self, runtime: &Runtime, req: SendRequest) -> Result<Response> {
        let (role, leader) = self
            .once
            .get_or_try_init(|| Self::elect_leader(runtime))
            .await?;
        match role {
            Role::Leader => {
                let offset = self.send_inner(req.key.clone(), req.msg).await?;

                let req = ReplicateSendRequest {
                    key: req.key,
                    msg: req.msg,
                };
                for to in runtime.neighbours() {
                    let to = to.clone();
                    let rt = runtime.clone();
                    let req = req.clone();
                    runtime.spawn(async move {
                        loop {
                            if let Ok(Response::ReplicateSendOk) =
                                Self::call(&rt, &to, req.clone()).await
                            {
                                break;
                            }
                        }
                    });
                }

                let resp = SendResponse { offset };
                Ok(Response::SendOk(resp))
            }
            Role::Follower => Self::call(runtime, leader, req).await,
        }
    }

    async fn replicate_send(&self, req: ReplicateSendRequest) -> Result<Response> {
        self.send_inner(req.key, req.msg).await?;
        Ok(Response::ReplicateSendOk)
    }

    async fn send_inner(&self, key: String, msg: u64) -> Result<u64> {
        let mut guard = self.entries.write().unwrap();
        let entry = guard.entry(key).or_default();
        entry.latest += 1;
        entry.logs.insert(entry.latest, msg);
        Ok(entry.latest)
    }

    async fn poll(&self, req: PollRequest) -> Result<Response> {
        const LIMIT: usize = 8;
        let mut resp = PollResponse::default();
        let guard = self.entries.read().unwrap();
        for (key, offset) in req.offsets {
            let mut msgs = Vec::with_capacity(LIMIT);
            if let Some(entry) = guard.get(&key) {
                for log in entry
                    .logs
                    .iter()
                    .skip_while(|log| *log.0 < offset)
                    .take(LIMIT)
                    .map(|(x, y)| (*x, *y))
                {
                    msgs.push(log);
                }
            }
            resp.msgs.insert(key, msgs);
        }
        Ok(Response::PollOk(resp))
    }

    async fn commit_offsets(
        &self,
        runtime: &Runtime,
        req: CommitOffsetsRequest,
    ) -> Result<Response> {
        let (role, leader) = self
            .once
            .get_or_try_init(|| Self::elect_leader(runtime))
            .await?;
        match role {
            Role::Leader => {
                self.commit_offsets_inner(req.offsets.clone()).await?;

                let req = ReplicateCommitOffsetsRequest {
                    offsets: req.offsets,
                };
                for to in runtime.neighbours() {
                    let to = to.clone();
                    let rt = runtime.clone();
                    let req = req.clone();
                    runtime.spawn(async move {
                        loop {
                            if let Ok(Response::ReplicateCommitOffsetsOk) =
                                Self::call(&rt, &to, req.clone()).await
                            {
                                break;
                            }
                        }
                    });
                }

                Ok(Response::CommitOffsetsOk)
            }
            Role::Follower => Self::call(runtime, leader, req).await,
        }
    }

    async fn replicate_commit_offsets(
        &self,
        req: ReplicateCommitOffsetsRequest,
    ) -> Result<Response> {
        self.commit_offsets_inner(req.offsets).await?;
        Ok(Response::ReplicateCommitOffsetsOk)
    }

    async fn commit_offsets_inner(&self, offsets: HashMap<String, u64>) -> Result<()> {
        let mut guard = self.entries.write().unwrap();
        for (key, offset) in offsets {
            if let Some(entry) = guard.get_mut(&key) {
                entry.commited = offset;
            }
        }
        Ok(())
    }

    async fn list_committed_offsets(&self, req: ListCommittedOffsetsRequest) -> Result<Response> {
        let mut resp = ListCommittedOffsetsResponse::default();
        let guard = self.entries.read().unwrap();
        for key in req.keys {
            if let Some(entry) = guard.get(&key) {
                resp.offsets.insert(key, entry.commited);
            }
        }
        Ok(Response::ListCommittedOffsetsOk(resp))
    }

    async fn call(
        runtime: &Runtime,
        to: impl Into<String>,
        req: impl Serialize,
    ) -> Result<Response> {
        let (ctx, _) = Context::with_timeout(Duration::from_secs(1));
        let msg = runtime.call(ctx, to, req).await?;
        msg.body.as_obj::<Response>()
    }

    async fn elect_leader(runtime: &Runtime) -> Result<(Role, String)> {
        // TODO(panyang): Hard coded.
        let leader = "n0".to_string();
        let curr = runtime.node_id();
        let role = if leader == curr {
            Role::Leader
        } else {
            Role::Follower
        };
        Ok((role, leader))
    }
}
