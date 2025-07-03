use rust_norg::metadata::{parse_metadata, NorgMeta};
use rust_norg::NorgAST::{self, VerbatimRangedTag};
use serde_json::{json, Map, Value};
use wasm_bindgen::prelude::*;

/// Extracts metadata from a Norg AST as JSON.
/// Returns `Value::Null` if no metadata is found.
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
        .map(|meta| norg_meta_to_json(&meta))
        .unwrap_or(Value::Null)
}

pub fn extract_metadata_for_js(ast: &[NorgAST]) -> Result<JsValue, String> {
    let metadata = extract_metadata(ast);
    let serializable_metadata = if metadata.is_null() {
        json!({})
    } else {
        metadata
    };

    let json_string = serde_json::to_string(&serializable_metadata)
        .map_err(|e| format!("Metadata serialization failed: {e}"))?;

    js_sys::JSON::parse(&json_string)
        .map_err(|_| "Failed to parse metadata JSON in JavaScript".to_string())
}

fn norg_meta_to_json(meta: &NorgMeta) -> Value {
    use NorgMeta::*;
    match meta {
        Invalid | Nil | EmptyKey(_) => Value::Null,
        Bool(b) => json!(b),
        Str(s) => json!(s),
        Num(n) => json!(n),
        Array(arr) => json!(arr.iter().map(norg_meta_to_json).collect::<Vec<_>>()),
        Object(map) => json!(map
            .iter()
            .map(|(key, value)| (key.clone(), norg_meta_to_json(value)))
            .collect::<Map<_, _>>()),
    }
}
