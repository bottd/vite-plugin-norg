use crate::segments::convert_segments;
use crate::utils::into_slug;
use htmlescape::encode_minimal;
use rust_norg::{DetachedModifierExtension, NorgASTFlat, TodoStatus};
use std::fmt::Write;

pub fn nestable_modifier(
    text: &NorgASTFlat,
    extensions: &[DetachedModifierExtension],
    children_html: &str,
) -> Option<String> {
    let content = match text {
        NorgASTFlat::Paragraph(segments) => convert_segments(segments),
        _ => {
            eprintln!("Warning: non-Paragraph text in nestable modifier");
            String::new()
        }
    };
    if content.trim().is_empty() && children_html.trim().is_empty() && extensions.is_empty() {
        return None;
    }
    Some(format_nestable(&content, extensions, children_html))
}

fn format_nestable(
    content: &str,
    extensions: &[DetachedModifierExtension],
    children_html: &str,
) -> String {
    let mut classes = String::new();
    let mut attrs = String::new();
    let mut prefix = String::new();

    for extension in extensions {
        match extension {
            DetachedModifierExtension::Todo(status) => {
                if matches!(status, TodoStatus::Recurring(_)) {
                    push_class(&mut classes, "todo-recurring");
                }
                if !prefix.is_empty() {
                    prefix.push(' ');
                }
                prefix.push_str(todo_html(status));
            }
            DetachedModifierExtension::Priority(priority) => {
                push_class(&mut classes, &format!("priority-{}", into_slug(priority)));
                push_attr(&mut attrs, "data-priority", priority);
            }
            DetachedModifierExtension::Timestamp(timestamp) => {
                push_attr(&mut attrs, "data-timestamp", timestamp);
            }
            DetachedModifierExtension::DueDate(date) => {
                push_attr(&mut attrs, "data-due", date);
            }
            DetachedModifierExtension::StartDate(date) => {
                push_attr(&mut attrs, "data-start", date);
            }
        }
    }

    let class_attr = if classes.is_empty() {
        String::new()
    } else {
        format!(r#" class="{classes}""#)
    };
    let prefix_html = if prefix.is_empty() {
        String::new()
    } else if content.is_empty() {
        prefix
    } else {
        format!("{prefix} ")
    };

    format!("<li{class_attr}{attrs}>{prefix_html}{content}{children_html}</li>")
}

fn push_class(buf: &mut String, class: &str) {
    if !buf.is_empty() {
        buf.push(' ');
    }
    buf.push_str(class);
}

fn push_attr(buf: &mut String, name: &str, value: &str) {
    let _ = write!(buf, r#" {name}="{}""#, encode_minimal(value));
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
