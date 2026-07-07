use crate::ast_handlers::warn_carryover_ignored;
use crate::segments::convert_segments;
use crate::utils::into_slug;
use htmlescape::encode_minimal;
use rust_norg::{
    DetachedModifierExtension, NestableDetachedModifier, NorgAST, NorgASTFlat, TodoStatus,
};
use std::fmt::Write;

/// One list item flattened out of the AST's `List`/`NestableDetachedModifier`
/// nodes. rust-norg only nests same-marker items into an item's `content`;
/// deeper items of a *different* marker type end up as a sibling `List` node,
/// so the renderer re-nests flattened runs by `level`.
pub struct FlatListItem<'a> {
    kind: &'a NestableDetachedModifier,
    level: u16,
    text: &'a NorgASTFlat,
    extensions: &'a [DetachedModifierExtension],
    content: &'a [NorgAST],
}

/// Flattens a list-like node (`List`, a bare `NestableDetachedModifier`, or a
/// `CarryoverTag` wrapping either) into `items`. Returns false — consuming
/// nothing — for any other node.
pub fn collect_list_items<'a>(node: &'a NorgAST, items: &mut Vec<FlatListItem<'a>>) -> bool {
    match node {
        NorgAST::List {
            items: list_items, ..
        } => {
            for item in list_items {
                if !collect_list_items(item, items) {
                    crate::diagnostics::warn("unsupported node inside list — content skipped");
                }
            }
            true
        }
        NorgAST::NestableDetachedModifier {
            modifier_type,
            level,
            extensions,
            text,
            content,
        } => {
            items.push(FlatListItem {
                kind: modifier_type,
                level: *level,
                text,
                extensions,
                content,
            });
            true
        }
        NorgAST::CarryoverTag {
            name, next_object, ..
        } => {
            // The annotation itself is unimplemented; recurse into the object
            // it wraps. If that object is list content, keep it in the run and
            // warn the tag was ignored; otherwise this isn't part of the list.
            // (`collect_list_items` pushes nothing when it returns false, so
            // the speculative call is safe.)
            let consumed = collect_list_items(next_object, items);
            if consumed {
                warn_carryover_ignored(name);
            }
            consumed
        }
        _ => false,
    }
}

pub fn render_list_items(items: &[FlatListItem]) -> String {
    let mut out = String::new();
    render_into(items, &mut out, true);
    out
}

struct OpenList {
    tag: &'static str,
    level: u16,
    /// An `<li>` is left open so deeper sibling containers can nest inside
    /// it; quote items (`<p>`) close immediately.
    item_open: bool,
}

fn render_into(items: &[FlatListItem], out: &mut String, top_level: bool) {
    // No render-side depth cap: recursion here can't exceed the AST's list
    // nesting, which `parse_tree` already recursed through on the same
    // large-stack thread (see `parse_norg`). `top_level` only distinguishes the
    // outermost run (which starts a sibling list on its own line) from nested
    // runs (which open inside a parent's still-open item).
    let mut stack: Vec<OpenList> = Vec::new();

    for item in items {
        let tag = container_tag(item.kind);

        let mut nested = Vec::new();
        for node in item.content {
            if !collect_list_items(node, &mut nested) {
                // The pure list renderer can't place non-list content inside a
                // list item. rust_norg never actually puts such a node here, so
                // this is defensive: warn and skip rather than fail the whole
                // document, matching how every other unsupported node is handled.
                crate::diagnostics::warn("unsupported node inside a list item — content skipped");
            }
        }
        let mut children = String::new();
        render_into(&nested, &mut children, false);

        let Some((markup, leaves_item_open)) = item_markup(item, &children) else {
            continue;
        };

        // Close containers this item doesn't belong to: anything deeper, or a
        // same-level container of a different kind.
        while let Some(top) = stack.last() {
            if top.level > item.level || (top.level == item.level && top.tag != tag) {
                close_list(out, stack.pop().unwrap());
            } else {
                break;
            }
        }
        match stack.last_mut() {
            Some(top) if top.level == item.level => {
                if top.item_open {
                    out.push_str("</li>");
                }
            }
            _ => {
                // A deeper container opens inside the parent's still-open
                // item; a top-level sibling starts on its own line.
                if top_level && stack.is_empty() && !out.is_empty() {
                    out.push('\n');
                }
                let _ = write!(out, "<{tag}>");
                stack.push(OpenList {
                    tag,
                    level: item.level,
                    item_open: false,
                });
            }
        }
        out.push_str(&markup);
        if let Some(top) = stack.last_mut() {
            top.item_open = leaves_item_open;
        }
    }

    while let Some(open) = stack.pop() {
        close_list(out, open);
    }
}

fn close_list(out: &mut String, open: OpenList) {
    if open.item_open {
        out.push_str("</li>");
    }
    let _ = write!(out, "</{}>", open.tag);
}

fn container_tag(kind: &NestableDetachedModifier) -> &'static str {
    match kind {
        NestableDetachedModifier::UnorderedList => "ul",
        NestableDetachedModifier::OrderedList => "ol",
        NestableDetachedModifier::Quote => "blockquote",
    }
}

/// Renders one item's own markup, or `None` when the item renders to nothing.
/// The bool says whether an `<li>` was left open for nested content; quote
/// items render as a closed `<p>` (an `<li>` is invalid outside `ul`/`ol`,
/// and `<p>` cannot contain the nested `<blockquote>`s that follow it).
fn item_markup(item: &FlatListItem, children: &str) -> Option<(String, bool)> {
    let content = match item.text {
        NorgASTFlat::Paragraph(segments) => convert_segments(segments),
        _ => {
            crate::diagnostics::warn("unsupported text node in list item — skipped");
            String::new()
        }
    };
    let blank = content.trim().is_empty();
    if blank && children.is_empty() && item.extensions.is_empty() {
        return None;
    }

    let (class_attr, attrs, prefix) = extension_markup(item.extensions);
    let separator = if prefix.is_empty() || blank { "" } else { " " };

    Some(match item.kind {
        NestableDetachedModifier::Quote => {
            let mut s = String::new();
            if !blank || !prefix.is_empty() || !class_attr.is_empty() || !attrs.is_empty() {
                let _ = write!(s, "<p{class_attr}{attrs}>{prefix}{separator}{content}</p>");
            }
            s.push_str(children);
            (s, false)
        }
        _ => (
            format!("<li{class_attr}{attrs}>{prefix}{separator}{content}{children}"),
            true,
        ),
    })
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
