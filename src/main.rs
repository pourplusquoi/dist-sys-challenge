use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use async_trait::async_trait;
use maelstrom::{Node, Result, Runtime, done, protocol::Message};
use tokio_context::context::Context;

pub(crate) mod req;
pub(crate) mod resp;

use crate::{
    req::{AddRequest, BroadcastRequest, Request},
    resp::{ReadOkResponse, Response},
};

pub(crate) fn main() -> Result<()> {
    Runtime::init(try_main())
}

async fn try_main() -> Result<()> {
    let handler = Arc::new(Handler::new());
    Runtime::new().with_handler(handler).run().await
}

struct Handler {
    value: AtomicU64,
    serial: AtomicU64,
    recved: Mutex<HashMap<String, u64>>,
}

#[async_trait]
impl Node for Handler {
    async fn process(&self, runtime: Runtime, req: Message) -> Result<()> {
        match req.body.as_obj::<Request>() {
            Ok(r) => {
                let resp = match r {
                    Request::Add(req) => self.add(&runtime, req).await,
                    Request::Read => self.read().await,
                    Request::Broadcast(req) => self.broadcast(&runtime, req).await,
                }?;
                runtime.reply(req, resp).await
            }
            _ => done(runtime, req),
        }
    }
}

impl Handler {
    fn new() -> Self {
        Self {
            value: AtomicU64::new(0),
            serial: AtomicU64::new(1),
            recved: Mutex::new(HashMap::new()),
        }
    }

    async fn add(&self, runtime: &Runtime, req: AddRequest) -> Result<Response> {
        let serial = self.serial.fetch_add(1, Ordering::Relaxed);
        self.value.fetch_add(req.delta, Ordering::Relaxed);
        for to in runtime.neighbours() {
            self.broadcast_inner(runtime, to, serial, req.delta);
        }
        Ok(Response::AddOk)
    }

    async fn read(&self) -> Result<Response> {
        let value = self.value.load(Ordering::Relaxed);
        let resp = ReadOkResponse { value };
        Ok(Response::ReadOk(resp))
    }

    async fn broadcast(&self, runtime: &Runtime, req: BroadcastRequest) -> Result<Response> {
        let mut guard = self.recved.lock().unwrap();
        let latest = guard.entry(req.node).or_default();
        if req.serial <= *latest {
            return Ok(Response::BroadcastOk);
        }
        if req.serial > *latest + 1 {
            return Ok(Response::BroadcastErr);
        }
        self.value.fetch_add(req.delta, Ordering::Relaxed);
        for to in runtime.neighbours() {
            self.broadcast_inner(runtime, to, req.serial, req.delta);
        }
        Ok(Response::BroadcastOk)
    }

    fn broadcast_inner(&self, runtime: &Runtime, to: &str, serial: u64, delta: u64) {
        let from = runtime.node_id().to_string();
        let to = to.to_string();
        let rt = runtime.clone();
        let req = Request::Broadcast(BroadcastRequest {
            node: from,
            serial,
            delta,
        });
        runtime.spawn(async move {
            loop {
                let (ctx, _) = Context::with_timeout(Duration::from_secs(1));
                if let Ok(msg) = rt.call(ctx, to.clone(), &req).await {
                    if let Ok(Response::BroadcastOk) = msg.body.as_obj::<Response>() {
                        break;
                    }
                }
            }
        });
    }
}
