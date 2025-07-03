use crate::types::TocEntry;
use crate::utils::into_slug;
use htmlescape::encode_minimal;
use rust_norg::{
    DelimitingModifier, DetachedModifierExtension, LinkTarget, NestableDetachedModifier, NorgAST,
    NorgASTFlat, ParagraphSegment, ParagraphSegmentToken, RangeableDetachedModifier, TodoStatus,
};
use std::fmt::Write;
use textwrap::dedent;

/// Converts a Norg AST to HTML and extracts table of contents entries.
pub fn convert_nodes(ast: &[NorgAST]) -> (String, Vec<TocEntry>) {
    let mut toc = Vec::new();
    let mut html_parts = Vec::with_capacity(ast.len());
    let mut index = 0;

    while index < ast.len() {
        match &ast[index] {
            NorgAST::NestableDetachedModifier { modifier_type, .. } => {
                let (html, consumed_nodes) =
                    convert_grouped_modifiers(&ast[index..], modifier_type);
                if !html.is_empty() {
                    html_parts.push(html);
                }
                index += consumed_nodes;
            }
            node => {
                if let Some(html) = convert_single_node(node, &mut toc) {
                    html_parts.push(html);
                }
                index += 1;
            }
        }
    }

    (html_parts.join("\n"), toc)
}

