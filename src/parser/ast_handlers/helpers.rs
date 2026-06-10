use crate::segments::convert_segments;
use rust_norg::DelimitingModifier;

pub fn paragraph(segments: &[rust_norg::ParagraphSegment]) -> Option<String> {
    let content = convert_segments(segments);
    (!content.trim().is_empty()).then(|| format!("<p>{content}</p>"))
}

fn dotted(name: &[String]) -> String {
    if name.is_empty() {
        "<unnamed>".to_string()
    } else {
        name.join(".")
    }
}

/// Logs a skipped tag the renderer doesn't implement, naming its kind and the
/// dotted tag name (e.g. `image.gallery`) so the dropped content is traceable.
pub fn warn_unimplemented(kind: &str, name: &[String]) {
    eprintln!(
        "Warning: unimplemented {kind} tag '{}' — content skipped",
        dotted(name)
    );
}

/// Logs a carryover tag whose annotation the renderer doesn't implement; the
/// annotated object itself is still rendered.
pub fn warn_carryover_ignored(name: &[String]) {
    eprintln!(
        "Warning: unimplemented carryover tag '{}' — annotation ignored, content rendered",
        dotted(name)
    );
}

pub fn delimiter(delim: &DelimitingModifier) -> &'static str {
    match delim {
        DelimitingModifier::Weak => "<hr class=\"weak\" />",
        DelimitingModifier::Strong => "<hr class=\"strong\" />",
        DelimitingModifier::HorizontalRule => "<hr />",
    }
}
