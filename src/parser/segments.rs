use crate::utils::into_slug;
use htmlescape::encode_minimal;
use rust_norg::{LinkTarget, ParagraphSegment, ParagraphSegmentToken};

pub fn convert_segments(segments: &[ParagraphSegment]) -> String {
    let mut result = String::with_capacity(segments.len() * 32);
    for segment in segments {
        result.push_str(&convert_segment(segment));
    }
    result
}

pub fn convert_code_segments(segments: &[ParagraphSegment]) -> String {
    segments
        .iter()
        .filter_map(|segment| match segment {
            ParagraphSegment::Token(token) => Some(handle_segment_token(token, encode_minimal)),
            _ => None,
        })
        .collect()
}

fn convert_segment(segment: &ParagraphSegment) -> String {
    match segment {
        ParagraphSegment::Token(token) => handle_segment_token(token, encode_minimal),

        ParagraphSegment::AttachedModifier {
            modifier_type,
            content,
        } => convert_attached_modifier(*modifier_type, content),

        ParagraphSegment::Link {
            targets,
            description,
            filepath,
            ..
        } => convert_link(targets, description.as_ref(), filepath.as_ref()),

        ParagraphSegment::Anchor { content, .. } => convert_segments(content),

        ParagraphSegment::InlineVerbatim(tokens) => {
            format!(
                "<code>{}</code>",
                encode_minimal(&tokens.iter().map(ToString::to_string).collect::<String>())
            )
        }

        _ => {
            eprintln!("Warning: Unsupported paragraph segment type");
            String::new()
        }
    }
}

fn handle_segment_token(token: &ParagraphSegmentToken, encode: impl Fn(&str) -> String) -> String {
    match token {
        ParagraphSegmentToken::Whitespace => " ".into(),
        ParagraphSegmentToken::Text(text) => encode(text),
        ParagraphSegmentToken::Special(ch) => encode(&ch.to_string()),
        ParagraphSegmentToken::Escape(ch) => ch.to_string(),
    }
}

fn convert_attached_modifier(modifier_type: char, content: &[ParagraphSegment]) -> String {
    match modifier_type {
        '`' => format!("<code>{}</code>", convert_code_segments(content)),
        '*' => format!("<strong>{}</strong>", convert_segments(content)),
        '_' => format!("<em>{}</em>", convert_segments(content)),
        '^' => format!("<sup>{}</sup>", convert_segments(content)),
        ',' => format!("<sub>{}</sub>", convert_segments(content)),
        '-' => format!("<s>{}</s>", convert_segments(content)),
        '!' => format!(
            "<span class=\"spoiler\">{}</span>",
            convert_segments(content)
        ),
        '$' => format!("<span class=\"math\">{}</span>", convert_segments(content)),
        '&' => format!("<var>{}</var>", convert_segments(content)),
        '/' => format!("<i>{}</i>", convert_segments(content)),
        '=' => format!("<mark>{}</mark>", convert_segments(content)),
        _ => convert_segments(content),
    }
}

fn convert_link(
    targets: &[LinkTarget],
    description: Option<&Vec<ParagraphSegment>>,
    filepath: Option<&String>,
) -> String {
    let text = description.map(|d| convert_segments(d));

    match targets.first() {
        Some(LinkTarget::Url(url)) => {
            let display_text = text.as_deref().unwrap_or(url);
            let href = if let Some(fp) = filepath {
                fp.as_str()
            } else if url.starts_with("http") {
                url.as_str()
            } else if let Some(base) = url.strip_suffix(".norg") {
                return if url.starts_with("http") && filepath.is_none() {
                    format!(
                        r#"<a href="{base}.html" target="_blank">{}</a>"#,
                        encode_minimal(display_text)
                    )
                } else {
                    format!(
                        r#"<a href="{base}.html">{}</a>"#,
                        encode_minimal(display_text)
                    )
                };
            } else {
                url.as_str()
            };

            if url.starts_with("http") && filepath.is_none() {
                format!(
                    r#"<a href="{}" target="_blank">{}</a>"#,
                    encode_minimal(href),
                    encode_minimal(display_text)
                )
            } else {
                format!(
                    r#"<a href="{}">{}</a>"#,
                    encode_minimal(href),
                    encode_minimal(display_text)
                )
            }
        }
        Some(LinkTarget::Heading { title, .. }) => {
            let title_text = title
                .iter()
                .map(|segment| format!("{segment:?}"))
                .collect::<String>();
            let slug = into_slug(&title_text);
            let display_text = text.as_deref().unwrap_or(&title_text);
            format!(
                "<a href=\"#{}\">{}</a>",
                encode_minimal(&slug),
                encode_minimal(display_text)
            )
        }
        Some(LinkTarget::Footnote(name)) => {
            eprintln!("Warning: Footnote links not yet implemented: {name:?}");
            String::new()
        }
        Some(LinkTarget::Definition(name)) => {
            eprintln!("Warning: Definition links not yet implemented: {name:?}");
            String::new()
        }
        Some(LinkTarget::Path(_)) => {
            eprintln!("Warning: Path links not yet implemented");
            String::new()
        }
        Some(LinkTarget::Timestamp(_)) => {
            eprintln!("Warning: Timestamp links not yet implemented");
            String::new()
        }
        Some(LinkTarget::Generic(_)) => {
            eprintln!("Warning: Generic links not yet implemented");
            String::new()
        }
        Some(LinkTarget::Extendable(_)) => {
            eprintln!("Warning: Extendable links not yet implemented");
            String::new()
        }
        Some(LinkTarget::Wiki(_)) => {
            eprintln!("Warning: Wiki links not yet implemented");
            String::new()
        }
        None => String::new(),
    }
}
