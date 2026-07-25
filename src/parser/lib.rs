mod ast_handlers;
mod diagnostics;
mod html;
mod metadata;
mod segments;
mod toc;
mod types;
mod utils;

pub use html::transform;
pub use metadata::extract_metadata;
pub use toc::extract_toc;
pub use types::{EmbedComponent, OutputMode, TocEntry};
pub use utils::into_slug;

use arborium::theme::builtin;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde_json::{Map, Value};

#[cfg(not(target_arch = "wasm32"))]
const PARSER_STACK_SIZE: usize = 32 * 1024 * 1024;

#[napi(object)]
pub struct NorgParseResult {
    pub metadata: Map<String, Value>,
    pub html_parts: Vec<String>,
    pub toc: Vec<TocEntry>,
    pub embed_components: Vec<EmbedComponent>,
    pub embed_css: String,
    /// Non-fatal warnings from rendering (skipped/altered content), for the
    /// host to surface — stderr is invisible in a Vite worker.
    pub diagnostics: Option<Vec<String>>,
}

#[napi]
pub fn parse_norg(content: String, mode: Option<String>) -> Result<NorgParseResult> {
    #[cfg(target_arch = "wasm32")]
    {
        parse_norg_inner(&content, mode.as_deref()).map_err(Error::from_reason)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        // rust-norg recursively builds nested documents. A dedicated, bounded
        // stack prevents ordinary deep input from aborting the Node process.
        let handle = std::thread::Builder::new()
            .name("norg-parse".into())
            .stack_size(PARSER_STACK_SIZE)
            .spawn(move || parse_norg_inner(&content, mode.as_deref()))
            .map_err(|e| Error::from_reason(format!("Failed to spawn parser thread: {e}")))?;

        match handle.join() {
            Ok(result) => result.map_err(Error::from_reason),
            Err(_) => Err(Error::from_reason("Parser thread panicked")),
        }
    }
}

fn parse_norg_inner(
    content: &str,
    mode: Option<&str>,
) -> std::result::Result<NorgParseResult, String> {
    let ast = rust_norg::parse_tree(content).map_err(|e| format!("Parse error: {e:?}"))?;

    let output_mode = mode.and_then(|s| s.parse().ok());
    let (rendered, diagnostics) = diagnostics::capture(|| transform(&ast, output_mode));
    let toc = diagnostics::discard(|| extract_toc(&ast));
    let (html_parts, embed_components, embed_css) =
        rendered.map_err(|err| format_embed_error(&err))?;
    let metadata = extract_metadata(&ast);

    Ok(NorgParseResult {
        metadata,
        html_parts,
        toc,
        embed_components,
        embed_css,
        diagnostics: Some(diagnostics),
    })
}

fn format_embed_error(err: &crate::ast_handlers::EmbedParseError) -> String {
    format!("{err}. Offending line: {}", err.offending_line())
}

#[napi]
pub fn get_theme_css(theme: String) -> String {
    builtin::all()
        .into_iter()
        .find(|t| t.name == theme)
        .map(|t| t.to_css("pre.arborium"))
        .unwrap_or_default()
}
