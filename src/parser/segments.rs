use crate::utils::{UrlKind, has_unsafe_scheme, into_slug};
use htmlescape::encode_minimal;
use rust_norg::{LinkTarget, ParagraphSegment, ParagraphSegmentToken};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

#[cfg(test)]
pub fn convert_segments(segments: &[ParagraphSegment]) -> String {
    render_segments(segments, false, None)
}

pub fn convert_segments_with_ids(segments: &[ParagraphSegment], ids: &DocumentIds) -> String {
    render_segments(segments, false, Some(ids))
}

fn render_segments(
    segments: &[ParagraphSegment],
    in_anchor: bool,
    ids: Option<&DocumentIds>,
) -> String {
    let mut out = String::with_capacity(segments.len() * 32);
    convert_segments_into(segments, &mut out, in_anchor, ids);
    out
}

/// Tokens are the hottest path in the renderer (every word, space, and
/// punctuation char of every paragraph) — all conversion writes into one
/// shared buffer instead of allocating a `String` per segment.
fn convert_segments_into(
    segments: &[ParagraphSegment],
    out: &mut String,
    in_anchor: bool,
    ids: Option<&DocumentIds>,
) {
    for segment in segments {
        convert_segment(segment, out, in_anchor, ids);
    }
}

fn convert_code_segments(segments: &[ParagraphSegment], out: &mut String) {
    for segment in segments {
        if let ParagraphSegment::Token(token) = segment {
            render_token(token, out);
        }
    }
}

