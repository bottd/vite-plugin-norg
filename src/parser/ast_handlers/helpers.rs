use crate::segments::{DocumentIds, convert_segments_with_ids, title_key, title_slug};
use rust_norg::{
    CarryoverTag as CarryoverKind, DelimitingModifier, NorgAST, NorgASTFlat, ParagraphSegment,
    RangeableDetachedModifier,
};

pub fn paragraph(segments: &[rust_norg::ParagraphSegment], ids: &DocumentIds) -> Option<String> {
    let content = convert_segments_with_ids(segments, ids);
    (!content.trim().is_empty()).then(|| format!("<p>{content}</p>"))
}

fn dotted(name: &[String]) -> String {
    if name.is_empty() {
        "<unnamed>".to_string()
    } else {
        name.join(".")
    }
}

pub fn is_comment_tag(name: &[String]) -> bool {
    matches!(name, [name] if name == "comment")
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CommentKind {
    Weak,
    Strong,
}

pub fn comment_target(mut node: &NorgAST) -> Option<(CommentKind, &NorgAST)> {
    let mut comment = None;
    while let NorgAST::CarryoverTag {
        tag_type,
        name,
        next_object,
        ..
    } = node
    {
        if is_comment_tag(name) {
            let kind = if matches!(tag_type, CarryoverKind::Macro) {
                CommentKind::Strong
            } else {
                CommentKind::Weak
            };
            if comment != Some(CommentKind::Strong) {
                comment = Some(kind);
            }
        }
        node = next_object;
    }
    comment.map(|kind| (kind, node))
}

fn carryover_target(mut node: &NorgAST) -> &NorgAST {
    while let NorgAST::CarryoverTag { next_object, .. } = node {
        node = next_object;
    }
    node
}

fn heading_level(node: &NorgAST) -> Option<u16> {
    match carryover_target(node) {
        NorgAST::Heading { level, .. } => Some(*level),
        _ => None,
    }
}

fn has_chained_carryover(mut node: &NorgAST) -> bool {
    let mut seen = false;
    while let NorgAST::CarryoverTag { next_object, .. } = node {
        if seen {
            return true;
        }
        seen = true;
        node = next_object;
    }
    false
}

pub fn flat_comment_target(mut node: &NorgASTFlat) -> Option<(CommentKind, &NorgASTFlat)> {
    let mut comment = None;
    while let NorgASTFlat::CarryoverTag {
        tag_type,
        name,
        next_object,
        ..
    } = node
    {
        if is_comment_tag(name) {
            let kind = if matches!(tag_type, CarryoverKind::Macro) {
                CommentKind::Strong
            } else {
                CommentKind::Weak
            };
            if comment != Some(CommentKind::Strong) {
                comment = Some(kind);
            }
        }
        node = next_object;
    }
    comment.map(|kind| (kind, node))
}

pub fn flat_carryover_target(mut node: &NorgASTFlat) -> &NorgASTFlat {
    while let NorgASTFlat::CarryoverTag { next_object, .. } = node {
        node = next_object;
    }
    node
}

pub fn flat_heading_level(node: &NorgASTFlat) -> Option<u16> {
    match flat_carryover_target(node) {
        NorgASTFlat::Heading { level, .. } => Some(*level),
        _ => None,
    }
}

pub fn document_ids(ast: &[NorgAST]) -> DocumentIds {
    let mut headings = Vec::new();
    let mut footnotes = Vec::new();
    visit_visible_nodes(ast, &mut |node| match node {
        NorgAST::Heading { level, title, .. } => {
            headings.push((*level, title_key(title), title_slug(title)));
        }
        NorgAST::RangeableDetachedModifier {
            modifier_type: RangeableDetachedModifier::Footnote,
            title,
            ..
        } => footnotes.push((title_key(title), title_slug(title))),
        _ => {}
    });
    DocumentIds::new(headings, footnotes)
}

pub fn visit_visible_headings<'a>(
    nodes: &'a [NorgAST],
    visit: &mut impl FnMut(u16, &'a [ParagraphSegment]),
) {
    visit_visible_nodes(nodes, &mut |node| {
        if let NorgAST::Heading { level, title, .. } = node {
            visit(*level, title);
        }
    });
}

/// How far a comment reaches, and what it leaves behind.
pub struct CommentScope<'a> {
    /// Index of the first node the comment does not cover.
    pub end: usize,
    /// Headings that stay visible within the commented range, in document
    /// order. Always empty for a strong comment, which hides its whole scope.
    pub visible: Vec<&'a NorgAST>,
}

