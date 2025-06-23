mod html;
mod metadata;
mod types;
mod utils;

pub use html::{convert_ast_to_html, convert_ast_to_html_with_toc};
pub use metadata::extract_metadata;
pub use types::{ParsedNorg, TocEntry};
pub use utils::into_slug;

use serde_json::Value;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn parse_norg(content: &str) -> Result<JsValue, JsValue> {
    let ast = rust_norg::parse_tree(content).map_err(|e| format!("Parse error: {:?}", e))?;

    let metadata = extract_metadata(&ast);

    let (html, toc) = convert_ast_to_html_with_toc(&ast);
    let ast_value = serde_json::to_value(&ast)
        .unwrap_or_else(|_| Value::String(format!("AST with {} nodes", ast.len())));

    let parsed = ParsedNorg {
        ast: ast_value,
        metadata,
        html,
        toc,
    };

    // Use JSON serialization to ensure metadata is a plain object, not a Map
    let json_str =
        serde_json::to_string(&parsed).map_err(|e| format!("JSON serialization error: {}", e))?;

    js_sys::JSON::parse(&json_str)
        .map_err(|e| JsValue::from_str(&format!("JSON parse error: {:?}", e)))
}
