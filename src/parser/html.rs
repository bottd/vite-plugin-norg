use crate::utils::into_slug;
use htmlescape::encode_minimal;
use rust_norg::{
    DelimitingModifier, DetachedModifierExtension, LinkTarget, NestableDetachedModifier, NorgAST,
    NorgASTFlat, ParagraphSegment, ParagraphSegmentToken, RangeableDetachedModifier, TodoStatus,
};
use std::fmt::Write;
use textwrap::dedent;

pub fn transform(ast: &[NorgAST]) -> String {
    ast.iter()
        .filter_map(|node| {
            let html = match node {
            NorgAST::NestableDetachedModifier {
                modifier_type,
                text,
                extensions,
                ..
            } => match text.as_ref() {
                NorgASTFlat::Paragraph(segments) => {
                    let content = conv_segs(segments);
                    if !content.trim().is_empty() {
                        let tag = match modifier_type {
                            NestableDetachedModifier::UnorderedList => "ul",
                            NestableDetachedModifier::OrderedList => "ol",
                            NestableDetachedModifier::Quote => "blockquote",
                        };
                        format!("<{tag}>{}</{tag}>", fmt_li(&content, extensions))
                    } else {
                        String::new()
                    }
                }
                _ => todo!("Unsupported nestable modifier text type"),
            },

            NorgAST::VerbatimRangedTag {
                name,
                parameters,
                content,
                ..
            } => match name.as_slice() {
                [tag] if tag == "code" => {
                    let text = encode_minimal(&dedent(content));
                    if let Some(lang) = parameters.first().filter(|l| !l.is_empty()) {
                        format!(
                            r#"<pre class="language-{lang}"><code class="language-{lang}">{text}</code></pre>"#
                        )
                    } else {
                        format!("<pre><code>{text}</code></pre>")
                    }
                }
                [tag] if tag == "image" => {
                    let path = parameters.first().filter(|p| !p.is_empty());
                    match path {
                        Some(path) => {
                            let src = if path.starts_with('/') || path.starts_with("http") {
                                path.clone()
                            } else {
                                format!("./{path}")
                            };
                            format!(
                                r#"<img src="{}" alt="{}" />"#,
                                encode_minimal(&src),
                                encode_minimal(content.trim())
                            )
                        }
                        None => String::new(),
                    }
                }
                [doc, meta] if doc == "document" && meta == "meta" => String::new(),
                _ => format!(r#"<div class="verbatim">{}</div>"#, encode_minimal(content)),
            },

            NorgAST::Heading {
                level,
                title,
                content,
                ..
            } => {
                let text = conv_segs(title);
                let id = into_slug(&text);
                let heading = format!("<h{level} id=\"{id}\">{text}</h{level}>");

                let html = transform(content);

                if html.trim().is_empty() {
                    heading
                } else {
                    format!("{heading}\n{html}")
                }
            }

            NorgAST::Paragraph(segments) => {
                let content = conv_segs(segments);
                if content.trim().is_empty() {
                    String::new()
                } else {
                    format!("<p>{content}</p>")
                }
            }

            NorgAST::RangeableDetachedModifier {
                modifier_type,
                title,
                content,
                ..
            } => {
                let title = conv_segs(title);
                let content = content
                    .iter()
                    .filter_map(|node| match node {
                        NorgASTFlat::Paragraph(segments) => {
                            let html = conv_segs(segments);
                            (!html.trim().is_empty()).then(|| format!("<p>{html}</p>"))
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                match modifier_type {
                    RangeableDetachedModifier::Definition => format!(
                        "<dl><dt>{}</dt><dd>{}</dd></dl>",
                        encode_minimal(&title),
                        content
                    ),
                    RangeableDetachedModifier::Footnote => {
                        let id = into_slug(&title);
                        format!("<aside id=\"footnote-{}\" class=\"footnote\"><strong>{}</strong><p>{}</p></aside>", encode_minimal(&id), encode_minimal(&title), content)
                    }
                    RangeableDetachedModifier::Table => format!(
                        "<table><caption>{}</caption><tbody>{}</tbody></table>",
                        encode_minimal(&title),
                        content
                    ),
                }
            }

            NorgAST::DelimitingModifier(delimiter) => match delimiter {
                DelimitingModifier::Weak => "<hr class=\"weak\" />".to_string(),
                DelimitingModifier::Strong => "<hr class=\"strong\" />".to_string(),
                DelimitingModifier::HorizontalRule => "<hr />".to_string(),
            },

            NorgAST::CarryoverTag { .. } => todo!("CarryoverTag not implemented"),
            NorgAST::RangedTag { .. } => todo!("RangedTag not implemented"),
            NorgAST::InfirmTag { .. } => todo!("InfirmTag not implemented"),
            };

            if html.is_empty() {
                None
            } else {
                Some(html)
            }
        })
        .collect::<Vec<String>>()
        .join("\n")
}

fn fmt_li(content: &str, extensions: &[DetachedModifierExtension]) -> String {
    let mut classes = Vec::new();
    let mut attrs = Vec::new();
    let mut prefix = Vec::new();

    for extension in extensions {
        match extension {
            DetachedModifierExtension::Todo(status) => {
                if let TodoStatus::Recurring(_) = status {
                    classes.push("todo-recurring".to_string());
                }
                let html = match status {
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
                    TodoStatus::Recurring(date) => {
                        let date = date.as_deref().unwrap_or("");
                        return format!(
                            r#"<span class="todo-status todo-recurring">+ {}</span>"#,
                            encode_minimal(date)
                        );
                    }
                };
                prefix.push(html.to_string());
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

    let mut result = String::new();
    write!(&mut result, "<li").unwrap();

    if !classes.is_empty() {
        write!(&mut result, r#" class="{}""#, classes.join(" ")).unwrap();
    }

    for attr in &attrs {
        write!(&mut result, " {}", attr).unwrap();
    }

    write!(&mut result, ">").unwrap();

    if !prefix.is_empty() {
        write!(&mut result, "{} ", prefix.join(" ")).unwrap();
    }

    write!(&mut result, "{}</li>", content).unwrap();
    result
}

fn proc_token(token: &ParagraphSegmentToken, encode: impl Fn(&str) -> String) -> String {
    match token {
        ParagraphSegmentToken::Whitespace => " ".into(),
        ParagraphSegmentToken::Text(text) => encode(text),
        ParagraphSegmentToken::Special(ch) => encode(&ch.to_string()),
        ParagraphSegmentToken::Escape(ch) => ch.to_string(),
    }
}

fn conv_code(segments: &[ParagraphSegment]) -> String {
    segments
        .iter()
        .filter_map(|segment| match segment {
            ParagraphSegment::Token(token) => Some(proc_token(token, encode_minimal)),
            _ => None,
        })
        .collect()
}

fn conv_segs(segments: &[ParagraphSegment]) -> String {
    segments
        .iter()
        .map(|segment| match segment {
            ParagraphSegment::Token(token) => proc_token(token, encode_minimal),

            ParagraphSegment::AttachedModifier {
                modifier_type,
                content,
            } => match *modifier_type {
                '`' => format!("<code>{}</code>", conv_code(content)),
                '*' => format!("<strong>{}</strong>", conv_segs(content)),
                '_' => format!("<em>{}</em>", conv_segs(content)),
                '^' => format!("<sup>{}</sup>", conv_segs(content)),
                ',' => format!("<sub>{}</sub>", conv_segs(content)),
                '-' => format!("<s>{}</s>", conv_segs(content)),
                '!' => format!("<span class=\"spoiler\">{}</span>", conv_segs(content)),
                '$' => format!("<span class=\"math\">{}</span>", conv_segs(content)),
                '&' => format!("<var>{}</var>", conv_segs(content)),
                '/' => format!("<i>{}</i>", conv_segs(content)),
                '=' => format!("<mark>{}</mark>", conv_segs(content)),
                _ => conv_segs(content),
            },

            ParagraphSegment::Link {
                targets,
                description,
                filepath,
                ..
            } => {
                let text = description.as_ref().map(|d| conv_segs(d));

                match targets.first() {
                    Some(LinkTarget::Url(url)) => {
                        let display_text = text.as_deref().unwrap_or(url);
                        let href = if let Some(fp) = filepath {
                            fp.clone()
                        } else if url.starts_with("http") {
                            url.clone()
                        } else {
                            url.strip_suffix(".norg")
                                .map(|base| format!("{base}.html"))
                                .unwrap_or_else(|| url.clone())
                        };

                        if url.starts_with("http") && filepath.is_none() {
                            format!(
                                r#"<a href="{}" target="_blank">{}</a>"#,
                                encode_minimal(&href),
                                encode_minimal(display_text)
                            )
                        } else {
                            format!(
                                r#"<a href="{}">{}</a>"#,
                                encode_minimal(&href),
                                encode_minimal(display_text)
                            )
                        }
                    }
                    Some(LinkTarget::Heading { title, .. }) => {
                        let title_text = title
                            .iter()
                            .map(|segment| format!("{segment:?}"))
                            .collect::<String>();
                        let slug = into_slug(&title_text);
                        let display_text = text.as_deref().unwrap_or(&title_text);
                        format!(
                            "<a href=\"#{}\">{}</a>",
                            encode_minimal(&slug),
                            encode_minimal(display_text)
                        )
                    }
                    Some(LinkTarget::Footnote(_)) => todo!(),
                    Some(LinkTarget::Definition(_)) => todo!(),
                    Some(LinkTarget::Path(_)) => todo!(),
                    Some(LinkTarget::Timestamp(_)) => todo!(),
                    Some(LinkTarget::Generic(_)) => todo!(),
                    Some(LinkTarget::Extendable(_)) => todo!(),
                    Some(LinkTarget::Wiki(_)) => todo!(),
                    None => String::new(),
                }
            }
            ParagraphSegment::Anchor { content, .. } => conv_segs(content),

            ParagraphSegment::InlineVerbatim(tokens) => {
                format!(
                    "<code>{}</code>",
                    encode_minimal(&tokens.iter().map(ToString::to_string).collect::<String>())
                )
            }

            _ => todo!("Unsupported paragraph segment type"),
        })
        .collect()
}
