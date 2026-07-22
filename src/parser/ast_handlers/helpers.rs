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

pub fn carryover_target(mut node: &NorgAST) -> &NorgAST {
    while let NorgAST::CarryoverTag { next_object, .. } = node {
        node = next_object;
    }
    node
}

pub fn heading_level(node: &NorgAST) -> Option<u16> {
    match carryover_target(node) {
        NorgAST::Heading { level, .. } => Some(*level),
        _ => None,
    }
}

pub fn has_chained_carryover(mut node: &NorgAST) -> bool {
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

fn visit_visible_nodes<'a>(nodes: &'a [NorgAST], visit: &mut impl FnMut(&'a NorgAST)) {
    let mut index = 0;
    while index < nodes.len() {
        if let Some((kind, target)) = comment_target(&nodes[index]) {
            let Some(level) = heading_level(target) else {
                index += 1;
                continue;
            };

            if kind == CommentKind::Weak
                && let NorgAST::Heading { content, .. } = target
            {
                visit_nested_nodes(content, visit);
            }

            if !has_chained_carryover(&nodes[index]) {
                index += 1;
                continue;
            }

            index += 1;
            let mut current_level = level as i16;
            while index < nodes.len() {
                match heading_level(&nodes[index]) {
                    Some(next_level) if next_level <= level => break,
                    Some(next_level) => {
                        current_level = next_level as i16;
                        if kind == CommentKind::Weak {
                            visit_visible_nodes(std::slice::from_ref(&nodes[index]), visit);
                        }
                    }
                    None if matches!(
                        &nodes[index],
                        NorgAST::DelimitingModifier(delim)
                            if delimiter_exits_heading_scope(
                                delim,
                                level,
                                &mut current_level,
                            )
                    ) =>
                    {
                        break;
                    }
                    None => {}
                }
                index += 1;
            }
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

fn visit_nested_nodes<'a>(nodes: &'a [NorgAST], visit: &mut impl FnMut(&'a NorgAST)) {
    for node in nodes {
        if heading_level(node).is_some() {
            visit_visible_nodes(std::slice::from_ref(node), visit);
        }
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
