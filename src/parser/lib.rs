mod ast_handlers;
mod html;
mod metadata;
mod segments;
mod toc;
mod types;
mod utils;

pub use html::transform;
pub use metadata::{extract_meta_js, extract_metadata};
pub use toc::extract_toc;
pub use types::{ParsedNorg, TocEntry};
pub use utils::into_slug;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn parse_norg(content: &str) -> Result<JsValue, JsValue> {
    let ast = rust_norg::parse_tree(content).map_err(|e| format!("Parse error: {e:?}"))?;

    let html = transform(&ast);
    let toc = extract_toc(&ast);
    let meta = extract_meta_js(&ast)?;

    let result = js_sys::Object::new();

    js_sys::Reflect::set(&result, &"metadata".into(), &meta)
        .map_err(|_| "Set metadata failed".to_string())?;
    js_sys::Reflect::set(&result, &"html".into(), &html.into())
        .map_err(|_| "Set html failed".to_string())?;
    let toc =
        serde_wasm_bindgen::to_value(&toc).map_err(|e| format!("Serialize toc failed: {e}"))?;
    js_sys::Reflect::set(&result, &"toc".into(), &toc).map_err(|_| "Set toc failed".to_string())?;

    Ok(result.into())
}
