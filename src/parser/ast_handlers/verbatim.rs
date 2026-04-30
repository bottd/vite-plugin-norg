use super::error::EmbedParseError;
use crate::types::OutputMode;
use arborium::Highlighter;
use htmlescape::encode_minimal;
use textwrap::dedent;

pub enum VerbatimTagResult {
    Html(String),
    Css(String),
    Embed { mode: String, code: String },
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
        let first_param = || {
            parameters
                .first()
                .filter(|s| !s.is_empty())
                .map(String::as_str)
        };

        match self {
            Self::Code => {
                let code = dedent(content);
                let lang = first_param().unwrap_or("text");
                let body = match highlighter.highlight(lang, &code) {
                    Ok(highlighted) => format!(
                        r#"<pre class="arborium lang-{lang}"><code>{}</code></pre>"#,
                        wrap_lines(&highlighted)
                    ),
                    Err(_) => format!(
                        r#"<pre><code>{}</code></pre>"#,
                        wrap_lines(&encode_minimal(&code))
                    ),
                };
                Ok(Some(VerbatimTagResult::Html(body)))
            }

            Self::Image => Ok(first_param().map(|path| {
                let src = if path.starts_with('/') || path.starts_with("http") {
                    path.to_string()
                } else {
                    format!("./{path}")
                };
                VerbatimTagResult::Html(format!(
                    r#"<img src="{}" alt="{}" />"#,
                    encode_minimal(&src),
                    encode_minimal(content.trim())
                ))
            })),

            Self::Embed => render_embed(first_param(), content, mode, embed_index),

            Self::DocumentMeta => Ok(None),

            Self::Unknown => Ok(Some(VerbatimTagResult::Html(format!(
                r#"<div class="verbatim">{}</div>"#,
                encode_minimal(content)
            )))),
        }
    }
}

fn render_embed(
    lang: Option<&str>,
    content: &str,
    mode: Option<OutputMode>,
    index: usize,
) -> Result<Option<VerbatimTagResult>, EmbedParseError> {
    let Some(lang) = lang else {
        return Err(EmbedParseError::MissingLanguage { index });
    };

    if lang == "css" {
        return Ok(Some(VerbatimTagResult::Css(content.to_string())));
    }

    let embed_mode = lang
        .parse::<OutputMode>()
        .map_err(|_| EmbedParseError::InvalidLanguage {
            index,
            language: lang.to_string(),
        })?;

    match mode {
        None => Ok(None),
        Some(m) if m != embed_mode => Err(EmbedParseError::LanguageMismatch {
            index,
            language: lang.to_string(),
            mode: m,
        }),
        Some(_) => Ok(Some(VerbatimTagResult::Embed {
            mode: embed_mode.to_string(),
            code: content.to_string(),
        })),
    }
}

/// Wraps each line of highlighted HTML in `<span class="line">` so consumers
/// can attach per-line styling (line numbers, highlights, etc.).
fn wrap_lines(html: &str) -> String {
    let mut out = String::with_capacity(html.len() + 64);
    for (i, line) in html.lines().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(r#"<span class="line">"#);
        out.push_str(line);
        out.push_str("</span>");
    }
    out
}
