use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocEntry {
    pub level: usize,
    pub title: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedNorg {
    pub ast: Value,
    pub metadata: Value,
    pub html: String,
    pub toc: Vec<TocEntry>,
}

impl ParsedNorg {
    pub fn has_metadata(&self) -> bool {
        self.metadata
            .as_object()
            .is_some_and(|metadata| !metadata.is_empty())
    }

    pub fn has_toc(&self) -> bool {
        !self.toc.is_empty()
    }
}
