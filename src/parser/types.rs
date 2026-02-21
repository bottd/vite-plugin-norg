use napi_derive::napi;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Html,
    Svelte,
    Vue,
    React,
}

impl OutputMode {
    pub const ALL: [Self; 4] = [Self::Html, Self::Svelte, Self::Vue, Self::React];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Html => "html",
            Self::Svelte => "svelte",
            Self::Vue => "vue",
            Self::React => "react",
        }
    }
}

impl std::str::FromStr for OutputMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "html" => Ok(Self::Html),
            "svelte" => Ok(Self::Svelte),
            "vue" => Ok(Self::Vue),
            "react" => Ok(Self::React),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for OutputMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TocEntry {
    pub level: u32,
    pub title: String,
    pub id: String,
}

/// An inline block extracted from an @inline tag
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineComponent {
    /// Position of this inline component in the document (0-indexed)
    pub index: u32,
    /// Target mode ("html" | "svelte" | "vue" | "react")
    pub mode: String,
    /// Raw component code (user writes full component with imports)
    pub code: String,
}
