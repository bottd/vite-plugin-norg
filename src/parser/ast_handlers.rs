use crate::segments::convert_segments;
use crate::types::InlineComponent;
use crate::utils::into_slug;
use arborium::Highlighter;
use htmlescape::encode_minimal;
use itertools::Itertools;
use rust_norg::{DelimitingModifier, DetachedModifierExtension, NorgASTFlat, TodoStatus};
use textwrap::dedent;

const INLINE_FRAMEWORKS: &[&str] = &["html", "svelte", "vue"];

#[derive(Debug)]
pub struct InlineParseError {
    pub index: usize,
    pub kind: InlineParseErrorKind,
}

#[derive(Debug)]
pub enum InlineParseErrorKind {
    MissingFramework,
    InvalidFramework { framework: String },
    FrameworkMismatch { framework: String, target: String },
}

impl std::fmt::Display for InlineParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let n = self.index + 1;
        let supported = INLINE_FRAMEWORKS.join(", ");
        match &self.kind {
            InlineParseErrorKind::MissingFramework => write!(
                f,
                "Inline error (inline #{n}): missing framework. Supported frameworks: {supported}"
            ),
            InlineParseErrorKind::InvalidFramework { framework } => write!(
                f,
                "Inline error (inline #{n}): invalid framework \"{framework}\". Supported frameworks: {supported}"
            ),
            InlineParseErrorKind::FrameworkMismatch { framework, target } => write!(
                f,
                "Inline error (inline #{n}): @inline {framework} cannot be used in a {target} project"
            ),
        }
    }
}

pub struct VerbatimTagResult {
    pub html: Option<String>,
    pub inline: Option<InlineComponent>,
    pub css: Option<String>,
}

impl VerbatimTagResult {
    fn html_only(html: impl Into<String>) -> Self {
        Self {
            html: Some(html.into()),
            inline: None,
            css: None,
        }
    }

    fn inline_only(inline: InlineComponent) -> Self {
        Self {
            html: None,
            inline: Some(inline),
            css: None,
        }
    }

    fn css_only(css: impl Into<String>) -> Self {
        Self {
            html: None,
            inline: None,
            css: Some(css.into()),
        }
    }
}

pub fn nestable_modifier(
    text: &NorgASTFlat,
    extensions: &[DetachedModifierExtension],
) -> Option<String> {
    match text {
        NorgASTFlat::Paragraph(segments) => {
            let content = convert_segments(segments);
            (!content.trim().is_empty()).then(|| format_list_item(&content, extensions))
        }
        _ => None,
    }
}

pub fn verbatim_tag_with_embeds(
    name: &[String],
    parameters: &[String],
    content: &str,
    target_framework: Option<&str>,
) -> Result<Option<VerbatimTagResult>, InlineParseErrorKind> {
    match name {
        [tag] if tag == "code" => {
            let code = dedent(content);
            let lang = parameters
                .first()
                .filter(|s| !s.is_empty())
                .map(String::as_str)
                .unwrap_or("text");

            let highlighted = Highlighter::new().highlight(lang, &code);
            let html = match highlighted {
                Ok(h) => format!(
                    r#"<pre class="arborium lang-{lang}"><code>{}</code></pre>"#,
                    wrap_lines(&h)
                ),
                Err(_) => format!(
                    r#"<pre><code>{}</code></pre>"#,
                    wrap_lines(&encode_minimal(&code))
                ),
            };
            Ok(Some(VerbatimTagResult::html_only(html)))
        }
        [tag] if tag == "image" => Ok(parameters.first().filter(|s| !s.is_empty()).map(|path| {
            let src = if path.starts_with('/') || path.starts_with("http") {
                path.clone()
            } else {
                format!("./{path}")
            };
            VerbatimTagResult::html_only(format!(
                r#"<img src="{}" alt="{}" />"#,
                encode_minimal(&src),
                encode_minimal(content.trim())
            ))
        })),
        [tag] if tag == "inline" => {
            let framework = parameters
                .first()
                .filter(|s| !s.is_empty())
                .map(String::as_str)
                .or(target_framework)
                .unwrap_or("");

            // CSS is not a framework — return early without entering framework validation
            if framework == "css" {
                return Ok(Some(VerbatimTagResult::css_only(content)));
            }

            if framework.is_empty() {
                return Err(InlineParseErrorKind::MissingFramework);
            }

            if !INLINE_FRAMEWORKS.contains(&framework) {
                return Err(InlineParseErrorKind::InvalidFramework {
                    framework: framework.to_string(),
                });
            }

            // Validate that the inline framework matches the target framework
            if let Some(target) = target_framework {
                if framework != target {
                    return Err(InlineParseErrorKind::FrameworkMismatch {
                        framework: framework.to_string(),
                        target: target.to_string(),
                    });
                }
            }
            Ok(Some(VerbatimTagResult::inline_only(InlineComponent {
                index: 0, // Set by caller
                framework: framework.to_string(),
                code: content.to_string(),
            })))
        }
        [doc, meta] if doc == "document" && meta == "meta" => Ok(None),
        _ => Ok(Some(VerbatimTagResult::html_only(format!(
            r#"<div class="verbatim">{}</div>"#,
            encode_minimal(content)
        )))),
    }
}

