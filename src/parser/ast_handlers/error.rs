use crate::types::OutputMode;

#[derive(Debug)]
pub enum InlineParseError {
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
}

impl InlineParseError {
    pub fn index(&self) -> usize {
        match self {
            Self::MissingLanguage { index }
            | Self::InvalidLanguage { index, .. }
            | Self::LanguageMismatch { index, .. } => *index,
        }
    }
}

impl std::fmt::Display for InlineParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let n = self.index() + 1;
        let supported = OutputMode::ALL.map(|m| m.as_str()).join(", ");
        match self {
            Self::MissingLanguage { .. } => write!(
                f,
                "Inline error (inline #{n}): missing language. Supported languages: {supported}"
            ),
            Self::InvalidLanguage { language, .. } => write!(
                f,
                "Inline error (inline #{n}): invalid language \"{language}\". Supported languages: {supported}"
            ),
            Self::LanguageMismatch { language, mode, .. } => write!(
                f,
                "Inline error (inline #{n}): @inline {language} cannot be used in {mode} mode"
            ),
        }
    }
}