/// The extent of the comment on `nodes[index]`, or `None` if it isn't
/// commented. A comment on a non-heading hides only that node; on a heading
/// with a chained carryover it hides the whole scope, down to a heading of the
/// same or shallower level or a delimiter that closes it. `+comment` hides only
/// the heading text, leaving nested headings visible; `#comment` hides those
/// too.
///
/// The renderer and the id pre-pass both walk through here, and must agree on
/// what is visible — see [`DocumentIds::unconsumed`].
pub fn comment_scope<'a>(nodes: &'a [NorgAST], index: usize) -> Option<CommentScope<'a>> {
    let (kind, target) = comment_target(&nodes[index])?;
    let mut visible = Vec::new();

    // `comment_target` unwrapped the carryovers, so this is the annotated node.
    let NorgAST::Heading { level, content, .. } = target else {
        return Some(CommentScope {
            end: index + 1,
            visible,
        });
    };
    let level = *level;

    if kind == CommentKind::Weak {
        visible.extend(content.iter().filter(|node| heading_level(node).is_some()));
    }

    if !has_chained_carryover(&nodes[index]) {
        return Some(CommentScope {
            end: index + 1,
            visible,
        });
    }

    let mut end = index + 1;
    let mut current_level = level as i16;
    while end < nodes.len() {
        match heading_level(&nodes[end]) {
            Some(next_level) if next_level <= level => break,
            Some(next_level) => {
                current_level = next_level as i16;
                if kind == CommentKind::Weak {
                    visible.push(&nodes[end]);
                }
            }
            None if matches!(
                &nodes[end],
                NorgAST::DelimitingModifier(delim)
                    if delimiter_exits_heading_scope(delim, level, &mut current_level)
            ) =>
            {
                break;
            }
            None => {}
        }
        end += 1;
    }
    Some(CommentScope { end, visible })
}

fn visit_visible_nodes<'a>(nodes: &'a [NorgAST], visit: &mut impl FnMut(&'a NorgAST)) {
    let mut index = 0;
    while index < nodes.len() {
        if let Some(scope) = comment_scope(nodes, index) {
            for node in scope.visible {
                visit_visible_nodes(std::slice::from_ref(node), visit);
            }
            index = scope.end;
            continue;
        }

        match &nodes[index] {
            NorgAST::Heading { content, .. } => {
                visit(&nodes[index]);
                visit_visible_nodes(content, visit);
            }
            NorgAST::CarryoverTag { next_object, .. } => {
                visit_visible_nodes(std::slice::from_ref(next_object), visit);
            }
            _ => visit(&nodes[index]),
        }
        index += 1;
    }
}

/// Records a skipped tag the renderer doesn't implement, naming its kind and
/// the dotted tag name (e.g. `image.gallery`) so the dropped content is traceable.
pub fn warn_unimplemented(kind: &str, name: &[String]) {
    crate::diagnostics::warn(format!(
        "unimplemented {kind} tag '{}' — content skipped",
        dotted(name)
    ));
}

/// Records a carryover tag whose annotation the renderer doesn't implement; the
/// annotated object itself is still rendered.
pub fn warn_carryover_ignored(name: &[String]) {
    crate::diagnostics::warn(format!(
        "unimplemented carryover tag '{}' — annotation ignored, content rendered",
        dotted(name)
    ));
}

pub fn delimiter_exits_heading_scope(
    delim: &DelimitingModifier,
    start_level: u16,
    current_level: &mut i16,
) -> bool {
    match delim {
        DelimitingModifier::Strong => true,
        DelimitingModifier::Weak => {
            *current_level -= 1;
            *current_level < start_level as i16
        }
        DelimitingModifier::HorizontalRule => false,
    }
}

pub fn delimiter(delim: &DelimitingModifier) -> &'static str {
    match delim {
        DelimitingModifier::Weak => "<hr class=\"weak\" />",
        DelimitingModifier::Strong => "<hr class=\"strong\" />",
        DelimitingModifier::HorizontalRule => "<hr />",
    }
}
