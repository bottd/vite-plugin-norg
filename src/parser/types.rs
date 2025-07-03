use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TocEntry {
    pub level: usize,
    pub title: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedNorg {
    pub metadata: Value,
    pub html: String,
    pub toc: Vec<TocEntry>,
}
