use napi_derive::napi;
use serde::{Deserialize, Serialize};

#[napi(string_enum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum OutputMode {
    html,
    svelte,
    vue,
    react,
}

impl OutputMode {
    pub const ALL: [Self; 4] = [Self::html, Self::svelte, Self::vue, Self::react];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::html => "html",
            Self::svelte => "svelte",
            Self::vue => "vue",
            Self::react => "react",
        }
    }
}

impl std::str::FromStr for OutputMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "html" => Ok(Self::html),
            "svelte" => Ok(Self::svelte),
            "vue" => Ok(Self::vue),
            "react" => Ok(Self::react),
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
