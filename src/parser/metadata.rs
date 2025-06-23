use rust_norg::metadata::{parse_metadata as norg_parse_metadata, NorgMeta};
use rust_norg::NorgAST::VerbatimRangedTag;
use serde_json::{json, Map, Value};

pub fn extract_metadata(ast: &[rust_norg::NorgAST]) -> Value {
    ast.iter()
        .find_map(|node| match node {
            VerbatimRangedTag { name, content, .. } if is_document_meta(name) => Some(content),
            _ => None,
        })
        .and_then(|content| norg_parse_metadata(content).ok())
        .map_or_else(|| Value::Null, |meta| norg_meta_to_json(&meta))
}

fn norg_meta_to_json(meta: &NorgMeta) -> Value {
    match meta {
        NorgMeta::Invalid => Value::Null,
        NorgMeta::Nil => Value::Null,
        NorgMeta::Bool(b) => json!(b),
        NorgMeta::Str(s) => json!(s),
        NorgMeta::EmptyKey(_) => Value::Null,
        NorgMeta::Num(n) => json!(n),
        NorgMeta::Array(arr) => Value::Array(arr.iter().map(norg_meta_to_json).collect()),
        NorgMeta::Object(map) => {
            let mut json_map = Map::new();
            for (k, v) in map {
                json_map.insert(k.clone(), norg_meta_to_json(v));
            }
            Value::Object(json_map)
        }
    }
}

/// Check if the tag is a document meta tag
#[inline]
fn is_document_meta(name: &[String]) -> bool {
    println!("ranged tag name: {}", name.join(","));
    matches!(name, [doc, meta] if doc == "document" && meta == "meta")
}
