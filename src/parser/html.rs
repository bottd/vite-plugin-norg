use crate::types::TocEntry;
use crate::utils::into_slug;
use htmlescape::encode_minimal;
use rust_norg::{
    DelimitingModifier, DetachedModifierExtension, LinkTarget, NestableDetachedModifier, NorgAST,
    NorgASTFlat, ParagraphSegment, ParagraphSegmentToken, RangeableDetachedModifier, TodoStatus,
};
use textwrap::dedent;

const HTML_TAGS: &[(char, &str, &str)] = &[
    ('*', "<strong>", "</strong>"),
    ('_', "<em>", "</em>"),
    ('^', "<sup>", "</sup>"),
    (',', "<sub>", "</sub>"),
    ('-', "<s>", "</s>"),
    ('!', "<span class=\"spoiler\">", "</span>"),
    ('$', "<span class=\"math\">", "</span>"),
    ('&', "<var>", "</var>"),
    ('/', "<i>", "</i>"),
    ('=', "<mark>", "</mark>"),
];

fn todo_status_html(status: &TodoStatus) -> String {
    match status {
        TodoStatus::Undone => r#"<input type="checkbox" class="todo-status todo-undone" disabled />"#.into(),
        TodoStatus::Done => r#"<input type="checkbox" class="todo-status todo-done" checked disabled />"#.into(),
        TodoStatus::NeedsClarification => r#"<span class="todo-status todo-clarification">?</span>"#.into(),
        TodoStatus::Paused => r#"<span class="todo-status todo-paused">=</span>"#.into(),
        TodoStatus::Urgent => r#"<span class="todo-status todo-urgent">!</span>"#.into(),
        TodoStatus::Pending => r#"<span class="todo-status todo-pending">-</span>"#.into(),
        TodoStatus::Canceled => r#"<span class="todo-status todo-canceled">_</span>"#.into(),
        TodoStatus::Recurring(date) => format!(
            r#"<span class="todo-status todo-recurring">+ {}</span>"#,
            encode_minimal(date.as_deref().unwrap_or(""))
        ),
    }
}

fn convert_verbatim_tag(name: &[String], parameters: &[String], content: &str) -> Option<String> {
    match name {
        [tag] if tag == "code" => Some(convert_code_block(parameters, content)),
        [tag] if tag == "image" => convert_image_tag(parameters, content),
        [doc, meta] if doc == "document" && meta == "meta" => None,
        _ => Some(format!("<div class=\"verbatim\">{}</div>", encode_minimal(content))),
    }
}

fn convert_code_block(parameters: &[String], content: &str) -> String {
    let encoded = encode_minimal(&dedent(content));
    match parameters.first().filter(|l| !l.is_empty()) {
        Some(lang) => format!(r#"<pre class="language-{lang}"><code class="language-{lang}">{encoded}</code></pre>"#),
        None => format!("<pre><code>{encoded}</code></pre>"),
    }
}

fn convert_image_tag(parameters: &[String], content: &str) -> Option<String> {
    let path = parameters.first().filter(|p| !p.is_empty())?;
    let src = if path.starts_with('/') || path.starts_with("http") {
        path.clone()
    } else {
        format!("./{path}")
    };
    Some(format!(
        r#"<img src="{}" alt="{}" />"#,
        encode_minimal(&src),
        encode_minimal(content.trim())
    ))
}

const fn delimiter_html(delimiter: &DelimitingModifier) -> &'static str {
    match delimiter {
        DelimitingModifier::Weak => "<hr class=\"weak\" />",
        DelimitingModifier::Strong => "<hr class=\"strong\" />",
        DelimitingModifier::HorizontalRule => "<hr />",
    }
}

pub fn convert_nodes(ast: &[NorgAST]) -> (String, Vec<TocEntry>) {
    let mut toc = Vec::new();
    let mut result = Vec::new();
    let mut i = 0;

    while i < ast.len() {
        match &ast[i] {
            NorgAST::NestableDetachedModifier { modifier_type, .. } => {
                let (html, consumed) = convert_grouped_modifiers(&ast[i..], modifier_type);
                if !html.is_empty() {
                    result.push(html);
                }
                i += consumed;
            }
            node => {
                convert_single_node(node, &mut toc).map(|html| result.push(html));
                i += 1;
            }
        }
    }

    (result.join("\n"), toc)
}

