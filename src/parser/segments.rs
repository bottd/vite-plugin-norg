use crate::utils::{has_unsafe_scheme, into_slug, is_external_url};
use htmlescape::encode_minimal;
use rust_norg::{LinkTarget, ParagraphSegment, ParagraphSegmentToken};
use std::cell::Cell;
use std::collections::HashMap;
use std::fmt::Write;

thread_local! {
    /// True while a link's display text is being rendered, so a link nested in
    /// another link's description emits as plain text instead of `<a>` inside
    /// `<a>` (invalid HTML that browsers silently auto-close).
    static IN_ANCHOR: Cell<bool> = const { Cell::new(false) };
}

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
            push_escaped(out, &text);
            out.push_str("</code>");
        }

        _ => crate::diagnostics::warn("unsupported paragraph segment type"),
    }
}

fn render_token(token: &ParagraphSegmentToken, out: &mut String) {
    match token {
        ParagraphSegmentToken::Whitespace => out.push(' '),
        ParagraphSegmentToken::Text(text) => push_escaped(out, text),
        ParagraphSegmentToken::Special(ch) | ParagraphSegmentToken::Escape(ch) => {
            let mut buf = [0u8; 4];
            push_escaped(out, ch.encode_utf8(&mut buf));
        }
    }
}

/// Writes `s` into `out` HTML-escaped, matching `htmlescape::encode_minimal`'s
/// five entities byte-for-byte but without the `String` it allocates per call.
/// Tokens are the hottest path in the renderer (every word/space/punctuation of
/// every paragraph), so escaping streams straight into the shared buffer.
fn push_escaped(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '"' => out.push_str("&quot;"),
            '&' => out.push_str("&amp;"),
            '\'' => out.push_str("&#x27;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
}

/// Renders a heading's final HTML, its slug id, and its level clamped to the
/// `<h1>`–`<h6>` range, so the renderer and the TOC can't derive any of the
/// three differently for the same heading (`rust_norg` parses 7+ `*` as level
/// 7+, but HTML has no `<h7>`). `seen` de-duplicates ids across the document so
/// two headings with the same title don't emit the same (invalid) id.
pub fn heading_html_and_id(
    title: &[ParagraphSegment],
    level: u16,
    seen: &mut HashMap<String, u32>,
) -> (String, String, u16) {
    let html = convert_segments(title);
    let id = dedup_id(title_slug(title), seen);
    (html, id, level.min(6))
}

/// Disambiguates a slug against the ones already emitted in the same document:
/// the first `setup` stays `setup`, the next becomes `setup-1`, and so on. An
/// empty slug (symbol-only title) is left empty — the emit site omits the `id`
/// attribute. The renderer and TOC each walk headings in document order, so they
/// derive identical suffixes from separate counters.
fn dedup_id(base: String, seen: &mut HashMap<String, u32>) -> String {
    if base.is_empty() {
        return base;
    }
    let count = seen.entry(base.clone()).or_insert(0);
    let id = if *count == 0 {
        base.clone()
    } else {
        format!("{base}-{count}")
    };
    *count += 1;
    id
}

/// The slug id for a heading or footnote title, derived from its plain visible
/// *text* rather than its rendered HTML — so a title like `{url}[Install]` slugs
/// to `install`, not the `<a href…>` markup `convert_segments` would emit. Every
/// id site (heading tag/TOC, same-document `{# Heading}` links, footnote anchor)
/// routes through here, so an id and the anchor pointing at it always match.
pub fn title_slug(title: &[ParagraphSegment]) -> String {
    let mut text = String::new();
    push_title_text(title, &mut text);
    into_slug(&text)
}

/// Appends the plain visible text of `segments` to `out`: the words a reader
/// sees, with inline markup (emphasis, links, code, anchors) unwrapped and no
/// HTML tags or entities.
fn push_title_text(segments: &[ParagraphSegment], out: &mut String) {
    for segment in segments {
        match segment {
            ParagraphSegment::Token(ParagraphSegmentToken::Whitespace) => out.push(' '),
            ParagraphSegment::Token(ParagraphSegmentToken::Text(text)) => out.push_str(text),
            ParagraphSegment::Token(
                ParagraphSegmentToken::Special(c) | ParagraphSegmentToken::Escape(c),
            ) => out.push(*c),
            ParagraphSegment::AttachedModifier { content, .. }
            | ParagraphSegment::Anchor { content, .. } => push_title_text(content, out),
            // A link shows its description if it has one, otherwise the target
            // itself (URL/path text, or a nested heading title).
            ParagraphSegment::Link {
                targets,
                description,
                ..
            } => match description {
                Some(desc) => push_title_text(desc, out),
                None => match targets.first() {
                    Some(LinkTarget::Url(u)) => out.push_str(u),
                    Some(LinkTarget::Path(p)) => out.push_str(p),
                    Some(LinkTarget::Heading { title, .. }) => push_title_text(title, out),
                    _ => {}
                },
            },
            ParagraphSegment::InlineVerbatim(tokens) => {
                tokens.iter().for_each(|t| out.push_str(&t.to_string()));
            }
            _ => {}
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
///
/// Two safety measures apply here, the single chokepoint for every link:
/// a target with an unsafe URL scheme (`javascript:`, scriptable `data:`, …) is
/// dropped to its plain display text rather than emitted as a clickable script
/// URL, and external links get `rel="noopener noreferrer"` alongside
/// `target="_blank"` to prevent the opened page from hijacking `window.opener`.
/// When `nested`, this link sits inside another link's display, so only the
/// display text is emitted (an `<a>` inside an `<a>` is invalid HTML).
fn anchor(out: &mut String, href: &str, display_html: &str, external: bool, nested: bool) {
    if has_unsafe_scheme(href) {
        crate::diagnostics::warn(format!("dropping link with unsafe URL scheme: {href}"));
        out.push_str(display_html);
        return;
    }
    if nested {
        out.push_str(display_html);
        return;
    }
    let target = if external {
        r#" target="_blank" rel="noopener noreferrer""#
    } else {
        ""
    };
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
    // Render this link's display with nested links flattened to plain text;
    // `nested` records whether we are ourselves inside a parent link's display.
    let nested = IN_ANCHOR.replace(true);
    let display = description.map(convert_segments);

    let link = match targets.first() {
        Some(LinkTarget::Url(url)) => {
            let display_html = display.unwrap_or_else(|| encode_minimal(url));
            let (href, external) = match filepath {
                // `{:file.norg:url}` carries a file path; rewrite it to `.html`
                // like the Heading/Path/None branches do, or the link is dead.
                Some(fp) => (norg_to_html(fp), false),
                // External (`http(s)://` or protocol-relative `//host`): emit
                // as-is with external-link hardening, never as an in-site path.
                None if is_external_url(url) => (url.clone(), true),
                None => (norg_to_html(url), false),
            };
            Some((href, display_html, external))
        }
        Some(LinkTarget::Heading { title, .. }) => {
            // Same derivation as the heading tag/TOC so the anchor resolves.
            let slug = title_slug(title);
            // `{:path:# Heading}` links carry both a file path and a heading
            // target; keep the path instead of degrading to a same-page anchor.
            let href = match filepath {
                Some(fp) => format!("{}#{slug}", norg_to_html(fp)),
                None => format!("#{slug}"),
            };
            // Only render the title HTML when there's no description to use.
            let display_html = display.unwrap_or_else(|| convert_segments(title));
            Some((href, display_html, false))
        }
        Some(LinkTarget::Path(path)) => {
            let display_html = display.unwrap_or_else(|| encode_minimal(path));
            Some((norg_to_html(path), display_html, false))
        }
        Some(
            LinkTarget::Footnote(_)
            | LinkTarget::Definition(_)
            | LinkTarget::Timestamp(_)
            | LinkTarget::Generic(_)
            | LinkTarget::Extendable(_)
            | LinkTarget::Wiki(_),
        ) => None,
        None => filepath.map(|fp| {
            let display_html = display.unwrap_or_else(|| encode_minimal(fp));
            (norg_to_html(fp), display_html, false)
        }),
    };

    // Restore the flag before emitting, so this link's own `<a>` is written
    // unless it is itself nested inside a parent link.
    IN_ANCHOR.set(nested);
    if let Some((href, display_html, external)) = link {
        anchor(out, &href, &display_html, external, nested);
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
            r#"<a href="https://example.com" target="_blank" rel="noopener noreferrer">AT&amp;T</a>"#
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
            r#"<a href="https://example.com" target="_blank" rel="noopener noreferrer"><strong>bold</strong></a>"#
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

    #[test]
    fn javascript_scheme_link_is_dropped_to_plain_text() {
        // A crafted `javascript:` target must not become a clickable script
        // URL; the link degrades to its display text.
        let description = [text("click me")];
        let mut out = String::new();
        convert_link(
            &[LinkTarget::Url("javascript:alert(document.cookie)".into())],
            Some(&description),
            None,
            &mut out,
        );
        assert_eq!(out, "click me");
    }

    #[test]
    fn protocol_relative_url_is_treated_as_external() {
        // `//host` is a cross-origin URL, not an in-site path: it must be
        // emitted as-is (never rewritten to `.html`) and get external-link
        // hardening, not rendered as a bare same-site link.
        let description = [text("cdn")];
        let mut out = String::new();
        convert_link(
            &[LinkTarget::Url("//cdn.example.com/x".into())],
            Some(&description),
            None,
            &mut out,
        );
        assert_eq!(
            out,
            r#"<a href="//cdn.example.com/x" target="_blank" rel="noopener noreferrer">cdn</a>"#
        );
    }

    #[test]
    fn title_slug_derives_from_visible_text_not_markup() {
        // A heading with a link and emphasis must slug from the words a reader
        // sees, never the `<a href…>`/`<i>` markup `convert_segments` emits.
        let title = [
            ParagraphSegment::Link {
                filepath: None,
                targets: vec![LinkTarget::Url(
                    "https://neovim.io/doc#nvim_create_buf()".into(),
                )],
                description: Some(vec![text("nvim_create_buf")]),
            },
            text(" in "),
            ParagraphSegment::AttachedModifier {
                modifier_type: '/',
                content: vec![text("Lua")],
            },
        ];
        assert_eq!(title_slug(&title), "nvim-create-buf-in-lua");
    }

    #[test]
    fn nested_link_in_description_does_not_nest_anchors() {
        // A link whose description contains another link must not emit `<a>`
        // inside `<a>` (invalid HTML); the inner link degrades to its text.
        let inner = ParagraphSegment::Link {
            filepath: None,
            targets: vec![LinkTarget::Url("https://b.com".into())],
            description: Some(vec![text("inner")]),
        };
        let description = [text("see "), inner];
        let mut out = String::new();
        convert_link(
            &[LinkTarget::Url("https://a.com".into())],
            Some(&description),
            None,
            &mut out,
        );
        assert_eq!(
            out,
            r#"<a href="https://a.com" target="_blank" rel="noopener noreferrer">see inner</a>"#
        );
        assert_eq!(
            out.matches("<a ").count(),
            1,
            "nested anchor emitted: {out}"
        );
    }
}
