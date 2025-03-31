use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use async_trait::async_trait;
use maelstrom::{Node, Result, Runtime, done, protocol::Message};
use resp::EchoOkResponse;

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
    let handler = Arc::new(Handler::default());
    Runtime::new().with_handler(handler).run().await
}

#[derive(Default)]
struct Handler {
    next: AtomicU64,
}

#[async_trait]
impl Node for Handler {
    async fn process(&self, runtime: Runtime, req: Message) -> Result<()> {
        match req.body.as_obj::<Request>() {
            Ok(r) => {
                let resp = match r {
                    Request::Echo(req) => self.echo(req).await,
                    Request::Generate => self.generate(runtime.node_id()).await,
                }?;
                runtime.reply(req, resp).await
            }
            _ => done(runtime, req),
        }
    }
}

impl Handler {
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
}
