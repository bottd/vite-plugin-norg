use super::error::EmbedParseError;
use crate::types::{EmbedComponent, OutputMode};
use arborium::Highlighter;
use htmlescape::encode_minimal;
use itertools::Itertools;
use textwrap::dedent;

pub enum VerbatimTagResult {
    Html(String),
    Css(String),
    Embed(EmbedComponent),
}

pub enum VerbatimTag {
    Code,
    Image,
    Embed,
    DocumentMeta,
    Unknown,
}

impl From<&[String]> for VerbatimTag {
    fn from(name: &[String]) -> Self {
        match name {
            [tag] if tag == "code" => Self::Code,
            [tag] if tag == "image" => Self::Image,
            [tag] if tag == "embed" => Self::Embed,
            [doc, meta] if doc == "document" && meta == "meta" => Self::DocumentMeta,
            _ => Self::Unknown,
        }
    }
}

impl VerbatimTag {
    pub fn render(
        self,
        parameters: &[String],
        content: &str,
        mode: Option<OutputMode>,
        highlighter: &mut Highlighter,
        embed_index: usize,
    ) -> Result<Option<VerbatimTagResult>, EmbedParseError> {
        match self {
            Self::Code => {
                let code = dedent(content);
                let lang = parameters
                    .first()
                    .filter(|s| !s.is_empty())
                    .map(String::as_str)
                    .unwrap_or("text");

                let highlighted = highlighter.highlight(lang, &code);
                let html = match highlighted {
                    Ok(h) => format!(
                        r#"<pre class="arborium lang-{lang}"><code>{}</code></pre>"#,
                        wrap_lines(&h)
                    ),
                    Err(_) => format!(
                        r#"<pre><code>{}</code></pre>"#,
                        wrap_lines(&encode_minimal(&code))
                    ),
                };
                Ok(Some(VerbatimTagResult::Html(html)))
            }
            Self::Image => Ok(parameters.first().filter(|s| !s.is_empty()).map(|path| {
                let src = if path.starts_with('/') || path.starts_with("http") {
                    path.clone()
                } else {
                    format!("./{path}")
                };
                VerbatimTagResult::Html(format!(
                    r#"<img src="{}" alt="{}" />"#,
                    encode_minimal(&src),
                    encode_minimal(content.trim())
                ))
            })),
            Self::Embed => {
                let embed_lang = parameters
                    .first()
                    .filter(|s| !s.is_empty())
                    .map(String::as_str);

                match embed_lang {
                    Some("css") => Ok(Some(VerbatimTagResult::Css(content.to_string()))),
                    None => Err(EmbedParseError::MissingLanguage { index: embed_index }),
                    Some(lang) => {
                        let embed_mode = lang.parse::<OutputMode>().map_err(|_| {
                            EmbedParseError::InvalidLanguage {
                                index: embed_index,
                                language: lang.to_string(),
                            }
                        })?;

                        match mode {
                            None => Ok(None),
                            Some(m) if m != embed_mode => Err(EmbedParseError::LanguageMismatch {
                                index: embed_index,
                                language: lang.to_string(),
                                mode: m,
                            }),
                            Some(_) => Ok(Some(VerbatimTagResult::Embed(EmbedComponent {
                                index: 0,
                                mode: embed_mode.to_string(),
                                code: content.to_string(),
                            }))),
                        }
                    }
                }
            }
            Self::DocumentMeta => Ok(None),
            Self::Unknown => Ok(Some(VerbatimTagResult::Html(format!(
                r#"<div class="verbatim">{}</div>"#,
                encode_minimal(content)
            )))),
        }
    }
}

/// Wraps each of highlighted HTML in `<span class="line">`
/// This enables per-line styling such as line numbers or highlighting specific lines
fn wrap_lines(html: &str) -> String {
    html.lines()
        .map(|line| format!(r#"<span class="line">{line}</span>"#))
        .join("")
}
