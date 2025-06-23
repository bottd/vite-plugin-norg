use crate::types::TocEntry;
use crate::utils::into_slug;
use htmlescape::encode_minimal;
use rust_norg::{
    DelimitingModifier, DetachedModifierExtension, LinkTarget, NestableDetachedModifier, NorgAST,
    NorgASTFlat, ParagraphSegment, ParagraphSegmentToken, RangeableDetachedModifier, TodoStatus,
};
use textwrap::dedent;

const HTML_TAGS: &[(char, &str, &str)] = &[
    ('*', "<strong>", "</strong>"),
    ('_', "<em>", "</em>"),
    ('^', "<sup>", "</sup>"),
    (',', "<sub>", "</sub>"),
    ('-', "<s>", "</s>"),
    ('!', "<span class=\"spoiler\">", "</span>"),
    ('$', "<span class=\"math\">", "</span>"),
    ('&', "<var>", "</var>"),
    ('/', "<i>", "</i>"),
    ('=', "<mark>", "</mark>"),
];

#[derive(Debug, PartialEq)]
enum VerbatimTag {
    Code,
    Image,
    DocumentMeta,
    Unknown,
}

impl VerbatimTag {
    fn from_slice(slice: &[String]) -> Self {
        match slice {
            [tag] if tag == "code" => Self::Code,
            [tag] if tag == "image" => Self::Image,
            [doc, meta] if doc == "document" && meta == "meta" => Self::DocumentMeta,
            _ => Self::Unknown,
        }
    }
}

const fn delimiter_html(delimiter: &DelimitingModifier) -> &'static str {
    match delimiter {
        DelimitingModifier::Weak => "<hr class=\"weak\" />",
        DelimitingModifier::Strong => "<hr class=\"strong\" />",
        DelimitingModifier::HorizontalRule => "<hr />",
    }
}

pub fn convert_nodes(ast: &[NorgAST]) -> (String, Vec<TocEntry>) {
    let mut toc = Vec::new();
    let mut result = Vec::new();
    let mut i = 0;

    while i < ast.len() {
        match &ast[i] {
            NorgAST::NestableDetachedModifier { modifier_type, .. } => {
                let (html, consumed) = convert_grouped_modifiers(&ast[i..], modifier_type);
                if !html.trim().is_empty() {
                    result.push(html);
                }
                i += consumed;
            }
            node => {
                if let Some(html) = convert_single_node(node, &mut toc) {
                    result.push(html);
                }
                i += 1;
            }
        }
    }

    (result.join("\n"), toc)
}

fn convert_single_node(node: &NorgAST, toc: &mut Vec<TocEntry>) -> Option<String> {
    let html = match node {
        NorgAST::VerbatimRangedTag {
            name,
            parameters,
            content,
            ..
        } => match VerbatimTag::from_slice(name) {
            VerbatimTag::DocumentMeta => return None,
            VerbatimTag::Code => {
                let dedented = dedent(content);
                let encoded = encode_minimal(&dedented);
                match parameters.first().filter(|l| !l.is_empty()) {
                    Some(lang) => format!("<pre class=\"language-{lang}\"><code class=\"language-{lang}\">{encoded}</code></pre>"),
                    None => format!("<pre><code>{encoded}</code></pre>"),
                }
            }
            VerbatimTag::Image => parameters.first().filter(|p| !p.is_empty()).map(|path| {
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
            })?,
            VerbatimTag::Unknown => {
                format!("<div class=\"verbatim\">{}</div>", encode_minimal(content))
            }
        },

        NorgAST::Heading {
            level,
            title,
            content,
            ..
        } => {
            let title_text = convert_paragraph_segments(title);
            let heading_id = into_slug(&title_text);
            let heading = format!("<h{level} id=\"{heading_id}\">{title_text}</h{level}>");

            toc.push(TocEntry {
                level: *level as usize,
                title: title_text,
                id: heading_id,
            });
            let (content_html, _) = convert_nodes(content);

            if content_html.trim().is_empty() {
                heading
            } else {
                format!("{heading}\n{content_html}")
            }
        }

        NorgAST::Paragraph(segments) => {
            let content = convert_paragraph_segments(segments);
            (!content.trim().is_empty()).then(|| format!("<p>{content}</p>"))?
        }

        NorgAST::RangeableDetachedModifier {
            modifier_type,
            title,
            content,
            ..
        } => {
            let title_html = convert_paragraph_segments(title);
            let content_html = convert_flat_content(content);

            match modifier_type {
                RangeableDetachedModifier::Definition => {
                    format!(
                        "<dl><dt>{}</dt><dd>{}</dd></dl>",
                        encode_minimal(&title_html),
                        content_html
                    )
                }
                RangeableDetachedModifier::Footnote => {
                    let id = into_slug(&title_html);
                    format!(
                        "<aside id=\"footnote-{}\" class=\"footnote\"><strong>{}</strong><p>{}</p></aside>",
                        encode_minimal(&id), encode_minimal(&title_html), content_html
                    )
                }
                RangeableDetachedModifier::Table => {
                    format!(
                        "<table><caption>{}</caption><tbody>{}</tbody></table>",
                        encode_minimal(&title_html),
                        content_html
                    )
                }
            }
        }

        NorgAST::DelimitingModifier(delimiter) => delimiter_html(delimiter).to_string(),

        NorgAST::NestableDetachedModifier { .. } => return None,

        NorgAST::CarryoverTag { .. } | NorgAST::RangedTag { .. } | NorgAST::InfirmTag { .. } => {
            todo!()
        }
    };

    Some(html)
}

