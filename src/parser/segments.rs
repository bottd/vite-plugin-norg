use crate::utils::into_slug;
use htmlescape::encode_minimal;
use rust_norg::{LinkTarget, ParagraphSegment, ParagraphSegmentToken};

pub fn convert_segments(segments: &[ParagraphSegment]) -> String {
    let mut out = String::with_capacity(segments.len() * 32);
    for segment in segments {
        out.push_str(&convert_segment(segment));
    }
    out
}

pub fn convert_code_segments(segments: &[ParagraphSegment]) -> String {
    segments
        .iter()
        .filter_map(|segment| match segment {
            ParagraphSegment::Token(token) => Some(render_token(token)),
            _ => None,
        })
        .collect()
}

fn convert_segment(segment: &ParagraphSegment) -> String {
    match segment {
        ParagraphSegment::Token(token) => render_token(token),

        ParagraphSegment::AttachedModifier {
            modifier_type,
            content,
        } => convert_attached_modifier(*modifier_type, content),

        ParagraphSegment::Link {
            targets,
            description,
            filepath,
            ..
        } => convert_link(targets, description.as_deref(), filepath.as_deref()),

        ParagraphSegment::Anchor { content, .. } => convert_segments(content),

        ParagraphSegment::InlineVerbatim(tokens) => {
            let text: String = tokens.iter().map(ToString::to_string).collect();
            format!("<code>{}</code>", encode_minimal(&text))
        }

        _ => {
            eprintln!("Warning: Unsupported paragraph segment type");
            String::new()
        }
    }
}

fn render_token(token: &ParagraphSegmentToken) -> String {
    match token {
        ParagraphSegmentToken::Whitespace => " ".into(),
        ParagraphSegmentToken::Text(text) => encode_minimal(text),
        ParagraphSegmentToken::Special(ch) => {
            let mut buf = [0u8; 4];
            encode_minimal(ch.encode_utf8(&mut buf))
        }
        ParagraphSegmentToken::Escape(ch) => ch.to_string(),
    }
}

fn convert_attached_modifier(modifier_type: char, content: &[ParagraphSegment]) -> String {
    if modifier_type == '`' {
        return format!("<code>{}</code>", convert_code_segments(content));
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
        _ => return convert_segments(content),
    };
    let inner = convert_segments(content);
    format!("{open}{inner}{close}")
}

/// `.norg` paths are rewritten to `.html` so links resolve in the build output.
fn norg_to_html(path: &str) -> String {
    path.strip_suffix(".norg")
        .map(|base| format!("{base}.html"))
        .unwrap_or_else(|| path.to_string())
}

fn anchor(href: &str, display: &str, external: bool) -> String {
    let target = if external { r#" target="_blank""# } else { "" };
    format!(
        r#"<a href="{}"{target}>{}</a>"#,
        encode_minimal(href),
        encode_minimal(display)
    )
}

fn convert_link(
    targets: &[LinkTarget],
    description: Option<&[ParagraphSegment]>,
    filepath: Option<&str>,
) -> String {
    let display = description.map(convert_segments);

    match targets.first() {
        Some(LinkTarget::Url(url)) => {
            let display_text = display.as_deref().unwrap_or(url);
            match filepath {
                Some(fp) => anchor(fp, display_text, false),
                None if url.starts_with("http") => anchor(url, display_text, true),
                None => anchor(&norg_to_html(url), display_text, false),
            }
        }
        Some(LinkTarget::Heading { title, .. }) => {
            let title_html = convert_segments(title);
            let slug = into_slug(&title_html);
            let display_text = display.as_deref().unwrap_or(&title_html);
            format!("<a href=\"#{slug}\">{display_text}</a>")
        }
        Some(LinkTarget::Path(path)) => anchor(
            &norg_to_html(path),
            display.as_deref().unwrap_or(path),
            false,
        ),
        Some(_) => String::new(),
        None => filepath
            .map(|fp| anchor(&norg_to_html(fp), display.as_deref().unwrap_or(fp), false))
            .unwrap_or_default(),
    }
}
