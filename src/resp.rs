use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub(crate) enum Response {
    EchoOk(EchoOkResponse),
    GenerateOk(GenerateOkResponse),
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct EchoOkResponse {
    pub(crate) echo: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct GenerateOkResponse {
    pub(crate) id: String,
}
