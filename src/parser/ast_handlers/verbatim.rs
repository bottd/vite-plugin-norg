use super::error::EmbedParseError;
use crate::types::OutputMode;
use crate::utils::UrlKind;
use arborium::advanced::{Span, spans_to_html};
use arborium::{Highlighter, HtmlFormat};
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
                let dedented = dedent(content);
                // Trim the trailing newline once, up front, so the highlight
                // and fallback paths see identical input and emit the same
                // number of `<span class="line">` rows. (Highlighting the
                // untrimmed string but the fallback the trimmed one could yield
                // a different line count between languages arborium does and
                // doesn't support.)
                let code = dedented.trim_end_matches('\n');
                let lang = first_param().unwrap_or("text");
                let body = match highlighter.highlight_spans(lang, code) {
                    Ok(spans) => format!(
                        r#"<pre class="arborium lang-{}"><code>{}</code></pre>"#,
                        encode_minimal(lang),
                        highlight_lines(code, spans)
                    ),
                    Err(_) => format!(
                        r#"<pre><code>{}</code></pre>"#,
                        wrap_plain_lines(&encode_minimal(code))
                    ),
                };
                Ok(Some(VerbatimTagResult::Html(body)))
            }

            Self::Image => Ok(first_param().map(|path| {
                // Only a bare relative path needs `./`; rooted, `//host` and
                // scheme'd sources already resolve.
                let src = if UrlKind::of(path).is_site_relative() && !path.starts_with('/') {
                    format!("./{path}")
                } else {
                    path.to_string()
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

fn highlight_lines(code: &str, mut spans: Vec<Span>) -> String {
    spans.sort_by_key(|s| s.start);
    let mut cursor = 0usize;
    wrap_lines(code, |line, line_start, line_end, out| {
        while cursor < spans.len() && spans[cursor].end <= line_start {
            cursor += 1;
        }

        let clipped = spans[cursor..]
            .iter()
            .take_while(|span| span.start < line_end)
            .filter(|span| span.end > line_start)
            .map(|span| Span {
                start: span.start.max(line_start) - line_start,
                end: span.end.min(line_end) - line_start,
                capture: span.capture.clone(),
                pattern_index: span.pattern_index,
            })
            .collect();
        out.push_str(&spans_to_html(line, clipped, &HtmlFormat::CustomElements));
    })
}

fn wrap_plain_lines(text: &str) -> String {
    wrap_lines(text, |line, _, _, out| out.push_str(line))
}

fn wrap_lines(source: &str, mut body: impl FnMut(&str, u32, u32, &mut String)) -> String {
    if source.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(source.len() + source.len() / 4);
    let mut line_start = 0u32;
    for (i, line) in source.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(r#"<span class="line">"#);
        let line_end = line_start + line.len() as u32;
        body(line, line_start, line_end, &mut out);
        out.push_str("</span>");
        line_start = line_end + 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlight_lines_clips_multiline_spans() {
        let span = Span {
            start: 0,
            end: 5,
            capture: "comment".into(),
            pattern_index: 0,
        };
        let out = highlight_lines("ab\ncd", vec![span]);
        assert_eq!(out.matches(r#"<span class="line">"#).count(), 2, "{out}");
        assert!(out.contains("ab") && out.contains("cd"), "{out}");
    }

    #[test]
    fn wrap_plain_lines_wraps_each_line() {
        assert_eq!(
            wrap_plain_lines("a\nb"),
            "<span class=\"line\">a</span>\n<span class=\"line\">b</span>"
        );
    }
}
