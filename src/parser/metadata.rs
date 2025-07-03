use rust_norg::metadata::{parse_metadata, NorgMeta};
use rust_norg::NorgAST::VerbatimRangedTag;
use serde_json::{json, Map, Value};

pub fn extract_metadata(ast: &[rust_norg::NorgAST]) -> Value {
    ast.iter()
        .find_map(|node| match node {
            VerbatimRangedTag { name, content, .. }
                if matches!(name.as_slice(), [doc, meta] if doc == "document" && meta == "meta") => Some(content),
            _ => None,
        })
        .and_then(|content| parse_metadata(content).ok())
        .map(|meta| norg_meta_to_json(&meta))
        .unwrap_or(Value::Null)
}

fn norg_meta_to_json(meta: &NorgMeta) -> Value {
    match meta {
        NorgMeta::Invalid | NorgMeta::Nil | NorgMeta::EmptyKey(_) => Value::Null,
        NorgMeta::Bool(b) => json!(b),
        NorgMeta::Str(s) => json!(s),
        NorgMeta::Num(n) => json!(n),
        NorgMeta::Array(arr) => json!(arr.iter().map(norg_meta_to_json).collect::<Vec<Value>>()),
        NorgMeta::Object(map) => json!(map
            .iter()
            .map(|(k, v)| (k.clone(), norg_meta_to_json(v)))
            .collect::<Map<_, _>>()),
    }
}
