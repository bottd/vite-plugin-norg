use crate::diagnostics;
use rust_norg::NorgAST::{self, VerbatimRangedTag};
use rust_norg::metadata::{NorgMeta, parse_metadata};
use serde_json::{Map, Value, json};

pub fn extract_metadata(ast: &[NorgAST]) -> Map<String, Value> {
    let Some(content) = ast.iter().find_map(|node| match node {
        VerbatimRangedTag { name, content, .. }
            if matches!(name.as_slice(), [doc, meta] if doc == "document" && meta == "meta") =>
        {
            Some(content.as_str())
        }
        _ => None,
    }) else {
        return Map::new();
    };

    match parse_metadata(&collapse_blank_lines(content)) {
        Ok(meta) => match meta_to_json(&meta) {
            Value::Object(map) => map,
            _ => Map::new(),
        },
        Err(err) => {
            diagnostics::warn(format!("ignored invalid @document.meta block: {err:?}"));
            Map::new()
        }
    }
}

/// rust-norg rejects two or more consecutive blank lines between keys,
/// while a single one is fine, so squash each run down to one.
fn collapse_blank_lines(content: &str) -> String {
    let mut previous_blank = false;
    content
        .lines()
        .filter(|line| {
            let blank = line.trim().is_empty();
            let keep = !(blank && previous_blank);
            previous_blank = blank;
            keep
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn meta_to_json(meta: &NorgMeta) -> Value {
    use NorgMeta::*;
    match meta {
        Invalid | Nil | EmptyKey(_) => Value::Null,
        Bool(b) => json!(b),
        Str(s) => json!(s),
        Num(n) => json!(n),
        Array(array) => json!(array.iter().map(meta_to_json).collect::<Vec<_>>()),
        Object(map) => json!(
            map.iter()
                .map(|(key, value)| (key.clone(), meta_to_json(value)))
                .collect::<Map<_, _>>()
        ),
    }
}