fn convert_single_node(node: &NorgAST, toc: &mut Vec<TocEntry>) -> Option<String> {
    let html = match node {
        NorgAST::VerbatimRangedTag {
            name,
            parameters,
            content,
            ..
        } => convert_verbatim_tag(name, parameters, content)?,

        NorgAST::Heading { level, title, content, .. } => {
            let title_text = convert_paragraph_segments(title);
            let heading_id = into_slug(&title_text);
            let heading = format!("<h{level} id=\"{heading_id}\">{title_text}</h{level}>");

            toc.push(TocEntry {
                level: *level as usize,
                title: title_text,
                id: heading_id,
            });
            let (content_html, _) = convert_nodes(content);

            match content_html.trim().is_empty() {
                true => heading,
                false => format!("{heading}\n{content_html}"),
            }
        }

        NorgAST::Paragraph(segments) => {
            let content = convert_paragraph_segments(segments);
            (!content.trim().is_empty()).then(|| format!("<p>{content}</p>"))?
        }

        NorgAST::RangeableDetachedModifier {
            modifier_type,
            title,
            content,
            ..
        } => {
            let title_html = convert_paragraph_segments(title);
            let content_html = convert_flat_content(content);

            match modifier_type {
                RangeableDetachedModifier::Definition => {
                    format!(
                        "<dl><dt>{}</dt><dd>{}</dd></dl>",
                        encode_minimal(&title_html),
                        content_html
                    )
                }
                RangeableDetachedModifier::Footnote => {
                    let id = into_slug(&title_html);
                    format!(
                        "<aside id=\"footnote-{}\" class=\"footnote\"><strong>{}</strong><p>{}</p></aside>",
                        encode_minimal(&id), encode_minimal(&title_html), content_html
                    )
                }
                RangeableDetachedModifier::Table => {
                    format!(
                        "<table><caption>{}</caption><tbody>{}</tbody></table>",
                        encode_minimal(&title_html),
                        content_html
                    )
                }
            }
        }

        NorgAST::DelimitingModifier(delimiter) => delimiter_html(delimiter).to_string(),

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
    use std::fmt::Write;

    let mut classes = Vec::new();
    let mut attributes = Vec::new();
    let mut prefix_parts = Vec::new();

    for ext in extensions {
        match ext {
            DetachedModifierExtension::Todo(status) => {
                if let TodoStatus::Recurring(_) = status {
                    classes.push("todo-recurring".to_string());
                }
                prefix_parts.push(todo_status_html(status));
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

            ParagraphSegment::AttachedModifier { modifier_type, content } => match *modifier_type {
                '`' => format!("<code>{}</code>", convert_paragraph_segments_with_code_escaping(content)),
                ch => HTML_TAGS.iter()
                    .find_map(|(tag_ch, open, close)| {
                        (*tag_ch == ch).then(|| format!("{open}{}{close}", convert_paragraph_segments(content)))
                    })
                    .unwrap_or_else(|| convert_paragraph_segments(content)),
            },

            ParagraphSegment::Link { targets, description, .. } => targets
                .first()
                .map(|target| convert_link(
                    target,
                    description.as_ref().map(|d| convert_paragraph_segments(d)).as_deref(),
                ))
                .unwrap_or_default(),

            ParagraphSegment::Anchor { content, .. } => convert_paragraph_segments(content),

            ParagraphSegment::InlineVerbatim(tokens) => {
                format!("<code>{}</code>", encode_minimal(&tokens.iter().map(ToString::to_string).collect::<String>()))
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
                format!(r#"<a href="{}" target="_blank">{}</a>"#, encode_minimal(url), encode_minimal(display))
            } else {
                let href = url.strip_suffix(".norg")
                    .map(|base| format!("{base}.html"))
                    .unwrap_or_else(|| url.clone());
                format!(r#"<a href="{}">{}</a>"#, encode_minimal(&href), encode_minimal(display))
            }
        }
        LinkTarget::Heading { title, .. } => {
            let title_text: String = title.iter().map(|seg| format!("{seg:?}")).collect();
            let slug = into_slug(&title_text);
            format!("<a href=\"#{}\">{}</a>", encode_minimal(&slug), encode_minimal(custom_text.unwrap_or(&title_text)))
        }
        _ => format!("{:?}", target),
    }
}

fn convert_flat_content(content: &[NorgASTFlat]) -> String {
    content
        .iter()
        .filter_map(|node| match node {
            NorgASTFlat::Paragraph(segments) => {
                let html = convert_paragraph_segments(segments);
                (!html.trim().is_empty()).then(|| format!("<p>{html}</p>"))
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}
