use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub(crate) enum Request {
    Echo(EchoRequest),
    Generate,
    // Init,
    // Read,
    // Add { element: i64 },
    // ReplicateOne { element: i64 },
    // ReplicateFull { value: Vec<i64> },
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct EchoRequest {
    pub(crate) echo: String,
}
