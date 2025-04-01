use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicU64, Ordering}, Arc, Mutex, RwLock
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
    received: RwLock<HashMap<String, Mutex<(u64, HashSet<u64>)>>>,
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
            received: RwLock::new(HashMap::new()),
        }
    }

    async fn add(&self, runtime: &Runtime, req: AddRequest) -> Result<Response> {
        let serial = self.serial.fetch_add(1, Ordering::SeqCst);
        self.value.fetch_add(req.delta, Ordering::SeqCst);
        let from = runtime.node_id();
        for to in runtime.neighbours() {
            self.broadcast_inner(runtime, from, to, serial, req.delta);
        }
        Ok(Response::AddOk)
    }

    async fn read(&self) -> Result<Response> {
        let value = self.value.load(Ordering::SeqCst);
        let resp = ReadOkResponse { value };
        Ok(Response::ReadOk(resp))
    }

    async fn broadcast(&self, runtime: &Runtime, req: BroadcastRequest) -> Result<Response> {
        if req.from == runtime.node_id() {
            return Ok(Response::BroadcastOk);
        }
        {
            self.received.write().unwrap().entry(req.from.clone()).or_default();
            let guard = self.received.read().unwrap();
            let mut latest = guard.get(&req.from).unwrap().lock().unwrap();
            if req.serial < latest.0 + 1 {
                return Ok(Response::BroadcastOk);
            }
            if req.serial > latest.0 + 1 {
                latest.1.insert(req.serial);
            } else {
                latest.0 = req.serial;
                latest.1.remove(&req.serial);
                for x in (req.serial + 1).. {
                    if !latest.1.remove(&x) {
                        break;
                    }
                    latest.0 = x;
                }
            }
        }
        self.value.fetch_add(req.delta, Ordering::SeqCst);
        let from = req.from.as_str();
        for to in runtime.neighbours() {
            self.broadcast_inner(runtime, from, to, req.serial, req.delta);
        }
        Ok(Response::BroadcastOk)
    }

    fn broadcast_inner(
        &self,
        runtime: &Runtime,
        from: impl Into<String>,
        to: impl Into<String>,
        serial: u64,
        delta: u64,
    ) {
        let from = from.into();
        let to = to.into();
        let rt = runtime.clone();
        let req = Request::Broadcast(BroadcastRequest {
            from,
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