fn convert_grouped_modifiers(
    ast: &[NorgAST],
    modifier_type: &NestableDetachedModifier,
) -> (String, usize) {
    let matching_nodes: Vec<_> = ast.iter()
        .take_while(|node| matches!(node, NorgAST::NestableDetachedModifier { modifier_type: mt, .. } if mt == modifier_type))
        .collect();

    let items: Vec<String> = matching_nodes
        .iter()
        .filter_map(|node| match node {
            NorgAST::NestableDetachedModifier {
                text, extensions, ..
            } => {
                if let NorgASTFlat::Paragraph(segments) = text.as_ref() {
                    let content = convert_paragraph_segments(segments);
                    (!content.trim().is_empty()).then(|| format_list_item(&content, extensions))
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect();

    let consumed = matching_nodes.len();
    let html = if items.is_empty() {
        String::new()
    } else {
        let joined = items.join("\n");
        let tag_name = match modifier_type {
            NestableDetachedModifier::UnorderedList => "ul",
            NestableDetachedModifier::OrderedList => "ol",
            NestableDetachedModifier::Quote => "blockquote",
        };
        format!("<{tag_name}>\n{joined}\n</{tag_name}>")
    };

    (html, consumed)
}

fn format_list_item(content: &str, extensions: &[DetachedModifierExtension]) -> String {
    let mut classes = Vec::new();
    let mut attributes = Vec::new();
    let mut prefix_parts = Vec::new();

    for ext in extensions {
        match ext {
            DetachedModifierExtension::Todo(status) => {
                if let TodoStatus::Recurring(date) = status {
                    let date_text = date.as_deref().unwrap_or("");
                    prefix_parts.push(format!(
                        "<span class=\"todo-status\">+ {}</span>",
                        encode_minimal(date_text)
                    ));
                    classes.push("todo-recurring".to_string());
                } else {
                    let status_html = match status {
                        TodoStatus::Undone => String::from("<input type=\"checkbox\" class=\"todo-status todo-undone\" disabled />"),
                        TodoStatus::Done => String::from("<input type=\"checkbox\" class=\"todo-status todo-done\" checked disabled />"),
                        TodoStatus::NeedsClarification => String::from("<span class=\"todo-status todo-clarification\">?</span>"),
                        TodoStatus::Paused => String::from("<span class=\"todo-status todo-paused\">=</span>"),
                        TodoStatus::Urgent => String::from("<span class=\"todo-status todo-urgent\">!</span>"),
                        TodoStatus::Pending => String::from("<span class=\"todo-status todo-pending\">-</span>"),
                        TodoStatus::Canceled => String::from("<span class=\"todo-status todo-canceled\">_</span>"),
                        TodoStatus::Recurring(date) => match date {
                            Some(date) => format!(
                                "<span class=\"todo-status todo-recurring\">+ {}</span>",
                                encode_minimal(date)
                            ),
                            None => String::from("<span class=\"todo-status todo-recurring\">_</span>")
                        },
                    };
                    prefix_parts.push(status_html);
                }
            }
            DetachedModifierExtension::Priority(p) => {
                classes.push(format!("priority-{}", into_slug(p)));
                attributes.push(format!("data-priority=\"{}\"", encode_minimal(p)));
            }
            DetachedModifierExtension::Timestamp(ts) => {
                attributes.push(format!("data-timestamp=\"{}\"", encode_minimal(ts)))
            }
            DetachedModifierExtension::DueDate(date) => {
                attributes.push(format!("data-due=\"{}\"", encode_minimal(date)))
            }
            DetachedModifierExtension::StartDate(date) => {
                attributes.push(format!("data-start=\"{}\"", encode_minimal(date)))
            }
        }
    }

    let mut li_tag = vec!["<li".to_string()];

    if !classes.is_empty() {
        li_tag.push(format!("class=\"{}\"", classes.join(" ")));
    }
    if !attributes.is_empty() {
        li_tag.push(attributes.join(" "));
    }

    li_tag.push(">".to_string());
    if !prefix_parts.is_empty() {
        li_tag.push(prefix_parts.join(" "));
        li_tag.push(" ".to_string());
    }
    li_tag.push(content.to_string());
    li_tag.push("</li>".to_string());

    li_tag.join("")
}

fn process_token<F>(token: &ParagraphSegmentToken, encode_fn: F) -> String
where
    F: Fn(&str) -> String,
{
    match token {
        ParagraphSegmentToken::Whitespace => " ".into(),
        ParagraphSegmentToken::Text(text) => encode_fn(text),
        ParagraphSegmentToken::Special(ch) => encode_fn(&ch.to_string()),
        ParagraphSegmentToken::Escape(ch) => ch.to_string(),
    }
}

fn convert_paragraph_segments_with_code_escaping(segments: &[ParagraphSegment]) -> String {
    segments
        .iter()
        .filter_map(|segment| match segment {
            ParagraphSegment::Token(token) => Some(process_token(token, encode_minimal)),
            _ => None,
        })
        .collect()
}

fn convert_paragraph_segments(segments: &[ParagraphSegment]) -> String {
    segments
        .iter()
        .map(|segment| match segment {
            ParagraphSegment::Token(token) => process_token(token, encode_minimal),

            ParagraphSegment::AttachedModifier {
                modifier_type,
                content,
            } => match *modifier_type {
                '`' => {
                    let code_inner = convert_paragraph_segments_with_code_escaping(content);
                    format!("<code>{code_inner}</code>")
                }
                _ => {
                    let inner = convert_paragraph_segments(content);
                    if let Some((_, open, close)) =
                        HTML_TAGS.iter().find(|(ch, _, _)| ch == modifier_type)
                    {
                        format!("{open}{inner}{close}")
                    } else {
                        inner
                    }
                }
            },

            ParagraphSegment::Link {
                targets,
                description,
                ..
            } => targets
                .first()
                .map(|target| {
                    convert_link(
                        target,
                        description
                            .as_ref()
                            .map(|d| convert_paragraph_segments(d))
                            .as_deref(),
                    )
                })
                .unwrap_or_default(),

            ParagraphSegment::Anchor { content, .. } => convert_paragraph_segments(content),

            ParagraphSegment::InlineVerbatim(tokens) => {
                let estimated_size = tokens.len() * 8; // Conservative estimate
                let content =
                    tokens
                        .iter()
                        .fold(String::with_capacity(estimated_size), |mut acc, token| {
                            acc.push_str(&token.to_string());
                            acc
                        });
                format!("<code>{}</code>", encode_minimal(&content))
            }

            _ => String::new(),
        })
        .collect()
}

fn convert_link(target: &LinkTarget, custom_text: Option<&str>) -> String {
    match target {
        LinkTarget::Url(url) => {
            let display = custom_text.unwrap_or(url);

            if url.starts_with("http") {
                format!(
                    r#"<a href="{}" target="_blank">{}</a>"#,
                    encode_minimal(url),
                    encode_minimal(display)
                )
            } else {
                let href = url
                    .strip_suffix(".norg")
                    .map_or_else(|| url.clone(), |base| format!("{base}.html"));
                format!(
                    r#"<a href="{}">{}</a>"#,
                    encode_minimal(&href),
                    encode_minimal(display)
                )
            }
        }
        LinkTarget::Heading { title, .. } => {
            let estimated_size = title.len() * 16; // Conservative estimate for debug format
            let title_text =
                title
                    .iter()
                    .fold(String::with_capacity(estimated_size), |mut acc, seg| {
                        acc.push_str(&format!("{seg:?}"));
                        acc
                    });
            let slug = into_slug(&title_text);
            format!(
                "<a href=\"#{}\">{}</a>",
                encode_minimal(&slug),
                encode_minimal(custom_text.unwrap_or(&title_text))
            )
        }
        _ => format!("{:?}", target),
    }
}

fn convert_flat_content(content: &[NorgASTFlat]) -> String {
    content
        .iter()
        .filter_map(|node| match node {
            NorgASTFlat::Paragraph(segments) => {
                let html = convert_paragraph_segments(segments);
                (!html.trim().is_empty()).then(|| format!("<p>{html}</p>"))
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}
