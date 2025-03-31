use std::{
    collections::HashSet,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use async_trait::async_trait;
use maelstrom::{Node, Result, Runtime, done, protocol::Message};
use req::{BroadcastRequest, TopologyRequest};
use resp::{EchoOkResponse, ReadOkResponse};
use tokio_context::context::Context;

pub(crate) mod req;
pub(crate) mod resp;

use crate::{
    req::{EchoRequest, Request},
    resp::{GenerateOkResponse, Response},
};

pub(crate) fn main() -> Result<()> {
    Runtime::init(try_main())
}

async fn try_main() -> Result<()> {
    let handler = Arc::new(Handler::new());
    Runtime::new().with_handler(handler).run().await
}

#[derive(Default)]
struct Handler {
    next: AtomicU64,
    messages: Mutex<HashSet<u64>>,
    neigbors: Mutex<Vec<String>>,
}

#[async_trait]
impl Node for Handler {
    async fn process(&self, runtime: Runtime, req: Message) -> Result<()> {
        match req.body.as_obj::<Request>() {
            Ok(r) => {
                let resp = match r {
                    Request::Echo(req) => self.echo(req).await,
                    Request::Generate => self.generate(runtime.node_id()).await,
                    Request::Broadcast(req) => self.broadcast(&runtime, req).await,
                    Request::Read => self.read().await,
                    Request::Topology(req) => self.topology(&runtime, req).await,
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
            next: AtomicU64::new(0),
            messages: Mutex::new(HashSet::new()),
            neigbors: Mutex::new(Vec::new()),
        }
    }

    async fn echo(&self, req: EchoRequest) -> Result<Response> {
        let resp = EchoOkResponse { echo: req.echo };
        Ok(Response::EchoOk(resp))
    }

    async fn generate(&self, node_id: &str) -> Result<Response> {
        let num = self.next.fetch_add(1, Ordering::SeqCst);
        let id = format!("{node_id}::{num}");
        let resp = GenerateOkResponse { id };
        Ok(Response::GenerateOk(resp))
    }

    async fn broadcast(&self, runtime: &Runtime, req: BroadcastRequest) -> Result<Response> {
        let message = req.message;
        if self.messages.lock().unwrap().insert(message) {
            for to in self.neigbors.lock().unwrap().iter() {
                let to = to.clone();
                let rt = runtime.clone();
                runtime.spawn(async move {
                    loop {
                        let (ctx, _) = Context::with_timeout(Duration::from_secs(1));
                        let req = Request::Broadcast(BroadcastRequest { message });
                        if let Ok(_) = rt.call(ctx, &to, req).await {
                            break;
                        }
                    }
                });
            }
        }
        Ok(Response::BroadcastOk)
    }

    async fn read(&self) -> Result<Response> {
        let messages = self
            .messages
            .lock()
            .unwrap()
            .iter()
            .copied()
            .collect::<Vec<_>>();
        let resp = ReadOkResponse { messages };
        Ok(Response::ReadOk(resp))
    }

    async fn topology(&self, runtime: &Runtime, mut req: TopologyRequest) -> Result<Response> {
        if let Some(neigbors) = req.topology.remove(runtime.node_id()) {
            *self.neigbors.lock().unwrap() = neigbors;
        }
        Ok(Response::TopologyOk)
    }
}