fn convert_segment(
    segment: &ParagraphSegment,
    out: &mut String,
    in_anchor: bool,
    ids: Option<&DocumentIds>,
) {
    match segment {
        ParagraphSegment::Token(token) => render_token(token, out),

        ParagraphSegment::AttachedModifier {
            modifier_type,
            content,
        } => convert_attached_modifier(*modifier_type, content, out, in_anchor, ids),

        ParagraphSegment::Link {
            targets,
            description,
            filepath,
            ..
        } => convert_link(
            targets,
            description.as_deref(),
            filepath.as_deref(),
            out,
            in_anchor,
            ids,
        ),

        ParagraphSegment::Anchor {
            content,
            description,
        } => convert_segments_into(
            description.as_deref().unwrap_or(content),
            out,
            in_anchor,
            ids,
        ),

        ParagraphSegment::AnchorDefinition { content, target } => match target.as_ref() {
            ParagraphSegment::Link {
                targets, filepath, ..
            } if !targets.is_empty() || filepath.is_some() => {
                convert_link(
                    targets,
                    Some(content),
                    filepath.as_deref(),
                    out,
                    in_anchor,
                    ids,
                );
            }
            ParagraphSegment::Link { .. } => {
                crate::diagnostics::warn("anchor definition has no target");
                convert_segments_into(content, out, in_anchor, ids);
            }
            _ => {
                crate::diagnostics::warn("unsupported anchor definition target");
                convert_segments_into(content, out, in_anchor, ids);
            }
        },

        ParagraphSegment::InlineLinkTarget(content) => {
            convert_segments_into(content, out, in_anchor, ids)
        }

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
    ids: &mut DocumentIds,
) -> (String, String, u16) {
    let id = ids.next_heading();
    let html = convert_segments_with_ids(title, ids);
    (html, id, level.min(6))
}

#[derive(Default)]
pub struct DocumentIds {
    headings: Vec<String>,
    footnotes: Vec<String>,
    heading_links: HashMap<(u16, String), String>,
    footnote_links: HashMap<String, String>,
    next_heading: usize,
    next_footnote: usize,
}

impl DocumentIds {
    pub fn new(headings: Vec<(u16, String, String)>, footnotes: Vec<(String, String)>) -> Self {
        let mut allocator = IdAllocator::default();
        for (_, _, slug) in &headings {
            allocator.reserve(slug);
        }
        for (_, slug) in &footnotes {
            allocator.reserve(&format!("footnote-{slug}"));
        }

        let mut ids = Self::default();
        for (level, key, slug) in headings {
            let id = allocator.allocate(slug.clone());
            ids.heading_links
                .entry((level, key))
                .or_insert_with(|| id.clone());
            ids.headings.push(id);
        }
        for (key, slug) in footnotes {
            let id = allocator.allocate(format!("footnote-{slug}"));
            ids.footnote_links.entry(key).or_insert_with(|| id.clone());
            ids.footnotes.push(id);
        }
        ids
    }

    fn next_heading(&mut self) -> String {
        take_id(&self.headings, &mut self.next_heading, "heading")
    }

    pub fn next_footnote(&mut self) -> String {
        take_id(&self.footnotes, &mut self.next_footnote, "footnote")
    }

    fn heading_link(&self, level: u16, slug: &str) -> Option<&str> {
        self.heading_links
            .get(&(level, slug.to_string()))
            .map(String::as_str)
    }

    fn footnote_link(&self, slug: &str) -> Option<&str> {
        self.footnote_links.get(slug).map(String::as_str)
    }

    /// Reserved but never handed out, as `(headings, footnotes)`. Both must
    /// reach zero: ids are consumed positionally, so a divergence means every
    /// node past it already took its neighbour's id. Callers assert on this.
    pub fn unconsumed(&self) -> (usize, usize) {
        (
            self.headings.len().saturating_sub(self.next_heading),
            self.footnotes.len().saturating_sub(self.next_footnote),
        )
    }
}

/// Release-mode net only — [`DocumentIds::unconsumed`] is what actually catches
/// a desync. An anchorless heading beats panicking out of the parse thread.
fn take_id(ids: &[String], next: &mut usize, kind: &str) -> String {
    let id = ids.get(*next).cloned().unwrap_or_else(|| {
        crate::diagnostics::warn(format!(
            "internal: {kind} ids exhausted — this {kind} gets no anchor, \
             and links to it will not resolve"
        ));
        String::new()
    });
    *next += 1;
    id
}

#[derive(Default)]
struct IdAllocator {
    seen: HashSet<String>,
    reserved: HashSet<String>,
}

impl IdAllocator {
    fn reserve(&mut self, id: &str) {
        if !id.is_empty() {
            self.reserved.insert(id.to_string());
        }
    }

    fn allocate(&mut self, base: String) -> String {
        if base.is_empty() || self.seen.insert(base.clone()) {
            return base;
        }

        for suffix in 1.. {
            let id = format!("{base}-{suffix}");
            if !self.reserved.contains(&id) && self.seen.insert(id.clone()) {
                return id;
            }
        }
        unreachable!()
    }
}

/// The slug id for a heading or footnote title, derived from its plain visible
/// *text* rather than its rendered HTML — so a title like `{url}[Install]` slugs
/// to `install`, not the `<a href…>` markup `convert_segments` would emit. Every
/// base-slug site routes through here so links and generated IDs use the same
/// visible-text rules.
pub fn title_slug(title: &[ParagraphSegment]) -> String {
    let mut text = String::new();
    push_title_text(title, &mut text);
    into_slug(&text)
}

pub fn title_key(title: &[ParagraphSegment]) -> String {
    let mut key = String::new();
    push_title_key(title, &mut key);
    key
}

fn push_title_key(segments: &[ParagraphSegment], out: &mut String) {
    for segment in segments {
        match segment {
            ParagraphSegment::Token(ParagraphSegmentToken::Whitespace) => {}
            ParagraphSegment::Token(ParagraphSegmentToken::Text(text)) => {
                push_key_text(text, out);
            }
            ParagraphSegment::Token(
                ParagraphSegmentToken::Special(c) | ParagraphSegmentToken::Escape(c),
            ) => push_key_char(*c, out),
            ParagraphSegment::AttachedModifier {
                modifier_type,
                content,
            } => {
                out.push('m');
                out.push(*modifier_type);
                out.push('(');
                push_title_key(content, out);
                out.push(')');
            }
            ParagraphSegment::Link {
                filepath,
                targets,
                description,
            } => {
                out.push_str("link(");
                if let Some(filepath) = filepath {
                    push_key_text(filepath, out);
                }
                for target in targets {
                    push_link_target_key(target, out);
                }
                out.push(')');
                if let Some(description) = description {
                    out.push('[');
                    push_title_key(description, out);
                    out.push(']');
                }
            }
            ParagraphSegment::AnchorDefinition { content, target } => {
                out.push_str("anchor-def[");
                push_title_key(content, out);
                out.push(']');
                push_title_key(std::slice::from_ref(target.as_ref()), out);
            }
            ParagraphSegment::Anchor {
                content,
                description,
            } => {
                out.push_str("anchor[");
                push_title_key(content, out);
                out.push(']');
                if let Some(description) = description {
                    out.push('[');
                    push_title_key(description, out);
                    out.push(']');
                }
            }
            ParagraphSegment::InlineLinkTarget(content) => {
                out.push('<');
                push_title_key(content, out);
                out.push('>');
            }
            ParagraphSegment::InlineVerbatim(tokens) => {
                out.push('`');
                for token in tokens {
                    push_key_text(&token.to_string(), out);
                }
                out.push('`');
            }
            _ => out.push('?'),
        }
    }
}

fn push_link_target_key(target: &LinkTarget, out: &mut String) {
    match target {
        LinkTarget::Heading { level, title } => {
            let _ = write!(out, "h{level}(");
            push_title_key(title, out);
            out.push(')');
        }
        LinkTarget::Footnote(title) => {
            out.push_str("footnote(");
            push_title_key(title, out);
            out.push(')');
        }
        LinkTarget::Definition(title) => {
            out.push_str("definition(");
            push_title_key(title, out);
            out.push(')');
        }
        LinkTarget::Generic(title) => {
            out.push_str("generic(");
            push_title_key(title, out);
            out.push(')');
        }
        LinkTarget::Wiki(title) => {
            out.push_str("wiki(");
            push_title_key(title, out);
            out.push(')');
        }
        LinkTarget::Extendable(title) => {
            out.push_str("extendable(");
            push_title_key(title, out);
            out.push(')');
        }
        LinkTarget::Path(path) => {
            out.push_str("path(");
            push_key_text(path, out);
            out.push(')');
        }
        LinkTarget::Url(url) => {
            out.push_str("url(");
            push_key_text(url, out);
            out.push(')');
        }
        LinkTarget::Timestamp(timestamp) => {
            out.push_str("timestamp(");
            push_key_text(timestamp, out);
            out.push(')');
        }
    }
}

fn push_key_text(text: &str, out: &mut String) {
    for c in text.chars() {
        push_key_char(c, out);
    }
}

fn push_key_char(c: char, out: &mut String) {
    if c.is_whitespace() || c == '\\' {
        return;
    }
    out.extend(c.to_lowercase());
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
            | ParagraphSegment::AnchorDefinition { content, .. }
            | ParagraphSegment::InlineLinkTarget(content) => push_title_text(content, out),
            ParagraphSegment::Anchor {
                content,
                description,
            } => push_title_text(description.as_deref().unwrap_or(content), out),
            // A link shows its description if it has one, otherwise the target
            // itself (URL/path text, or a nested heading title).
            ParagraphSegment::Link {
                targets,
                description,
                filepath,
            } => match description {
                Some(desc) => push_title_text(desc, out),
                None => match targets.first() {
                    Some(LinkTarget::Url(u)) => out.push_str(u),
                    Some(LinkTarget::Path(p)) => out.push_str(p),
                    Some(LinkTarget::Heading { title, .. }) => push_title_text(title, out),
                    Some(
                        LinkTarget::Footnote(title)
                        | LinkTarget::Definition(title)
                        | LinkTarget::Generic(title)
                        | LinkTarget::Wiki(title)
                        | LinkTarget::Extendable(title),
                    ) => push_title_text(title, out),
                    Some(LinkTarget::Timestamp(timestamp)) => out.push_str(timestamp),
                    _ => {
                        if let Some(filepath) = filepath {
                            out.push_str(filepath);
                        }
                    }
                },
            },
            ParagraphSegment::InlineVerbatim(tokens) => {
                tokens.iter().for_each(|t| out.push_str(&t.to_string()));
            }
            _ => {}
        }
    }
}

