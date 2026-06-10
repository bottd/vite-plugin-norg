use super::error::EmbedParseError;
use crate::types::OutputMode;
use crate::utils::is_http_url;
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
                        r#"<pre class="arborium lang-{}"><code>{}</code></pre>"#,
                        encode_minimal(lang),
                        wrap_lines(&highlighted)
                    ),
                    // Trim the trailing newline like the highlighter does, so
                    // both paths emit the same number of line spans.
                    Err(_) => format!(
                        r#"<pre><code>{}</code></pre>"#,
                        wrap_lines(&encode_minimal(code.trim_end_matches('\n')))
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

/// Wraps each line of highlighted HTML in `<span class="line">` so consumers
/// can attach per-line styling (line numbers, highlights, etc.). The
/// highlighter emits multi-line tokens (block comments, multi-line strings)
/// as a single `<span>` with the newline inside; those spans are closed at
/// each line break and re-opened on the next line so every line span stays
/// self-contained and balanced.
fn wrap_lines(html: &str) -> String {
    let mut out = String::with_capacity(html.len() + 64);
    let mut open_spans: Vec<&str> = Vec::new();
    for (i, line) in html.lines().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(r#"<span class="line">"#);
        for span in &open_spans {
            out.push_str(span);
        }
        out.push_str(line);
        track_open_spans(line, &mut open_spans);
        for span in open_spans.iter().rev() {
            push_close_tag(span, &mut out);
        }
        out.push_str("</span>");
    }
    out
}

/// Emits `</name>` for an open tag like `<name …>`. The highlighter's tag
/// names vary by format (`<a-c>` custom elements by default, `<span>` with
/// the classic format), so the close tag must be derived from the open tag.
fn push_close_tag(open: &str, out: &mut String) {
    let name = open[1..]
        .split(|c: char| c == '>' || c.is_whitespace())
        .next()
        .unwrap_or("");
    out.push_str("</");
    out.push_str(name);
    out.push('>');
}

/// Updates `stack` with the highlight spans still open at the end of `line`.
/// The input contains only `<span …>`/`</span>` tags around HTML-escaped text
/// (both the highlighter output and the `encode_minimal` fallback), so plain
/// tag scanning is sufficient.
fn track_open_spans<'a>(line: &'a str, stack: &mut Vec<&'a str>) {
    let mut rest = line;
    while let Some(start) = rest.find('<') {
        rest = &rest[start..];
        let Some(end) = rest.find('>') else { break };
        let tag = &rest[..=end];
        if tag.starts_with("</") {
            stack.pop();
        } else {
            stack.push(tag);
        }
        rest = &rest[end + 1..];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_lines_keeps_single_line_spans_intact() {
        let html = "<span class=\"kw\">fn</span> main";
        assert_eq!(
            wrap_lines(html),
            "<span class=\"line\"><span class=\"kw\">fn</span> main</span>"
        );
    }

    #[test]
    fn wrap_lines_rebalances_multiline_spans() {
        // A token spanning a newline must be split into one balanced span per
        // line, otherwise its closing tag would close the line span instead.
        let html = "<span class=\"string\">\"a\nb\"</span>";
        assert_eq!(
            wrap_lines(html),
            "<span class=\"line\"><span class=\"string\">\"a</span></span>\n<span class=\"line\"><span class=\"string\">b\"</span></span>"
        );
    }

    #[test]
    fn wrap_lines_closes_custom_elements_with_matching_tags() {
        // arborium's default HtmlFormat::CustomElements emits tags like
        // <a-c>; a multi-line token must be closed and reopened with that
        // exact element name, not a hardcoded </span>.
        let html = "<a-c>/* one\ntwo */</a-c>";
        assert_eq!(
            wrap_lines(html),
            "<span class=\"line\"><a-c>/* one</a-c></span>\n<span class=\"line\"><a-c>two */</a-c></span>"
        );
    }
}
