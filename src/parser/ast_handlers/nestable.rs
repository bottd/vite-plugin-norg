use crate::ast_handlers::{CommentKind, comment_target, warn_carryover_ignored};
use crate::segments::{DocumentIds, convert_segments_with_ids};
use crate::utils::into_slug;
use htmlescape::encode_minimal;
use rust_norg::{
    DetachedModifierExtension, NestableDetachedModifier, NorgAST, NorgASTFlat, ParagraphSegment,
    TodoStatus,
};
use std::fmt::Write;

pub struct FlatListItem<'a> {
    kind: NestableDetachedModifier,
    level: u16,
    text: &'a [ParagraphSegment],
    extensions: &'a [DetachedModifierExtension],
}

pub enum FlatListEvent<'a> {
    Item(FlatListItem<'a>),
    StrongComment {
        kind: NestableDetachedModifier,
        level: u16,
    },
}

// rust-norg nests only same-marker descendants. Preorder flattening lets one
// level-aware stack handle marker changes without losing the parent item.
pub fn collect_list_items<'a>(node: &'a NorgAST, events: &mut Vec<FlatListEvent<'a>>) -> bool {
    if let Some((comment, target)) = comment_target(node) {
        return match target {
            NorgAST::NestableDetachedModifier {
                modifier_type,
                level,
                content,
                ..
            } => {
                match comment {
                    CommentKind::Strong => events.push(FlatListEvent::StrongComment {
                        kind: *modifier_type,
                        level: *level,
                    }),
                    CommentKind::Weak => collect_list_children(content, events),
                }
                true
            }
            NorgAST::List { items, .. } => {
                if comment == CommentKind::Weak {
                    collect_list_children(items, events);
                }
                true
            }
            _ => false,
        };
    }

    match node {
        NorgAST::List {
            items: list_items, ..
        } => {
            collect_list_children(list_items, events);
            true
        }
        NorgAST::NestableDetachedModifier {
            modifier_type,
            level,
            extensions,
            text,
            content,
        } => {
            // rust-norg parses item text as a paragraph; anything else loses
            // only this item's text, not the items nested under it.
            let NorgASTFlat::Paragraph(text) = text.as_ref() else {
                crate::diagnostics::warn("list item text is not a paragraph — item text skipped");
                collect_list_children(content, events);
                return true;
            };
            events.push(FlatListEvent::Item(FlatListItem {
                kind: *modifier_type,
                level: *level,
                text,
                extensions,
            }));
            collect_list_children(content, events);
            true
        }
        NorgAST::CarryoverTag {
            name, next_object, ..
        } => {
            let consumed = collect_list_items(next_object, events);
            if consumed {
                warn_carryover_ignored(name);
            }
            consumed
        }
        _ => false,
    }
}

fn collect_list_children<'a>(nodes: &'a [NorgAST], events: &mut Vec<FlatListEvent<'a>>) {
    for node in nodes {
        // rust-norg keeps a list item's indented blocks as siblings, so its
        // content holds only further list nodes.
        if !collect_list_items(node, events) {
            crate::diagnostics::warn("unsupported block inside a list item — content skipped");
        }
    }
}

pub fn render_list_items(events: &[FlatListEvent], ids: &DocumentIds) -> String {
    let mut out = String::new();
    let mut stack: Vec<OpenContainer> = Vec::new();
    let mut suppressed = None;

    for event in events {
        let item = match event {
            FlatListEvent::StrongComment { kind, level } => {
                suppressed = Some((*kind, *level));
                continue;
            }
            FlatListEvent::Item(item) => item,
        };

        if let Some((kind, level)) = suppressed {
            if item.level < level || (item.level == level && item.kind != kind) {
                suppressed = None;
            } else {
                continue;
            }
        }

        while stack.last().is_some_and(|open| {
            open.level > item.level || (open.level == item.level && open.kind != item.kind)
        }) {
            close_container(&mut out, stack.pop().unwrap());
        }

        if stack.last().is_some_and(|open| open.level == item.level) {
            close_item(&mut out, item.kind);
        } else {
            if stack.is_empty() && !out.is_empty() {
                out.push('\n');
            }
            let _ = write!(out, "<{}>", container_tag(item.kind));
            stack.push(OpenContainer {
                kind: item.kind,
                level: item.level,
            });
        }

        render_item(item, &mut out, ids);
    }

    while let Some(open) = stack.pop() {
        close_container(&mut out, open);
    }

    out
}

