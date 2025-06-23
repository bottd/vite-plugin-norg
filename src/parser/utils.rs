pub fn into_slug(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|char| if char.is_alphanumeric() { char } else { '-' })
        .fold(String::new(), |mut text, ch| {
            if ch != '-' || text.chars().last() != Some('-') {
                text.push(ch);
            }
            text
        })
        .trim_matches('-')
        .into()
}
