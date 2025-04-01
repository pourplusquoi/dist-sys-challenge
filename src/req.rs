use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub(crate) enum Request {
    Add(AddRequest),
    Read,
    Broadcast(BroadcastRequest),
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct AddRequest {
    pub(crate) delta: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct BroadcastRequest {
    pub(crate) node: String,
    pub(crate) serial: u64,
    pub(crate) delta: u64,
}
