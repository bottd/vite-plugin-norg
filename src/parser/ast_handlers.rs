use crate::segments::convert_segments;
use crate::types::{InlineComponent, OutputMode};
use crate::utils::into_slug;
use arborium::Highlighter;
use htmlescape::encode_minimal;
use itertools::Itertools;
use rust_norg::{DelimitingModifier, DetachedModifierExtension, NorgASTFlat, TodoStatus};
use textwrap::dedent;

#[derive(Debug)]
pub struct InlineParseError {
    pub index: usize,
    pub kind: InlineParseErrorKind,
}

#[derive(Debug)]
pub enum InlineParseErrorKind {
    MissingLanguage,
    InvalidLanguage { language: String },
    LanguageMismatch { language: String, mode: OutputMode },
}

impl std::fmt::Display for InlineParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let n = self.index + 1;
        let supported = OutputMode::ALL.map(|m| m.as_str()).join(", ");
        match &self.kind {
            InlineParseErrorKind::MissingLanguage => write!(
                f,
                "Inline error (inline #{n}): missing language. Supported languages: {supported}"
            ),
            InlineParseErrorKind::InvalidLanguage { language } => write!(
                f,
                "Inline error (inline #{n}): invalid language \"{language}\". Supported languages: {supported}"
            ),
            InlineParseErrorKind::LanguageMismatch { language, mode } => write!(
                f,
                "Inline error (inline #{n}): @inline {language} cannot be used in {mode} mode"
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
            (!content.trim().is_empty()).then(|| format_nestable(&content, extensions))
        }
        _ => None,
    }
}

pub fn verbatim_tag_with_embeds(
    name: &[String],
    parameters: &[String],
    content: &str,
    mode: Option<OutputMode>,
    highlighter: &mut Highlighter,
) -> Result<Option<VerbatimTagResult>, InlineParseErrorKind> {
    match name {
        [tag] if tag == "code" => {
            let code = dedent(content);
            let lang = parameters
                .first()
                .filter(|s| !s.is_empty())
                .map(String::as_str)
                .unwrap_or("text");

            let highlighted = highlighter.highlight(lang, &code);
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
            let inline_lang = parameters
                .first()
                .filter(|s| !s.is_empty())
                .map(String::as_str);

            match inline_lang {
                Some("css") => Ok(Some(VerbatimTagResult::css_only(content))),
                None => Err(InlineParseErrorKind::MissingLanguage),
                Some(lang) => {
                    let inline_mode = lang.parse::<OutputMode>().map_err(|_| {
                        InlineParseErrorKind::InvalidLanguage {
                            language: lang.to_string(),
                        }
                    })?;

                    match mode {
                        None => Ok(None),
                        Some(m) if m != inline_mode => {
                            Err(InlineParseErrorKind::LanguageMismatch {
                                language: lang.to_string(),
                                mode: m,
                            })
                        }
                        Some(_) => Ok(Some(VerbatimTagResult::inline_only(InlineComponent {
                            index: 0,
                            mode: inline_mode.to_string(),
                            code: content.to_string(),
                        }))),
                    }
                }
            }
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

fn format_nestable(content: &str, extensions: &[DetachedModifierExtension]) -> String {
    let mut classes: Vec<String> = Vec::new();
    let mut attrs: Vec<String> = Vec::new();
    let mut prefix: Vec<&str> = Vec::new();

    for extension in extensions {
        match extension {
            DetachedModifierExtension::Todo(status) => {
                if matches!(status, TodoStatus::Recurring(_)) {
                    classes.push("todo-recurring".into());
                }
                prefix.push(todo_html(status));
            }
            DetachedModifierExtension::Priority(priority) => {
                classes.push(format!("priority-{}", into_slug(priority)));
                attrs.push(format!(r#"data-priority="{}""#, encode_minimal(priority)));
            }
            DetachedModifierExtension::Timestamp(timestamp) => {
                attrs.push(format!(r#"data-timestamp="{}""#, encode_minimal(timestamp)));
            }
            DetachedModifierExtension::DueDate(date) => {
                attrs.push(format!(r#"data-due="{}""#, encode_minimal(date)));
            }
            DetachedModifierExtension::StartDate(date) => {
                attrs.push(format!(r#"data-start="{}""#, encode_minimal(date)));
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
    let prefix_html = if prefix.is_empty() {
        String::new()
    } else {
        format!("{} ", prefix.join(" "))
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

/// Wraps each of highlighted HTML in `<span class="line">`
/// This enables per-line styling such as line numbers or highlighting specific lines
fn wrap_lines(html: &str) -> String {
    html.lines()
        .map(|line| format!(r#"<span class="line">{line}</span>"#))
        .join("")
}
