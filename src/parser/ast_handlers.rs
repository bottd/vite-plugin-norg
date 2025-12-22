use crate::segments::convert_segments;
use crate::utils::into_slug;
use arborium::Highlighter;
use htmlescape::encode_minimal;
use rust_norg::{
    DelimitingModifier, DetachedModifierExtension, NorgAST, NorgASTFlat, RangeableDetachedModifier,
    TodoStatus,
};
use textwrap::dedent;

pub fn nestable_modifier(
    text: &NorgASTFlat,
    extensions: &[DetachedModifierExtension],
) -> Option<String> {
    match text {
        NorgASTFlat::Paragraph(segments) => {
            let content = convert_segments(segments);
            (!content.trim().is_empty()).then(|| format_nestable(&content, extensions))
        }
        _ => None,
    }
}

pub fn verbatim_tag(name: &[String], parameters: &[String], content: &str) -> Option<String> {
    match name {
        [tag] if tag == "code" => {
            let code = dedent(content);
            let lang = parameters
                .first()
                .filter(|l| !l.is_empty())
                .map(String::as_str)
                .unwrap_or("text");
            let mut hl = Highlighter::new();
            let highlighted = hl.highlight(lang, &code);

            Some(match highlighted {
                Ok(html) => {
                    let wrapped = wrap_lines(&html);
                    format!(r#"<pre class="arborium lang-{lang}"><code>{wrapped}</code></pre>"#)
                }
                Err(_) => {
                    let wrapped = wrap_lines(&encode_minimal(&code));
                    format!(r#"<pre><code>{wrapped}</code></pre>"#)
                }
            })
        }
        [tag] if tag == "image" => parameters
            .first()
            .filter(|path| !path.is_empty())
            .map(|path| {
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
            }),
        [doc, meta] if doc == "document" && meta == "meta" => None,
        _ => Some(format!(
            r#"<div class="verbatim">{}</div>"#,
            encode_minimal(content)
        )),
    }
}

pub fn heading(level: u16, title: &[rust_norg::ParagraphSegment], content: &[NorgAST]) -> String {
    let text = convert_segments(title);
    let id = into_slug(&text);
    let heading = format!("<h{level} id=\"{id}\">{text}</h{level}>");
    let body = crate::transform(content);

    if body.trim().is_empty() {
        heading
    } else {
        format!("{heading}\n{body}")
    }
}

pub fn paragraph(segments: &[rust_norg::ParagraphSegment]) -> Option<String> {
    let content = convert_segments(segments);
    (!content.trim().is_empty()).then(|| format!("<p>{content}</p>"))
}

pub fn rangeable_modifier(
    modifier_type: &RangeableDetachedModifier,
    title: &[rust_norg::ParagraphSegment],
    content: &[NorgASTFlat],
) -> String {
    let title = convert_segments(title);
    let body: String = content
        .iter()
        .filter_map(|node| match node {
            NorgASTFlat::Paragraph(segments) => {
                let html = convert_segments(segments);
                (!html.trim().is_empty()).then(|| format!("<p>{html}</p>"))
            }
            _ => None,
        })
        .collect();

    match modifier_type {
        RangeableDetachedModifier::Definition => {
            format!(
                "<dl><dt>{}</dt><dd>{}</dd></dl>",
                encode_minimal(&title),
                body
            )
        }
        RangeableDetachedModifier::Footnote => {
            let id = into_slug(&title);
            format!(
                "<aside id=\"footnote-{}\" class=\"footnote\"><strong>{}</strong><p>{}</p></aside>",
                encode_minimal(&id),
                encode_minimal(&title),
                body
            )
        }
        RangeableDetachedModifier::Table => {
            format!(
                "<table><caption>{}</caption><tbody>{}</tbody></table>",
                encode_minimal(&title),
                body
            )
        }
    }
}

pub fn delimiter(delim: &DelimitingModifier) -> String {
    match delim {
        DelimitingModifier::Weak => "<hr class=\"weak\" />",
        DelimitingModifier::Strong => "<hr class=\"strong\" />",
        DelimitingModifier::HorizontalRule => "<hr />",
    }
    .into()
}

fn format_nestable(content: &str, extensions: &[DetachedModifierExtension]) -> String {
    let mut classes: Vec<String> = Vec::new();
    let mut attrs: Vec<String> = Vec::new();
    let mut prefix: Vec<&str> = Vec::new();

    for extension in extensions {
        match extension {
            DetachedModifierExtension::Todo(status) => {
                if matches!(status, TodoStatus::Recurring(_)) {
                    classes.push("todo-recurring".into());
                }
                prefix.push(todo_html(status));
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

    let class_attr = if classes.is_empty() {
        String::new()
    } else {
        format!(r#" class="{}""#, classes.join(" "))
    };
    let data_attrs = if attrs.is_empty() {
        String::new()
    } else {
        format!(" {}", attrs.join(" "))
    };
    let prefix_html = if prefix.is_empty() {
        String::new()
    } else {
        format!("{} ", prefix.join(" "))
    };

    format!("<li{class_attr}{data_attrs}>{prefix_html}{content}</li>")
}

fn todo_html(status: &TodoStatus) -> &'static str {
    match status {
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
        TodoStatus::Recurring(_) => r#"<span class="todo-status todo-recurring">+</span>"#,
    }
}

/// Wraps each of highlighted HTML in `<span class="line">`
/// This enables per-line styling such as line numbers or highlighting specific lines
fn wrap_lines(html: &str) -> String {
    html.lines()
        .map(|line| format!(r#"<span class="line">{line}</span>"#))
        .collect::<Vec<_>>()
        .join("")
}
