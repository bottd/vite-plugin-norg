use rust_norg::NorgAST::{self, VerbatimRangedTag};
use rust_norg::metadata::{NorgMeta, parse_metadata};
use serde_json::{Map, Value, json};

pub fn extract_metadata(ast: &[NorgAST]) -> Value {
    ast.iter()
        .find_map(|node| match node {
            VerbatimRangedTag { name, content, .. }
                if matches!(name.as_slice(), [doc, meta] if doc == "document" && meta == "meta") => {
                Some(content.as_str())
            }
            _ => None,
        })
        .and_then(|content| parse_metadata(content).ok())
        .map(|meta| meta_to_json(&meta))
        .unwrap_or(Value::Null)
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
