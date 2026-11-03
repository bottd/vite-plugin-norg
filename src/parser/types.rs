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

#[napi(string_enum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NorgErrorCode {
    #[napi(value = "ERR_NORG_PARSE")]
    Parse,
    #[napi(value = "ERR_NORG_EMBED")]
    Embed,
    #[napi(value = "ERR_NORG_INTERNAL")]
    Internal,
}

impl NorgErrorCode {
    pub fn error(self, reason: impl ToString) -> napi::Error<Self> {
        napi::Error::new(self, reason)
    }
}

impl AsRef<str> for NorgErrorCode {
    fn as_ref(&self) -> &str {
        match self {
            Self::Parse => "ERR_NORG_PARSE",
            Self::Embed => "ERR_NORG_EMBED",
            Self::Internal => "ERR_NORG_INTERNAL",
        }
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
    /// Position of this component among component embeds (0-indexed)
    pub index: u32,
    /// Target mode ("html" | "svelte" | "vue" | "react")
    pub mode: String,
    /// Raw component code (user writes full component with imports)
    pub code: String,
}
