use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "payload")]
pub enum IpcCommand {
    Toggle,
    Show,
    Hide,
    Paste { id: i64 },
    Status,
    Quit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", content = "data")]
pub enum IpcResponse {
    Ok { message: String },
    Status { running: bool, item_count: usize },
    Error { message: String },
}
