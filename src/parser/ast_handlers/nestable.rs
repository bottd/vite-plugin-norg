use crate::segments::convert_segments;
use crate::utils::into_slug;
use htmlescape::encode_minimal;
use rust_norg::{DetachedModifierExtension, NorgASTFlat, TodoStatus};

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
