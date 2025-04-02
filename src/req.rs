use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub(crate) enum Request {
    Send(SendRequest),
    Poll(PollRequest),
    CommitOffsets(CommitOffsetsRequest),
    ListCommittedOffsets(ListCommittedOffsetsRequest),
    ReplicateSend(ReplicateSendRequest),
    ReplicateCommitOffsets(ReplicateCommitOffsetsRequest),
}

#[derive(Serialize, Deserialize)]
pub(crate) struct SendRequest {
    pub(crate) key: String,
    pub(crate) msg: u64,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct PollRequest {
    pub(crate) offsets: HashMap<String, u64>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct CommitOffsetsRequest {
    pub(crate) offsets: HashMap<String, u64>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ListCommittedOffsetsRequest {
    pub(crate) keys: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct ReplicateSendRequest {
    pub(crate) key: String,
    pub(crate) msg: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct ReplicateCommitOffsetsRequest {
    pub(crate) offsets: HashMap<String, u64>,
}
