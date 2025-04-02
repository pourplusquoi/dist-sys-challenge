use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use maelstrom::{
    Error, Node, Result, Runtime, done,
    kv::{KV, Storage},
    protocol::Message,
};
use tokio_context::context::Context;

pub(crate) mod types;

use types::{Operation, Request, Response, TxnRequest, TxnResponse};

pub(crate) fn main() -> Result<()> {
    Runtime::init(try_main())
}

async fn try_main() -> Result<()> {
    let runtime = Runtime::new();
    let handler = Arc::new(Handler::new(runtime.clone()));
    runtime.with_handler(handler).run().await
}

struct Handler {
    store: Storage,
}

#[async_trait]
impl Node for Handler {
    async fn process(&self, runtime: Runtime, req: Message) -> Result<()> {
        match req.body.as_obj::<Request>() {
            Ok(r) => {
                let resp = match r {
                    Request::Txn(req) => self.txn(req).await,
                }?;
                runtime.reply(req, resp).await
            }
            _ => done(runtime, req),
        }
    }
}

impl Handler {
    fn new(runtime: Runtime) -> Self {
        Self {
            store: maelstrom::kv::lin_kv(runtime),
        }
    }

    async fn txn(&self, req: TxnRequest) -> Result<Response> {
        let mut txn = Vec::new();
        let mut workspace = HashMap::new();
        let lock = Lock::new(&self.store, "txn".into());
        lock.acquire().await?;
        for (op, key, val) in req.txn {
            match op {
                Operation::Read => {
                    let mut val = None;
                    if let Some(v) = workspace.get(&key) {
                        val = Some(*v);
                    } else {
                        let (ctx, _h) = Context::new();
                        if let Ok(v) = self.store.get::<u64>(ctx, format!("{key}")).await {
                            val = Some(v);
                        }
                    }
                    txn.push((Operation::Read, key, val));
                }
                Operation::Write => {
                    workspace.insert(key, val.unwrap());
                    txn.push((Operation::Write, key, val));
                }
            }
        }
        if !workspace.is_empty() {
            for (key, val) in workspace {
                let (ctx, _h) = Context::new();
                self.store.put(ctx, format!("{key}"), val).await?;
            }
        }
        lock.release().await?;
        Ok(Response::TxnOk(TxnResponse { txn }))
    }
}

fn is_precondition_failed(e: &(dyn std::error::Error + Send + Sync + 'static)) -> bool {
    e.downcast_ref::<Error>() == Some(&Error::PreconditionFailed)
}

struct Lock<'a> {
    store: &'a Storage,
    key: String,
}

impl<'a> Lock<'a> {
    fn new(store: &'a Storage, key: String) -> Self {
        Self { store, key }
    }

    async fn acquire(&self) -> Result<()> {
        loop {
            match self.try_acquire().await {
                Err(e) if is_precondition_failed(&*e) => {
                    std::thread::yield_now();
                }
                r => break r,
            }
        }
    }

    async fn try_acquire(&self) -> Result<()> {
        let (ctx, _h) = Context::new();
        self.store
            .cas(ctx, self.key.clone(), "", "locked", true)
            .await?;
        Ok(())
    }

    async fn release(&self) -> Result<()> {
        let (ctx, _h) = Context::new();
        self.store.put(ctx, self.key.clone(), "").await?;
        Ok(())
    }
}
