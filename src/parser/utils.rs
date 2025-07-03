/// Converts text into a URL-safe slug.
pub fn into_slug(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-")
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
    }
}
