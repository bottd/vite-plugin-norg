use crate::segments::convert_segments;
use crate::utils::into_slug;
use htmlescape::encode_minimal;
use rust_norg::{
    DelimitingModifier, DetachedModifierExtension, NestableDetachedModifier, NorgAST, NorgASTFlat,
    RangeableDetachedModifier, TodoStatus,
};
use std::fmt::Write;
use textwrap::dedent;

pub fn handle_nestable_modifier(
    modifier_type: &NestableDetachedModifier,
    text: &NorgASTFlat,
    extensions: &[DetachedModifierExtension],
) -> Option<String> {
    match text {
        NorgASTFlat::Paragraph(segments) => {
            let content = convert_segments(segments);
            if !content.trim().is_empty() {
                let tag = match modifier_type {
                    NestableDetachedModifier::UnorderedList => "ul",
                    NestableDetachedModifier::OrderedList => "ol",
                    NestableDetachedModifier::Quote => "blockquote",
                };
                Some(format!(
                    "<{tag}>{}</{tag}>",
                    format_list_item(&content, extensions)
                ))
            } else {
                None
            }
        }
        _ => {
            eprintln!("Warning: Unsupported nestable modifier text type");
            None
        }
    }
}

pub fn handle_verbatim_tag(
    name: &[String],
    parameters: &[String],
    content: &str,
) -> Option<String> {
    match name {
        [tag] if tag == "code" => {
            let text = encode_minimal(&dedent(content));
            if let Some(lang) = parameters.first().filter(|l| !l.is_empty()) {
                Some(format!(
                    r#"<pre class="lang-{lang}"><code class="lang-{lang}">{text}</code></pre>"#
                ))
            } else {
                Some(format!("<pre><code>{text}</code></pre>"))
            }
        }
        [tag] if tag == "image" => {
            let path = parameters.first().filter(|p| !p.is_empty());
            match path {
                Some(path) => {
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
                None => None,
            }
        }
        [doc, meta] if doc == "document" && meta == "meta" => None,
        _ => Some(format!(
            r#"<div class="verbatim">{}</div>"#,
            encode_minimal(content)
        )),
    }
}

pub fn handle_heading(
    level: u16,
    title: &[rust_norg::ParagraphSegment],
    content: &[NorgAST],
) -> String {
    let text = convert_segments(title);
    let id = into_slug(&text);
    let heading = format!("<h{level} id=\"{id}\">{text}</h{level}>");

    let html = crate::transform(content);

    if html.trim().is_empty() {
        heading
    } else {
        format!("{heading}\n{html}")
    }
}

pub fn handle_paragraph(segments: &[rust_norg::ParagraphSegment]) -> Option<String> {
    let content = convert_segments(segments);
    if content.trim().is_empty() {
        None
    } else {
        Some(format!("<p>{content}</p>"))
    }
}

pub fn handle_rangeable_modifier(
    modifier_type: &RangeableDetachedModifier,
    title: &[rust_norg::ParagraphSegment],
    content: &[NorgASTFlat],
) -> String {
    let title = convert_segments(title);
    let mut content_html = String::with_capacity(content.len() * 64);
    for node in content {
        if let NorgASTFlat::Paragraph(segments) = node {
            let html = convert_segments(segments);
            if !html.trim().is_empty() {
                content_html.push_str("<p>");
                content_html.push_str(&html);
                content_html.push_str("</p>");
            }
        }
    }
    let content = content_html;

    match modifier_type {
        RangeableDetachedModifier::Definition => format!(
            "<dl><dt>{}</dt><dd>{}</dd></dl>",
            encode_minimal(&title),
            content
        ),
        RangeableDetachedModifier::Footnote => {
            let id = into_slug(&title);
            format!(
                "<aside id=\"footnote-{}\" class=\"footnote\"><strong>{}</strong><p>{}</p></aside>",
                encode_minimal(&id),
                encode_minimal(&title),
                content
            )
        }
        RangeableDetachedModifier::Table => format!(
            "<table><caption>{}</caption><tbody>{}</tbody></table>",
            encode_minimal(&title),
            content
        ),
    }
}

pub fn handle_delimiter(delimiter: &DelimitingModifier) -> String {
    match delimiter {
        DelimitingModifier::Weak => "<hr class=\"weak\" />".to_string(),
        DelimitingModifier::Strong => "<hr class=\"strong\" />".to_string(),
        DelimitingModifier::HorizontalRule => "<hr />".to_string(),
    }
}

fn format_list_item(content: &str, extensions: &[DetachedModifierExtension]) -> String {
    let mut classes = Vec::new();
    let mut attrs = Vec::new();
    let mut prefix = Vec::new();

    for extension in extensions {
        match extension {
            DetachedModifierExtension::Todo(status) => {
                if let TodoStatus::Recurring(_) = status {
                    classes.push("todo-recurring".to_string());
                }
                let html = match status {
                    TodoStatus::Undone => {
                        r#"<input type="checkbox" class="todo-status todo-undone" disabled />"#
                    }
                    TodoStatus::Done => {
                        r#"<input type="checkbox" class="todo-status todo-done" checked disabled />"#
                    }
                    TodoStatus::NeedsClarification => {
                        r#"<span class="todo-status todo-clarification">?</span>"#
                    }
                    TodoStatus::Paused => r#"<span class="todo-status todo-paused">=</span>"#,
                    TodoStatus::Urgent => r#"<span class="todo-status todo-urgent">!</span>"#,
                    TodoStatus::Pending => r#"<span class="todo-status todo-pending">-</span>"#,
                    TodoStatus::Canceled => r#"<span class="todo-status todo-canceled">_</span>"#,
                    TodoStatus::Recurring(date) => {
                        let date = date.as_deref().unwrap_or("");
                        return format!(
                            r#"<span class="todo-status todo-recurring">+ {}</span>"#,
                            encode_minimal(date)
                        );
                    }
                };
                prefix.push(html.to_string());
            }
            DetachedModifierExtension::Priority(priority) => {
                classes.push(format!("priority-{}", into_slug(priority)));
                attrs.push(format!(r#"data-priority="{}""#, encode_minimal(priority)));
            }
            DetachedModifierExtension::Timestamp(timestamp) => {
                attrs.push(format!(r#"data-timestamp="{}""#, encode_minimal(timestamp)));
            }
            DetachedModifierExtension::DueDate(date) => {
                attrs.push(format!(r#"data-due="{}""#, encode_minimal(date)));
            }
            DetachedModifierExtension::StartDate(date) => {
                attrs.push(format!(r#"data-start="{}""#, encode_minimal(date)));
            }
        }
    }

    let mut result = String::new();
    write!(&mut result, "<li").unwrap();

    if !classes.is_empty() {
        write!(&mut result, r#" class="{}""#, classes.join(" ")).unwrap();
    }

    for attr in &attrs {
        write!(&mut result, " {attr}").unwrap();
    }

    write!(&mut result, ">").unwrap();

    if !prefix.is_empty() {
        write!(&mut result, "{} ", prefix.join(" ")).unwrap();
    }

    write!(&mut result, "{content}</li>").unwrap();
    result
}