fn convert_attached_modifier(
    modifier_type: char,
    content: &[ParagraphSegment],
    out: &mut String,
    in_anchor: bool,
    ids: Option<&DocumentIds>,
) {
    if modifier_type == '`' {
        out.push_str("<code>");
        convert_code_segments(content, out);
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
        _ => return convert_segments_into(content, out, in_anchor, ids),
    };
    out.push_str(open);
    convert_segments_into(content, out, in_anchor, ids);
    out.push_str(close);
}

/// Rewrites a `.norg` path to `.html` so links resolve in the build output.
/// Guards here rather than at the call sites because every link shape funnels
/// through it — see [`UrlKind::is_site_relative`] for what it refuses.
fn norg_to_html(path: &str) -> String {
    if !UrlKind::of(path).is_site_relative() {
        return path.to_string();
    }
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
    nested: bool,
    ids: Option<&DocumentIds>,
) {
    let display = description.map(|segments| render_segments(segments, true, ids));

    let link = match targets.first() {
        Some(LinkTarget::Url(url)) => {
            let display_html = display.unwrap_or_else(|| encode_minimal(url));
            let (href, external) = match filepath {
                // `{:file.norg:url}` carries a file path; rewrite it to `.html`
                // like the Heading/Path/None branches do, or the link is dead.
                Some(fp) => (norg_to_html(fp), false),
                // `norg_to_html` rewrites only in-site paths, so an `https:`,
                // `mailto:` or `//host` target passes through untouched and
                // keeps whatever hardening its scheme calls for.
                None => (norg_to_html(url), UrlKind::of(url).is_external()),
            };
            Some((href, display_html, external))
        }
        Some(LinkTarget::Heading { level, title }) => {
            // Same derivation as the heading tag/TOC so the anchor resolves.
            let slug = title_slug(title);
            let key = title_key(title);
            // `{:path:# Heading}` links carry both a file path and a heading
            // target; keep the path instead of degrading to a same-page anchor.
            let href = match filepath {
                Some(fp) => format!("{}#{slug}", norg_to_html(fp)),
                None => format!(
                    "#{}",
                    ids.and_then(|ids| ids.heading_link(*level, &key))
                        .unwrap_or(&slug)
                ),
            };
            // Only render the title HTML when there's no description to use.
            let display_html = display.unwrap_or_else(|| render_segments(title, true, ids));
            Some((href, display_html, false))
        }
        Some(LinkTarget::Path(path)) => {
            let display_html = display.unwrap_or_else(|| encode_minimal(path));
            Some((norg_to_html(path), display_html, false))
        }
        Some(LinkTarget::Footnote(title)) => {
            let slug = title_slug(title);
            let key = title_key(title);
            let href = match filepath {
                Some(fp) => format!("{}#footnote-{slug}", norg_to_html(fp)),
                None => ids
                    .and_then(|ids| ids.footnote_link(&key))
                    .map(|id| format!("#{id}"))
                    .unwrap_or_else(|| format!("#footnote-{slug}")),
            };
            let display_html = display.unwrap_or_else(|| render_segments(title, true, ids));
            Some((href, display_html, false))
        }
        Some(
            LinkTarget::Definition(title)
            | LinkTarget::Generic(title)
            | LinkTarget::Extendable(title)
            | LinkTarget::Wiki(title),
        ) => {
            out.push_str(&display.unwrap_or_else(|| render_segments(title, true, ids)));
            return;
        }
        Some(LinkTarget::Timestamp(timestamp)) => {
            out.push_str(&display.unwrap_or_else(|| encode_minimal(timestamp)));
            return;
        }
        None => filepath.map(|fp| {
            let display_html = display.unwrap_or_else(|| encode_minimal(fp));
            (norg_to_html(fp), display_html, false)
        }),
    };

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

    /// Renders a single link the way `convert_segment` would, at the top level
    /// of a paragraph.
    fn link_html(
        target: LinkTarget,
        description: Option<&[ParagraphSegment]>,
        filepath: Option<&str>,
    ) -> String {
        let mut out = String::new();
        convert_link(&[target], description, filepath, &mut out, false, None);
        out
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
        let out = link_html(
            LinkTarget::Url("https://example.com".into()),
            Some(&[text("AT&T")]),
            None,
        );
        assert_eq!(
            out,
            r#"<a href="https://example.com" target="_blank" rel="noopener noreferrer">AT&amp;T</a>"#
        );
    }

    #[test]
    fn link_description_keeps_inline_markup() {
        let out = link_html(
            LinkTarget::Url("https://example.com".into()),
            Some(&[ParagraphSegment::AttachedModifier {
                modifier_type: '*',
                content: vec![text("bold")],
            }]),
            None,
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
        let out = link_html(
            LinkTarget::Url("label".into()),
            Some(&[text("label")]),
            Some("notes.norg"),
        );
        assert_eq!(out, r#"<a href="notes.html">label</a>"#);
    }

    #[test]
    fn heading_link_with_filepath_keeps_the_path() {
        // `{:docs/readme.norg:# Install}` must link into the target document,
        // not to a same-page anchor.
        let out = link_html(
            LinkTarget::Heading {
                level: 1,
                title: vec![text("Install")],
            },
            None,
            Some("docs/readme.norg"),
        );
        assert_eq!(out, r##"<a href="docs/readme.html#install">Install</a>"##);
    }

    #[test]
    fn javascript_scheme_link_is_dropped_to_plain_text() {
        // A crafted `javascript:` target must not become a clickable script
        // URL; the link degrades to its display text.
        let out = link_html(
            LinkTarget::Url("javascript:alert(document.cookie)".into()),
            Some(&[text("click me")]),
            None,
        );
        assert_eq!(out, "click me");
    }

    #[test]
    fn protocol_relative_url_is_treated_as_external() {
        // `//host` is a cross-origin URL, not an in-site path: it must be
        // emitted as-is (never rewritten to `.html`) and get external-link
        // hardening, not rendered as a bare same-site link.
        let out = link_html(
            LinkTarget::Url("//cdn.example.com/x".into()),
            Some(&[text("cdn")]),
            None,
        );
        assert_eq!(
            out,
            r#"<a href="//cdn.example.com/x" target="_blank" rel="noopener noreferrer">cdn</a>"#
        );
    }

    #[test]
    fn non_addressable_links_keep_their_display_text() {
        let out = link_html(
            LinkTarget::Definition(vec![text("target")]),
            Some(&[text("shown")]),
            None,
        );
        assert_eq!(out, "shown");
    }

    #[test]
    fn anchor_descriptions_are_visible() {
        let anchor = [ParagraphSegment::Anchor {
            content: vec![text("target")],
            description: Some(vec![text("shown")]),
        }];
        assert_eq!(convert_segments(&anchor), "shown");
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
        let out = link_html(
            LinkTarget::Url("https://a.com".into()),
            Some(&[text("see "), inner]),
            None,
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

    #[test]
    fn exhausted_ids_drop_the_anchor_rather_than_panicking() {
        // Unreachable today; if it ever happens the node must lose only its
        // anchor rather than abort the parse thread.
        let ids = vec!["first".to_string()];
        let mut next = 0;
        let (taken, diagnostics) = crate::diagnostics::capture(|| {
            [
                take_id(&ids, &mut next, "heading"),
                take_id(&ids, &mut next, "heading"),
            ]
        });

        assert_eq!(taken, ["first".to_string(), String::new()]);
        assert_eq!(next, 2, "the cursor must still advance past the miss");
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert!(
            diagnostics[0].contains("heading ids exhausted"),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn only_in_site_paths_are_rewritten_to_html() {
        // `mailto:me@example.norg` is a mailbox, not a document; rewriting it
        // to `.html` points at nothing.
        for target in [
            "mailto:me@example.norg",
            "ftp://host/file.norg",
            "https://example.com/a.norg",
            "//cdn.example.com/a.norg",
        ] {
            assert_eq!(norg_to_html(target), target);
        }

        // A scheme-less path is still rewritten — that's the whole feature.
        assert_eq!(norg_to_html("docs/readme.norg"), "docs/readme.html");
        assert_eq!(norg_to_html("/rooted/a.norg"), "/rooted/a.html");
    }

    #[test]
    fn scheme_targets_reach_the_rewrite_guard_intact() {
        // `convert_link` must hand the raw target to `norg_to_html`.
        let out = link_html(
            LinkTarget::Path("mailto:me@example.norg".into()),
            Some(&[text("label")]),
            None,
        );
        assert_eq!(out, r#"<a href="mailto:me@example.norg">label</a>"#);
    }

    #[test]
    fn only_web_schemes_get_new_tab_hardening() {
        // `mailto:` hands off to the OS; a new tab leaves a blank page.
        let out = link_html(
            LinkTarget::Url("mailto:me@example.com".into()),
            Some(&[text("label")]),
            None,
        );
        assert_eq!(out, r#"<a href="mailto:me@example.com">label</a>"#);
    }
}
