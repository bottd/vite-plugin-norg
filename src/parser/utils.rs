/// Slugifies arbitrary text: lowercase alphanumerics joined by single dashes,
/// with no leading or trailing dash. Lowercases via `str::to_lowercase` so
/// context-sensitive mappings (e.g. Greek word-final Σ → ς) match the rendered
/// text; `char::to_lowercase` would emit a different codepoint here and break
/// inbound anchor links.
pub fn into_slug(text: &str) -> String {
    let lowered = text.to_lowercase();
    let mut slug = String::with_capacity(lowered.len());
    for c in lowered.chars() {
        if c.is_alphanumeric() {
            slug.push(c);
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    if slug.ends_with('-') {
        slug.pop();
    }
    slug
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_into_slug() {
        assert_eq!(into_slug("Hello World"), "hello-world");
        assert_eq!(into_slug("Special!@#Characters"), "special-characters");
        assert_eq!(into_slug("Multiple   Spaces"), "multiple-spaces");
        assert_eq!(into_slug(""), "");
        assert_eq!(into_slug("!!!"), "");
        assert_eq!(into_slug("123"), "123");
        // Greek word-final sigma must lowercase to ς (U+03C2), not σ (U+03C3),
        // matching `str::to_lowercase`'s context-aware mapping.
        assert_eq!(into_slug("ΛΟΓΟΣ"), "λογος");
    }
}