#[derive(Clone, Copy)]
struct OpenContainer {
    kind: NestableDetachedModifier,
    level: u16,
}

fn container_tag(kind: NestableDetachedModifier) -> &'static str {
    match kind {
        NestableDetachedModifier::UnorderedList => "ul",
        NestableDetachedModifier::OrderedList => "ol",
        NestableDetachedModifier::Quote => "blockquote",
    }
}

fn render_item(item: &FlatListItem, out: &mut String, ids: &DocumentIds) {
    let content = convert_segments_with_ids(item.text, ids);
    let blank = content.trim().is_empty();
    let (class_attr, attrs, prefix) = extension_markup(item.extensions);
    let separator = if prefix.is_empty() || blank { "" } else { " " };

    match item.kind {
        NestableDetachedModifier::Quote => {
            if !blank || !prefix.is_empty() || !class_attr.is_empty() || !attrs.is_empty() {
                let _ = write!(
                    out,
                    "<p{class_attr}{attrs}>{prefix}{separator}{content}</p>"
                );
            }
        }
        _ => {
            let _ = write!(out, "<li{class_attr}{attrs}>{prefix}{separator}{content}");
        }
    }
}

fn close_item(out: &mut String, kind: NestableDetachedModifier) {
    if !matches!(kind, NestableDetachedModifier::Quote) {
        out.push_str("</li>");
    }
}

fn close_container(out: &mut String, open: OpenContainer) {
    close_item(out, open.kind);
    let _ = write!(out, "</{}>", container_tag(open.kind));
}

fn extension_markup(extensions: &[DetachedModifierExtension]) -> (String, String, String) {
    let mut classes = String::new();
    let mut attrs = String::new();
    let mut prefix = String::new();

    for extension in extensions {
        match extension {
            DetachedModifierExtension::Todo(status) => {
                if let TodoStatus::Recurring(spec) = status {
                    push_space_separated(&mut classes, "todo-recurring");
                    if let Some(spec) = spec {
                        push_attr(&mut attrs, "data-recur", spec);
                    }
                }
                push_space_separated(&mut prefix, todo_html(status));
            }
            DetachedModifierExtension::Priority(priority) => {
                push_space_separated(&mut classes, &format!("priority-{}", into_slug(priority)));
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
    (class_attr, attrs, prefix)
}

fn push_space_separated(buf: &mut String, value: &str) {
    if !buf.is_empty() {
        buf.push(' ');
    }
    buf.push_str(value);
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

#[cfg(test)]
mod tests {
    use super::*;
    use rust_norg::{DelimitingModifier, ParagraphSegmentToken};

    fn flat_paragraph(text: &str) -> NorgASTFlat {
        NorgASTFlat::Paragraph(vec![ParagraphSegment::Token(ParagraphSegmentToken::Text(
            text.into(),
        ))])
    }

    fn item(level: u16, text: NorgASTFlat, content: Vec<NorgAST>) -> NorgAST {
        NorgAST::NestableDetachedModifier {
            modifier_type: NestableDetachedModifier::UnorderedList,
            level,
            extensions: Vec::new(),
            text: Box::new(text),
            content,
        }
    }

    fn render(node: &NorgAST) -> (String, Vec<String>) {
        let (html, diagnostics) = crate::diagnostics::capture(|| {
            let mut events = Vec::new();
            assert!(collect_list_items(node, &mut events));
            render_list_items(&events, &DocumentIds::default())
        });
        (html, diagnostics)
    }

    #[test]
    fn unexpected_block_in_list_content_is_skipped_not_fatal() {
        // Unreachable via rust-norg today; if that changes it must lose the
        // stray block, not abort the parse thread.
        let stray = NorgAST::Paragraph(vec![ParagraphSegment::Token(ParagraphSegmentToken::Text(
            "stray".into(),
        ))]);
        let (html, diagnostics) = render(&item(1, flat_paragraph("kept"), vec![stray]));

        assert_eq!(html, "<ul><li>kept</li></ul>");
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert!(
            diagnostics[0].contains("inside a list item"),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn non_paragraph_item_text_keeps_its_nested_items() {
        let nested = item(2, flat_paragraph("child"), Vec::new());
        let node = item(
            1,
            NorgASTFlat::DelimitingModifier(DelimitingModifier::Weak),
            vec![nested],
        );
        let (html, diagnostics) = render(&node);

        assert!(html.contains("child"), "{html}");
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert!(
            diagnostics[0].contains("not a paragraph"),
            "{diagnostics:?}"
        );
    }
}
