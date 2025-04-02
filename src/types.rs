use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub(crate) enum Request {
    Txn(TxnRequest),
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct TxnRequest {
    pub(crate) txn: Vec<(Operation, u64, Option<u64>)>,
}

#[allow(clippy::enum_variant_names)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub(crate) enum Response {
    TxnOk(TxnResponse),
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TxnResponse {
    pub(crate) txn: Vec<(Operation, u64, Option<u64>)>,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) enum Operation {
    #[serde(rename = "r")]
    Read,
    #[serde(rename = "w")]
    Write,
}
