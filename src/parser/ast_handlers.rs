use crate::segments::convert_segments;
use crate::types::{InlineComponent, OutputMode};
use crate::utils::into_slug;
use arborium::Highlighter;
use htmlescape::encode_minimal;
use itertools::Itertools;
use rust_norg::{DelimitingModifier, DetachedModifierExtension, NorgASTFlat, TodoStatus};
use textwrap::dedent;

#[derive(Debug)]
pub enum InlineParseError {
    MissingLanguage {
        index: usize,
    },
    InvalidLanguage {
        index: usize,
        language: String,
    },
    LanguageMismatch {
        index: usize,
        language: String,
        mode: OutputMode,
    },
}

impl InlineParseError {
    pub fn index(&self) -> usize {
        match self {
            Self::MissingLanguage { index }
            | Self::InvalidLanguage { index, .. }
            | Self::LanguageMismatch { index, .. } => *index,
        }
    }
}

impl std::fmt::Display for InlineParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let n = self.index() + 1;
        let supported = OutputMode::ALL.map(|m| m.as_str()).join(", ");
        match self {
            Self::MissingLanguage { .. } => write!(
                f,
                "Inline error (inline #{n}): missing language. Supported languages: {supported}"
            ),
            Self::InvalidLanguage { language, .. } => write!(
                f,
                "Inline error (inline #{n}): invalid language \"{language}\". Supported languages: {supported}"
            ),
            Self::LanguageMismatch { language, mode, .. } => write!(
                f,
                "Inline error (inline #{n}): @inline {language} cannot be used in {mode} mode"
            ),
        }
    }
}

pub enum VerbatimTagResult {
    Html(String),
    Css(String),
    Inline(InlineComponent),
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

pub enum VerbatimTag {
    Code,
    Image,
    Inline,
    DocumentMeta,
    Unknown,
}

impl VerbatimTag {
    pub fn from_name(name: &[String]) -> Self {
        match name {
            [tag] if tag == "code" => Self::Code,
            [tag] if tag == "image" => Self::Image,
            [tag] if tag == "inline" => Self::Inline,
            [doc, meta] if doc == "document" && meta == "meta" => Self::DocumentMeta,
            _ => Self::Unknown,
        }
    }
}

pub fn verbatim_tag(
    name: &[String],
    parameters: &[String],
    content: &str,
    mode: Option<OutputMode>,
    highlighter: &mut Highlighter,
    inline_index: usize,
) -> Result<Option<VerbatimTagResult>, InlineParseError> {
    match VerbatimTag::from_name(name) {
        VerbatimTag::Code => {
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
            Ok(Some(VerbatimTagResult::Html(html)))
        }
        VerbatimTag::Image => Ok(parameters.first().filter(|s| !s.is_empty()).map(|path| {
            let src = if path.starts_with('/') || path.starts_with("http") {
                path.clone()
            } else {
                format!("./{path}")
            };
            VerbatimTagResult::Html(format!(
                r#"<img src="{}" alt="{}" />"#,
                encode_minimal(&src),
                encode_minimal(content.trim())
            ))
        })),
        VerbatimTag::Inline => {
            let inline_lang = parameters
                .first()
                .filter(|s| !s.is_empty())
                .map(String::as_str);

            match inline_lang {
                Some("css") => Ok(Some(VerbatimTagResult::Css(content.to_string()))),
                None => Err(InlineParseError::MissingLanguage {
                    index: inline_index,
                }),
                Some(lang) => {
                    let inline_mode = lang.parse::<OutputMode>().map_err(|_| {
                        InlineParseError::InvalidLanguage {
                            index: inline_index,
                            language: lang.to_string(),
                        }
                    })?;

                    match mode {
                        None => Ok(None),
                        Some(m) if m != inline_mode => Err(InlineParseError::LanguageMismatch {
                            index: inline_index,
                            language: lang.to_string(),
                            mode: m,
                        }),
                        Some(_) => Ok(Some(VerbatimTagResult::Inline(InlineComponent {
                            index: 0,
                            mode: inline_mode.to_string(),
                            code: content.to_string(),
                        }))),
                    }
                }
            }
        }
        VerbatimTag::DocumentMeta => Ok(None),
        VerbatimTag::Unknown => Ok(Some(VerbatimTagResult::Html(format!(
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
