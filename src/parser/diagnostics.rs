//! Per-parse diagnostics sink.
//!
//! The renderer warns whenever it skips or alters author content (an
//! unimplemented tag, a dropped unsafe link, a node it can't place). Writing
//! those to stderr makes them invisible in a Vite worker and in CI, so instead
//! they are collected here and returned in `NorgParseResult` for the host to
//! surface (HMR overlay, `this.warn`, …).
//!
//! `parse_norg` runs each parse on its own dedicated thread (for stack depth),
//! so a thread-local sink is naturally scoped to a single parse with no locking
//! and no cross-parse bleed — which is why a shared collector isn't threaded
//! through every renderer signature (including the state-free `convert_segments`
//! and `extract_toc`).

use std::cell::RefCell;

thread_local! {
    static SINK: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

/// Records a warning about content the renderer skipped or changed. Only
/// observed when called on the parse thread (where `take` later drains it); a
/// call from any other thread accumulates on that thread and is never surfaced.
pub fn warn(message: impl Into<String>) {
    SINK.with(|sink| sink.borrow_mut().push(message.into()));
}

/// Drains and returns everything recorded on this thread since the last drain,
/// with exact duplicates collapsed (order preserved). A heading title is
/// converted twice per parse — once by the renderer, once by TOC extraction —
/// so a warning about it (dropped unsafe link, unsupported segment) would
/// otherwise surface twice; these are advisory, so reporting each once is enough.
pub fn take() -> Vec<String> {
    SINK.with(|sink| {
        let mut seen = std::collections::HashSet::new();
        std::mem::take(&mut *sink.borrow_mut())
            .into_iter()
            .filter(|message| seen.insert(message.clone()))
            .collect()
    })
}
