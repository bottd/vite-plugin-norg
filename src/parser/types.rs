use napi_derive::napi;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TocEntry {
    pub level: u32,
    pub title: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedNorg {
    pub metadata: Value,
    pub html: String,
    pub toc: Vec<TocEntry>,
}
