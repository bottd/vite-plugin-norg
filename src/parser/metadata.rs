use rust_norg::metadata::{parse_metadata as norg_parse_metadata, NorgMeta};
use rust_norg::NorgAST::VerbatimRangedTag;
use serde_json::{json, Map, Value};

pub fn extract_metadata(ast: &[rust_norg::NorgAST]) -> Value {
    ast.iter()
        .find_map(|node| match node {
            VerbatimRangedTag { name, content, .. }
                if name[0] == "document" && name[1] == "meta" =>
            {
                Some(content)
            }
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
            let json_map = map
                .into_iter()
                .map(|(key, value)| (key.clone(), norg_meta_to_json(value)))
                .collect::<Map<_, _>>();
            Value::Object(json_map)
        }
    }
}
