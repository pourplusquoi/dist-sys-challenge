use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub(crate) enum Request {
    Echo(EchoRequest),
    Generate,
    Broadcast(BroadcastRequest),
    Read,
    Topology(TopologyRequest),
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct EchoRequest {
    pub(crate) echo: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct BroadcastRequest {
    pub(crate) message: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct TopologyRequest {
    pub(crate) topology: HashMap<String, Vec<String>>,
}
