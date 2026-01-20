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

/// An inline framework component (Svelte, Vue, React) extracted from an @inline tag
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineComponent {
    /// Position of this inline component in the document (0-indexed)
    pub index: u32,
    /// Framework type ("svelte" | "react" | "vue")
    pub framework: String,
    /// Raw component code (user writes full component with imports)
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedNorg {
    pub metadata: Value,
    pub html_parts: Vec<String>,
    pub toc: Vec<TocEntry>,
    pub inlines: Vec<InlineComponent>,
}