pub fn paragraph(segments: &[rust_norg::ParagraphSegment]) -> Option<String> {
    let content = convert_segments(segments);
    (!content.trim().is_empty()).then(|| format!("<p>{content}</p>"))
}

pub fn delimiter(delim: &DelimitingModifier) -> &'static str {
    match delim {
        DelimitingModifier::Weak => "<hr class=\"weak\" />",
        DelimitingModifier::Strong => "<hr class=\"strong\" />",
        DelimitingModifier::HorizontalRule => "<hr />",
    }
}

fn format_list_item(content: &str, extensions: &[DetachedModifierExtension]) -> String {
    let mut classes: Vec<String> = Vec::new();
    let mut attrs: Vec<String> = Vec::new();
    let mut prefixes: Vec<&str> = Vec::new();

    for ext in extensions {
        match ext {
            DetachedModifierExtension::Todo(status) => {
                if matches!(status, TodoStatus::Recurring(_)) {
                    classes.push("todo-recurring".into());
                }
                prefixes.push(todo_html(status));
            }
            DetachedModifierExtension::Priority(p) => {
                classes.push(format!("priority-{}", into_slug(p)));
                attrs.push(format!(r#"data-priority="{}""#, encode_minimal(p)));
            }
            DetachedModifierExtension::Timestamp(ts) => {
                attrs.push(format!(r#"data-timestamp="{}""#, encode_minimal(ts)));
            }
            DetachedModifierExtension::DueDate(d) => {
                attrs.push(format!(r#"data-due="{}""#, encode_minimal(d)));
            }
            DetachedModifierExtension::StartDate(d) => {
                attrs.push(format!(r#"data-start="{}""#, encode_minimal(d)));
            }
        }
    }

    let class_attr = if classes.is_empty() {
        String::new()
    } else {
        format!(r#" class="{}""#, classes.join(" "))
    };
    let data_attrs = if attrs.is_empty() {
        String::new()
    } else {
        format!(" {}", attrs.join(" "))
    };
    let prefix_html = if prefixes.is_empty() {
        String::new()
    } else {
        format!("{} ", prefixes.join(" "))
    };

    format!("<li{class_attr}{data_attrs}>{prefix_html}{content}</li>")
}

fn todo_html(status: &TodoStatus) -> &'static str {
    match status {
        TodoStatus::Undone => {
            r#"<input type="checkbox" class="todo-status todo-undone" disabled />"#
        }
        TodoStatus::Done => {
            r#"<input type="checkbox" class="todo-status todo-done" checked disabled />"#
        }
        TodoStatus::NeedsClarification => {
            r#"<span class="todo-status todo-clarification">?</span>"#
        }
        TodoStatus::Paused => r#"<span class="todo-status todo-paused">=</span>"#,
        TodoStatus::Urgent => r#"<span class="todo-status todo-urgent">!</span>"#,
        TodoStatus::Pending => r#"<span class="todo-status todo-pending">-</span>"#,
        TodoStatus::Canceled => r#"<span class="todo-status todo-canceled">_</span>"#,
        TodoStatus::Recurring(_) => r#"<span class="todo-status todo-recurring">+</span>"#,
    }
}

fn wrap_lines(html: &str) -> String {
    html.lines()
        .map(|line| format!(r#"<span class="line">{line}</span>"#))
        .join("")
}
