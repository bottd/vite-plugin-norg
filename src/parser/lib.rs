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
pub use types::{EmbedComponent, NorgErrorCode, OutputMode, TocEntry};
pub use utils::into_slug;

use arborium::theme::builtin;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde_json::{Map, Value};

#[cfg(not(target_arch = "wasm32"))]
const PARSER_STACK_SIZE: usize = 32 * 1024 * 1024;

#[napi(object)]
pub struct NorgParseResult {
    #[napi(ts_type = "Record<string, unknown>")]
    pub metadata: Map<String, Value>,
    pub html_parts: Vec<String>,
    pub toc: Vec<TocEntry>,
    pub embed_components: Vec<EmbedComponent>,
    pub embed_css: String,
    /// Non-fatal warnings from rendering (skipped/altered content), for the
    /// host to surface — stderr is invisible in a Vite worker.
    pub diagnostics: Vec<String>,
}

/// Parses off the JS thread (libuv pool) so Vite can parse files concurrently.
pub struct ParseTask {
    content: String,
    mode: Option<OutputMode>,
}

pub type ParseResult = std::result::Result<NorgParseResult, Error<NorgErrorCode>>;

#[napi]
impl Task for ParseTask {
    type Output = ParseResult;
    type JsValue = NorgParseResult;

    fn compute(&mut self) -> Result<Self::Output> {
        Ok(parse_norg(std::mem::take(&mut self.content), self.mode))
    }

    fn resolve(&mut self, env: Env, output: Self::Output) -> Result<Self::JsValue> {
        output.map_err(|error| JsError::from(error).into_unknown(env).into())
    }
}

#[napi(js_name = "parseNorg")]
pub fn parse_norg_async(
    content: String,
    #[napi(ts_arg_type = "`${OutputMode}` | null")] mode: Option<OutputMode>,
) -> AsyncTask<ParseTask> {
    AsyncTask::new(ParseTask { content, mode })
}

/// Blocking parse; the JS binding is [`parse_norg_async`].
pub fn parse_norg(content: String, mode: Option<OutputMode>) -> ParseResult {
    #[cfg(target_arch = "wasm32")]
    {
        parse_norg_inner(&content, mode)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        // rust-norg recursively builds nested documents. A dedicated, bounded
        // stack prevents ordinary deep input from aborting the Node process.
        let handle = std::thread::Builder::new()
            .name("norg-parse".into())
            .stack_size(PARSER_STACK_SIZE)
            .spawn(move || parse_norg_inner(&content, mode))
            .map_err(|e| {
                NorgErrorCode::Internal.error(format!("Failed to spawn parser thread: {e}"))
            })?;

        handle
            .join()
            .unwrap_or_else(|_| Err(NorgErrorCode::Internal.error("Parser thread panicked")))
    }
}

fn parse_norg_inner(content: &str, mode: Option<OutputMode>) -> ParseResult {
    let ast = rust_norg::parse_tree(content)
        .map_err(|e| NorgErrorCode::Parse.error(format!("Parse error: {e:?}")))?;

    let ((rendered, metadata), diagnostics) =
        diagnostics::capture(|| (transform(&ast, mode), extract_metadata(&ast)));
    let toc = diagnostics::discard(|| extract_toc(&ast));
    let (html_parts, embed_components, embed_css) =
        rendered.map_err(|err| NorgErrorCode::Embed.error(format_embed_error(&err)))?;

    Ok(NorgParseResult {
        metadata,
        html_parts,
        toc,
        embed_components,
        embed_css,
        diagnostics,
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
