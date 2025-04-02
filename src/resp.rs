use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[allow(clippy::enum_variant_names)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub(crate) enum Response {
    SendOk(SendResponse),
    PollOk(PollResponse),
    CommitOffsetsOk,
    ListCommittedOffsetsOk(ListCommittedOffsetsResponse),
    ReplicateSendOk,
    ReplicateCommitOffsetsOk,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct SendResponse {
    pub(crate) offset: u64,
}

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct PollResponse {
    pub(crate) msgs: HashMap<String, Vec<(u64, u64)>>,
}

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct ListCommittedOffsetsResponse {
    pub(crate) offsets: HashMap<String, u64>,
}
