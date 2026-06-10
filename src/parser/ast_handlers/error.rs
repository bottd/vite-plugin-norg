use crate::types::OutputMode;

#[derive(Debug)]
pub enum EmbedParseError {
    MissingLanguage {
        index: usize,
    },
    InvalidLanguage {
        index: usize,
        language: String,
    },
    LanguageMismatch {
        index: usize,
        language: String,
        mode: OutputMode,
    },
    /// A non-list node (an embed, code block, heading, …) nested inside a list
    /// item. The list renderer is a pure function with no access to the
    /// embed/CSS stream, so such content cannot be rendered in place — and is
    /// reported rather than silently dropped. `node` names the unsupported kind.
    UnsupportedListItemContent {
        node: &'static str,
    },
}

impl EmbedParseError {
    /// The zero-based ordinal of the offending `@embed`, for errors that have
    /// one.
    pub fn index(&self) -> Option<usize> {
        match self {
            Self::MissingLanguage { index }
            | Self::InvalidLanguage { index, .. }
            | Self::LanguageMismatch { index, .. } => Some(*index),
            Self::UnsupportedListItemContent { .. } => None,
        }
    }

    /// The offending `@embed` declaration, reconstructed from the parsed
    /// language — not the source line verbatim, so any extra parameters the
    /// author wrote are not echoed back. Rebuilding it from AST data (rather
    /// than re-scanning the source text by ordinal) cannot mis-attribute the
    /// error to an `@embed` line sitting inside another verbatim block's raw
    /// content.
    pub fn offending_line(&self) -> Option<String> {
        match self {
            Self::MissingLanguage { .. } | Self::UnsupportedListItemContent { .. } => None,
            Self::InvalidLanguage { language, .. } | Self::LanguageMismatch { language, .. } => {
                Some(format!("@embed {language}"))
            }
        }
    }
}

impl std::fmt::Display for EmbedParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let supported = OutputMode::ALL.map(|m| m.as_str()).join(", ");
        let n = self.index().map(|i| i + 1).unwrap_or(0);
        match self {
            Self::MissingLanguage { .. } => write!(
                f,
                "Embed error (embed #{n}): missing language. Supported languages: {supported}"
            ),
            Self::InvalidLanguage { language, .. } => write!(
                f,
                "Embed error (embed #{n}): invalid language \"{language}\". Supported languages: {supported}"
            ),
            Self::LanguageMismatch { language, mode, .. } => write!(
                f,
                "Embed error (embed #{n}): @embed {language} cannot be used in {mode} mode"
            ),
            Self::UnsupportedListItemContent { node } => write!(
                f,
                "Unsupported content inside a list item: {node}. Move it out of the list \
                 — embeds and block content cannot be rendered inside a list item."
            ),
        }
    }
}

impl std::error::Error for EmbedParseError {}
