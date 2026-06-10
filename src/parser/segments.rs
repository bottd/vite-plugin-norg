use crate::utils::{into_slug, is_http_url};
use htmlescape::encode_minimal;
use rust_norg::{LinkTarget, ParagraphSegment, ParagraphSegmentToken};
use std::fmt::Write;

pub fn convert_segments(segments: &[ParagraphSegment]) -> String {
    let mut out = String::with_capacity(segments.len() * 32);
    convert_segments_into(segments, &mut out);
    out
}

/// Tokens are the hottest path in the renderer (every word, space, and
/// punctuation char of every paragraph) — all conversion writes into one
/// shared buffer instead of allocating a `String` per segment.
fn convert_segments_into(segments: &[ParagraphSegment], out: &mut String) {
    for segment in segments {
        convert_segment(segment, out);
    }
}

pub fn convert_code_segments(segments: &[ParagraphSegment]) -> String {
    let mut out = String::new();
    for segment in segments {
        if let ParagraphSegment::Token(token) = segment {
            render_token(token, &mut out);
        }
    }
    out
}

fn convert_segment(segment: &ParagraphSegment, out: &mut String) {
    match segment {
        ParagraphSegment::Token(token) => render_token(token, out),

        ParagraphSegment::AttachedModifier {
            modifier_type,
            content,
        } => convert_attached_modifier(*modifier_type, content, out),

        ParagraphSegment::Link {
            targets,
            description,
            filepath,
            ..
        } => convert_link(targets, description.as_deref(), filepath.as_deref(), out),

        ParagraphSegment::Anchor { content, .. } => convert_segments_into(content, out),

        ParagraphSegment::InlineVerbatim(tokens) => {
            let text: String = tokens.iter().map(ToString::to_string).collect();
            out.push_str("<code>");
            out.push_str(&encode_minimal(&text));
            out.push_str("</code>");
        }

        _ => eprintln!("Warning: Unsupported paragraph segment type"),
    }
}

fn render_token(token: &ParagraphSegmentToken, out: &mut String) {
    match token {
        ParagraphSegmentToken::Whitespace => out.push(' '),
        ParagraphSegmentToken::Text(text) => out.push_str(&encode_minimal(text)),
        ParagraphSegmentToken::Special(ch) | ParagraphSegmentToken::Escape(ch) => {
            let mut buf = [0u8; 4];
            out.push_str(&encode_minimal(ch.encode_utf8(&mut buf)));
        }
    }
}

