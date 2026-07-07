use super::error::EmbedParseError;
use crate::types::OutputMode;
use crate::utils::is_http_url;
use arborium::advanced::{spans_to_html, Span};
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
                let src = if path.starts_with('/') || is_http_url(path) {
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

/// Renders highlighted `code` as one `<span class="line">` row per source line,
/// so consumers can attach per-line styling (line numbers, highlights, etc.).
///
/// Rather than serialize the whole block and re-parse the resulting HTML, each
/// highlight span is clipped to the line it falls on and that line is rendered
/// with arborium's own `spans_to_html`. A token that crosses a newline (block
/// comment, multi-line string) is therefore emitted as one self-contained span
/// per line by construction, and arborium — not this code — owns the tag
/// format, so it can never drift from what the highlighter produces.
///
/// `code` must already have its trailing newline trimmed (see the caller),
/// matching arborium's own trailing-newline trimming; a single-line block thus
/// renders byte-for-byte identically to `spans_to_html` over the whole block.
fn highlight_lines(code: &str, spans: Vec<Span>) -> String {
    if code.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(code.len() + code.len() / 4);
    let mut line_start: u32 = 0;
    for (i, line) in code.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let line_end = line_start + line.len() as u32;
        out.push_str(r#"<span class="line">"#);
        // We render the spans ourselves (never `Highlighter::highlight`), so
        // this is the sole place the output format is chosen; CustomElements
        // matches what the plugin's stylesheet targets.
        out.push_str(&spans_to_html(
            line,
            clip_spans(&spans, line_start, line_end),
            &HtmlFormat::CustomElements,
        ));
        out.push_str("</span>");
        line_start = line_end + 1; // +1 skips the '\n' separator
    }
    out
}

/// The spans overlapping `[line_start, line_end)`, clipped to that range and
/// rebased to line-local byte offsets so they render against the line's own
/// substring.
fn clip_spans(spans: &[Span], line_start: u32, line_end: u32) -> Vec<Span> {
    spans
        .iter()
        .filter(|s| s.start < line_end && s.end > line_start)
        .map(|s| Span {
            start: s.start.max(line_start) - line_start,
            end: s.end.min(line_end) - line_start,
            capture: s.capture.clone(),
            pattern_index: s.pattern_index,
        })
        .collect()
}

/// Wraps each line of already-escaped, tag-free `text` in `<span class="line">`.
/// Used on the fallback path, when the language is unsupported and there are no
/// highlight spans to place.
fn wrap_plain_lines(text: &str) -> String {
    let mut out = String::new();
    for (i, line) in text.lines().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(r#"<span class="line">"#);
        out.push_str(line);
        out.push_str("</span>");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlight_lines_wraps_each_line() {
        // With no spans every line is just escaped text wrapped in a line span.
        assert_eq!(
            highlight_lines("a\nb", vec![]),
            "<span class=\"line\">a</span>\n<span class=\"line\">b</span>"
        );
    }

    #[test]
    fn highlight_lines_single_line() {
        assert_eq!(
            highlight_lines("x", vec![]),
            "<span class=\"line\">x</span>"
        );
    }

    #[test]
    fn highlight_lines_empty_is_empty() {
        assert_eq!(highlight_lines("", vec![]), "");
    }

    #[test]
    fn clip_spans_clips_and_rebases_to_line_local_offsets() {
        // A span over bytes 1..5 of "ab\ncd" (line 0 = [0,2), line 1 = [3,5))
        // crosses the newline and must clip to one span per line, rebased.
        let span = Span {
            start: 1,
            end: 5,
            capture: "kw".to_string(),
            pattern_index: 0,
        };
        let line0 = clip_spans(std::slice::from_ref(&span), 0, 2);
        assert_eq!((line0[0].start, line0[0].end), (1, 2));
        let line1 = clip_spans(std::slice::from_ref(&span), 3, 5);
        assert_eq!((line1[0].start, line1[0].end), (0, 2));
        // A span outside the line entirely is dropped.
        assert!(clip_spans(std::slice::from_ref(&span), 10, 12).is_empty());
    }

    #[test]
    fn wrap_plain_lines_wraps_each_line() {
        assert_eq!(
            wrap_plain_lines("a\nb"),
            "<span class=\"line\">a</span>\n<span class=\"line\">b</span>"
        );
    }
}
