use napi_derive::napi;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

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

impl FromStr for OutputMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL.into_iter().find(|m| m.as_str() == s).ok_or(())
    }
}

impl fmt::Display for OutputMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

/// An embed block extracted from an @embed tag
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedComponent {
    /// Target mode ("html" | "svelte" | "vue" | "react")
    pub mode: String,
    /// Raw component code (user writes full component with imports)
    pub code: String,
}
