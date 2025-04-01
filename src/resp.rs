use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub(crate) enum Response {
    AddOk,
    ReadOk(ReadOkResponse),
    BroadcastOk,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct ReadOkResponse {
    pub(crate) value: u64,
}