fn convert_single_node(node: &NorgAST, toc: &mut Vec<TocEntry>) -> Option<String> {
    let html = match node {
        NorgAST::VerbatimRangedTag {
            name,
            parameters,
            content,
            ..
        } => match name.as_slice() {
            [tag] if tag == "code" => {
                let encoded = encode_minimal(&dedent(content));
                if let Some(lang) = parameters.first().filter(|l| !l.is_empty()) {
                    format!(
                        r#"<pre class="language-{lang}"><code class="language-{lang}">{encoded}</code></pre>"#
                    )
                } else {
                    format!("<pre><code>{encoded}</code></pre>")
                }
            }
            [tag] if tag == "image" => {
                let path = parameters.first().filter(|p| !p.is_empty())?;
                let src = if path.starts_with('/') || path.starts_with("http") {
                    path.clone()
                } else {
                    format!("./{path}")
                };
                format!(
                    r#"<img src="{}" alt="{}" />"#,
                    encode_minimal(&src),
                    encode_minimal(content.trim())
                )
            }
            [doc, meta] if doc == "document" && meta == "meta" => return None,
            _ => format!(r#"<div class="verbatim">{}</div>"#, encode_minimal(content)),
        },

        NorgAST::Heading {
            level,
            title,
            content,
            ..
        } => {
            let title_text = convert_paragraph_segments(title);
            let heading_id = into_slug(&title_text);
            let heading = format!("<h{level} id=\"{heading_id}\">{title_text}</h{level}>");

            toc.push(TocEntry {
                level: *level as usize,
                title: title_text,
                id: heading_id,
            });
            let (content_html, _) = convert_nodes(content);

            if content_html.trim().is_empty() {
                heading
            } else {
                format!("{heading}\n{content_html}")
            }
        }

        NorgAST::Paragraph(segments) => {
            let content = convert_paragraph_segments(segments);
            if content.trim().is_empty() {
                return None;
            }
            format!("<p>{content}</p>")
        }

        NorgAST::RangeableDetachedModifier {
            modifier_type,
            title,
            content,
            ..
        } => {
            let title_html = convert_paragraph_segments(title);
            let content_html = content
                .iter()
                .filter_map(|node| match node {
                    NorgASTFlat::Paragraph(segments) => {
                        let html = convert_paragraph_segments(segments);
                        (!html.trim().is_empty()).then(|| format!("<p>{html}</p>"))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");

            match modifier_type {
                RangeableDetachedModifier::Definition => format!(
                    "<dl><dt>{}</dt><dd>{}</dd></dl>",
                    encode_minimal(&title_html),
                    content_html
                ),
                RangeableDetachedModifier::Footnote => {
                    let id = into_slug(&title_html);
                    format!(
                        "<aside id=\"footnote-{}\" class=\"footnote\"><strong>{}</strong><p>{}</p></aside>",
                        encode_minimal(&id), encode_minimal(&title_html), content_html
                    )
                }
                RangeableDetachedModifier::Table => format!(
                    "<table><caption>{}</caption><tbody>{}</tbody></table>",
                    encode_minimal(&title_html),
                    content_html
                ),
            }
        }

        NorgAST::DelimitingModifier(delimiter) => match delimiter {
            DelimitingModifier::Weak => "<hr class=\"weak\" />".to_string(),
            DelimitingModifier::Strong => "<hr class=\"strong\" />".to_string(),
            DelimitingModifier::HorizontalRule => "<hr />".to_string(),
        },

        NorgAST::NestableDetachedModifier { .. } => return None,

        NorgAST::CarryoverTag { .. } | NorgAST::RangedTag { .. } | NorgAST::InfirmTag { .. } => {
            todo!()
        }
    };

    Some(html)
}

fn convert_grouped_modifiers(
    ast: &[NorgAST],
    modifier_type: &NestableDetachedModifier,
) -> (String, usize) {
    let matching_nodes: Vec<_> = ast.iter()
        .take_while(|node| matches!(node, NorgAST::NestableDetachedModifier { modifier_type: mt, .. } if mt == modifier_type))
        .collect();

    let items: Vec<String> = matching_nodes
        .iter()
        .filter_map(|node| match node {
            NorgAST::NestableDetachedModifier {
                text, extensions, ..
            } => match text.as_ref() {
                NorgASTFlat::Paragraph(segments) => {
                    let content = convert_paragraph_segments(segments);
                    (!content.trim().is_empty()).then(|| format_list_item(&content, extensions))
                }
                _ => None,
            },
            _ => None,
        })
        .collect();

    let html = if items.is_empty() {
        (String::new(), matching_nodes.len())
    } else {
        let tag_name = match modifier_type {
            NestableDetachedModifier::UnorderedList => "ul",
            NestableDetachedModifier::OrderedList => "ol",
            NestableDetachedModifier::Quote => "blockquote",
        };
        (
            format!("<{tag_name}>\n{}\n</{tag_name}>", items.join("\n")),
            matching_nodes.len(),
        )
    };

    html
}

fn format_list_item(content: &str, extensions: &[DetachedModifierExtension]) -> String {
    let mut classes = Vec::new();
    let mut attributes = Vec::new();
    let mut prefix_parts = Vec::new();

    for ext in extensions {
        match ext {
            DetachedModifierExtension::Todo(status) => {
                if let TodoStatus::Recurring(_) = status {
                    classes.push("todo-recurring".to_string());
                }
                let html = {
                    match status {
                        TodoStatus::Undone => r#"<input type="checkbox" class="todo-status todo-undone" disabled />"#.to_string(),
                        TodoStatus::Done => r#"<input type="checkbox" class="todo-status todo-done" checked disabled />"#.to_string(),
                        TodoStatus::NeedsClarification => r#"<span class="todo-status todo-clarification">?</span>"#.to_string(),
                        TodoStatus::Paused => r#"<span class="todo-status todo-paused">="</span>"#.to_string(),
                        TodoStatus::Urgent => r#"<span class="todo-status todo-urgent">!</span>"#.to_string(),
                        TodoStatus::Pending => r#"<span class="todo-status todo-pending">-</span>"#.to_string(),
                        TodoStatus::Canceled => r#"<span class="todo-status todo-canceled">_</span>"#.to_string(),
                        TodoStatus::Recurring(date) => {
                            let date_text = date.as_deref().unwrap_or("");
                            format!(r#"<span class="todo-status todo-recurring">+ {}</span>"#, encode_minimal(date_text))
                        }
                    }
                };
                prefix_parts.push(html);
            }
            DetachedModifierExtension::Priority(p) => {
                classes.push(format!("priority-{}", into_slug(p)));
                attributes.push(format!(r#"data-priority="{}""#, encode_minimal(p)));
            }
            DetachedModifierExtension::Timestamp(ts) => {
                attributes.push(format!(r#"data-timestamp="{}""#, encode_minimal(ts)));
            }
            DetachedModifierExtension::DueDate(date) => {
                attributes.push(format!(r#"data-due="{}""#, encode_minimal(date)));
            }
            DetachedModifierExtension::StartDate(date) => {
                attributes.push(format!(r#"data-start="{}""#, encode_minimal(date)));
            }
        }
    }

    let mut result = String::new();
    write!(&mut result, "<li").unwrap();

    if !classes.is_empty() {
        write!(&mut result, r#" class="{}""#, classes.join(" ")).unwrap();
    }

    for attr in &attributes {
        write!(&mut result, " {}", attr).unwrap();
    }

    write!(&mut result, ">").unwrap();

    if !prefix_parts.is_empty() {
        write!(&mut result, "{} ", prefix_parts.join(" ")).unwrap();
    }

    write!(&mut result, "{}</li>", content).unwrap();
    result
}

fn process_token(token: &ParagraphSegmentToken, encode: impl Fn(&str) -> String) -> String {
    match token {
        ParagraphSegmentToken::Whitespace => " ".into(),
        ParagraphSegmentToken::Text(text) => encode(text),
        ParagraphSegmentToken::Special(ch) => encode(&ch.to_string()),
        ParagraphSegmentToken::Escape(ch) => ch.to_string(),
    }
}

fn convert_paragraph_segments_with_code_escaping(segments: &[ParagraphSegment]) -> String {
    segments
        .iter()
        .filter_map(|segment| match segment {
            ParagraphSegment::Token(token) => Some(process_token(token, encode_minimal)),
            _ => None,
        })
        .collect()
}

fn convert_paragraph_segments(segments: &[ParagraphSegment]) -> String {
    segments
        .iter()
        .map(|segment| match segment {
            ParagraphSegment::Token(token) => process_token(token, encode_minimal),

            ParagraphSegment::AttachedModifier {
                modifier_type,
                content,
            } => match *modifier_type {
                '`' => format!(
                    "<code>{}</code>",
                    convert_paragraph_segments_with_code_escaping(content)
                ),
                '*' => format!("<strong>{}</strong>", convert_paragraph_segments(content)),
                '_' => format!("<em>{}</em>", convert_paragraph_segments(content)),
                '^' => format!("<sup>{}</sup>", convert_paragraph_segments(content)),
                ',' => format!("<sub>{}</sub>", convert_paragraph_segments(content)),
                '-' => format!("<s>{}</s>", convert_paragraph_segments(content)),
                '!' => format!(
                    "<span class=\"spoiler\">{}</span>",
                    convert_paragraph_segments(content)
                ),
                '$' => format!(
                    "<span class=\"math\">{}</span>",
                    convert_paragraph_segments(content)
                ),
                '&' => format!("<var>{}</var>", convert_paragraph_segments(content)),
                '/' => format!("<i>{}</i>", convert_paragraph_segments(content)),
                '=' => format!("<mark>{}</mark>", convert_paragraph_segments(content)),
                _ => convert_paragraph_segments(content),
            },

            ParagraphSegment::Link {
                targets,
                description,
                ..
            } => targets
                .first()
                .map(|target| {
                    convert_link(
                        target,
                        description
                            .as_ref()
                            .map(|d| convert_paragraph_segments(d))
                            .as_deref(),
                    )
                })
                .unwrap_or_default(),

            ParagraphSegment::Anchor { content, .. } => convert_paragraph_segments(content),

            ParagraphSegment::InlineVerbatim(tokens) => {
                format!(
                    "<code>{}</code>",
                    encode_minimal(&tokens.iter().map(ToString::to_string).collect::<String>())
                )
            }

            _ => String::new(),
        })
        .collect()
}

fn convert_link(target: &LinkTarget, custom_text: Option<&str>) -> String {
    match target {
        LinkTarget::Url(url) => {
            let display = custom_text.unwrap_or(url);
            if url.starts_with("http") {
                format!(
                    r#"<a href="{}" target="_blank">{}</a>"#,
                    encode_minimal(url),
                    encode_minimal(display)
                )
            } else {
                let href = url
                    .strip_suffix(".norg")
                    .map(|base| format!("{base}.html"))
                    .unwrap_or_else(|| url.clone());
                format!(
                    r#"<a href="{}">{}</a>"#,
                    encode_minimal(&href),
                    encode_minimal(display)
                )
            }
        }
        LinkTarget::Heading { title, .. } => {
            let title_text: String = title.iter().map(|seg| format!("{seg:?}")).collect();
            let slug = into_slug(&title_text);
            format!(
                "<a href=\"#{}\">{}</a>",
                encode_minimal(&slug),
                encode_minimal(custom_text.unwrap_or(&title_text))
            )
        }
        _ => format!("{:?}", target),
    }
}
