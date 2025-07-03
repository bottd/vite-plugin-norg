mod html;
mod metadata;
mod types;
mod utils;

pub use html::convert_nodes;
pub use metadata::{extract_metadata, extract_metadata_for_js};
pub use types::{ParsedNorg, TocEntry};
pub use utils::into_slug;

use wasm_bindgen::prelude::*;

/// Parses Norg content and returns a JavaScript object with metadata, HTML, and table of contents.
#[wasm_bindgen]
pub fn parse_norg(content: &str) -> Result<JsValue, JsValue> {
    let ast = rust_norg::parse_tree(content)
        .map_err(|e| JsValue::from_str(&format!("Parse error: {e:?}")))?;

    let (html, toc) = convert_nodes(&ast);
    let metadata = extract_metadata_for_js(&ast)
        .map_err(|e| JsValue::from_str(&format!("Metadata error: {e}")))?;

    let result = js_sys::Object::new();

    js_sys::Reflect::set(&result, &"metadata".into(), &metadata)
        .map_err(|_| JsValue::from_str("Failed to set metadata property"))?;
    js_sys::Reflect::set(&result, &"html".into(), &html.into())
        .map_err(|_| JsValue::from_str("Failed to set html property"))?;
    js_sys::Reflect::set(
        &result,
        &"toc".into(),
        &serde_wasm_bindgen::to_value(&toc)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize toc: {e}")))?,
    )
    .map_err(|_| JsValue::from_str("Failed to set toc property"))?;

    Ok(result.into())
}