fn convert_attached_modifier(modifier_type: char, content: &[ParagraphSegment], out: &mut String) {
    if modifier_type == '`' {
        out.push_str("<code>");
        out.push_str(&convert_code_segments(content));
        out.push_str("</code>");
        return;
    }
    let (open, close) = match modifier_type {
        '*' => ("<strong>", "</strong>"),
        '_' => ("<em>", "</em>"),
        '^' => ("<sup>", "</sup>"),
        ',' => ("<sub>", "</sub>"),
        '-' => ("<s>", "</s>"),
        '!' => (r#"<span class="spoiler">"#, "</span>"),
        '$' => (r#"<span class="math">"#, "</span>"),
        '&' => ("<var>", "</var>"),
        '/' => ("<i>", "</i>"),
        '=' => ("<mark>", "</mark>"),
        _ => return convert_segments_into(content, out),
    };
    out.push_str(open);
    convert_segments_into(content, out);
    out.push_str(close);
}

/// `.norg` paths are rewritten to `.html` so links resolve in the build output.
fn norg_to_html(path: &str) -> String {
    path.strip_suffix(".norg")
        .map(|base| format!("{base}.html"))
        .unwrap_or_else(|| path.to_string())
}

/// Writes an anchor tag. `display_html` must already be final HTML — either
/// converted segments or an escaped raw fallback; escaping it here again
/// would double-encode descriptions and render their inline markup as text.
fn anchor(out: &mut String, href: &str, display_html: &str, external: bool) {
    let target = if external { r#" target="_blank""# } else { "" };
    let _ = write!(
        out,
        r#"<a href="{}"{target}>{display_html}</a>"#,
        encode_minimal(href)
    );
}

fn convert_link(
    targets: &[LinkTarget],
    description: Option<&[ParagraphSegment]>,
    filepath: Option<&str>,
    out: &mut String,
) {
    let display = description.map(convert_segments);

    match targets.first() {
        Some(LinkTarget::Url(url)) => {
            let display_html = display.unwrap_or_else(|| encode_minimal(url));
            match filepath {
                // `{:file.norg:url}` carries a file path; rewrite it to `.html`
                // like the Heading/Path/None branches do, or the link is dead.
                Some(fp) => anchor(out, &norg_to_html(fp), &display_html, false),
                None if is_http_url(url) => anchor(out, url, &display_html, true),
                None => anchor(out, &norg_to_html(url), &display_html, false),
            }
        }
        Some(LinkTarget::Heading { title, .. }) => {
            let title_html = convert_segments(title);
            let slug = into_slug(&title_html);
            // `{:path:# Heading}` links carry both a file path and a heading
            // target; keep the path instead of degrading to a same-page
            // anchor.
            let href = match filepath {
                Some(fp) => format!("{}#{slug}", norg_to_html(fp)),
                None => format!("#{slug}"),
            };
            let display_html = display.unwrap_or(title_html);
            anchor(out, &href, &display_html, false);
        }
        Some(LinkTarget::Path(path)) => {
            let display_html = display.unwrap_or_else(|| encode_minimal(path));
            anchor(out, &norg_to_html(path), &display_html, false);
        }
        Some(
            LinkTarget::Footnote(_)
            | LinkTarget::Definition(_)
            | LinkTarget::Timestamp(_)
            | LinkTarget::Generic(_)
            | LinkTarget::Extendable(_)
            | LinkTarget::Wiki(_),
        ) => {}
        None => {
            if let Some(fp) = filepath {
                let display_html = display.unwrap_or_else(|| encode_minimal(fp));
                anchor(out, &norg_to_html(fp), &display_html, false);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> ParagraphSegment {
        ParagraphSegment::Token(ParagraphSegmentToken::Text(s.to_string()))
    }

    #[test]
    fn escaped_metacharacters_are_html_escaped() {
        // `\<`, `\>`, `\&` escape the modifier meaning of the char but must
        // still be encoded so they render as literal text, not raw markup.
        let segments = [
            ParagraphSegment::Token(ParagraphSegmentToken::Escape('<')),
            ParagraphSegment::Token(ParagraphSegmentToken::Escape('&')),
            ParagraphSegment::Token(ParagraphSegmentToken::Escape('>')),
        ];
        assert_eq!(convert_segments(&segments), "&lt;&amp;&gt;");
    }

    #[test]
    fn link_description_is_encoded_exactly_once() {
        // The description is converted-segment HTML; encoding it again in
        // anchor() would display 'AT&amp;T' and turn markup into literal tags.
        let description = [text("AT&T")];
        let mut out = String::new();
        convert_link(
            &[LinkTarget::Url("https://example.com".into())],
            Some(&description),
            None,
            &mut out,
        );
        assert_eq!(
            out,
            r#"<a href="https://example.com" target="_blank">AT&amp;T</a>"#
        );
    }

    #[test]
    fn link_description_keeps_inline_markup() {
        let description = [ParagraphSegment::AttachedModifier {
            modifier_type: '*',
            content: vec![text("bold")],
        }];
        let mut out = String::new();
        convert_link(
            &[LinkTarget::Url("https://example.com".into())],
            Some(&description),
            None,
            &mut out,
        );
        assert_eq!(
            out,
            r#"<a href="https://example.com" target="_blank"><strong>bold</strong></a>"#
        );
    }

    #[test]
    fn url_link_with_norg_filepath_is_rewritten_to_html() {
        // `{:notes.norg:label}` carries a file path on a Url target; it must be
        // rewritten to `.html` like the Heading/Path branches, not left dead.
        let description = [text("label")];
        let mut out = String::new();
        convert_link(
            &[LinkTarget::Url("label".into())],
            Some(&description),
            Some("notes.norg"),
            &mut out,
        );
        assert_eq!(out, r#"<a href="notes.html">label</a>"#);
    }

    #[test]
    fn heading_link_with_filepath_keeps_the_path() {
        // `{:docs/readme.norg:# Install}` must link into the target document,
        // not to a same-page anchor.
        let title = vec![text("Install")];
        let mut out = String::new();
        convert_link(
            &[LinkTarget::Heading { level: 1, title }],
            None,
            Some("docs/readme.norg"),
            &mut out,
        );
        assert_eq!(out, r##"<a href="docs/readme.html#install">Install</a>"##);
    }
}
