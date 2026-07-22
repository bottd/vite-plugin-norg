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

/// Returns true for absolute `http://` / `https://` URLs. Deliberately requires
/// the `://` separator so it doesn't match a same-document path like
/// `httpserver.norg` the way a bare `starts_with("http")` would.
pub fn is_http_url(s: &str) -> bool {
    let Some((scheme, rest)) = s.split_once(':') else {
        return false;
    };
    rest.starts_with("//")
        && (scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"))
}

/// True for a link pointing off the current site: an absolute `http(s)://` URL
/// or a protocol-relative `//host/…` URL. Both open in a new tab with
/// `rel="noopener noreferrer"`; a protocol-relative URL must not be mistaken
/// for an in-site `.norg` path (which would get rewritten/prefixed and break).
pub fn is_external_url(s: &str) -> bool {
    is_http_url(s) || s.starts_with("//")
}

/// True if `href` carries an explicit URL scheme that can execute script when
/// navigated (`javascript:`, `vbscript:`, and scriptable `data:` payloads).
/// Benign schemes (`http`, `https`, `mailto`, `tel`, `ftp`, …) and scheme-less
/// relative links are safe. Browser URL parsing strips leading C0 controls/spaces
/// and ignores ASCII tab/newline inside URLs, so this check mirrors that before
/// finding a scheme.
pub fn has_unsafe_scheme(href: &str) -> bool {
    let normalized: String = href
        .trim_start_matches(|c: char| c.is_ascii_control() || c == ' ')
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();

    // A scheme is `ALPHA *( ALPHA / DIGIT / "+" / "-" / "." ) ":"` that
    // appears before any `/`, `?`, or `#`. If the first such delimiter is not a
    // `:`, there is no scheme (the href is relative / a fragment) and it is
    // safe.
    let Some(pos) = normalized.find([':', '/', '?', '#']) else {
        return false;
    };
    if normalized.as_bytes()[pos] != b':' {
        return false;
    }
    let scheme = &normalized[..pos];
    let mut chars = scheme.chars();
    let valid = chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    // A `pos` of 0 (leading `:`) or a non-scheme-shaped prefix isn't a real
    // scheme; treat it as safe (relative) rather than blocking it.
    if !valid {
        return false;
    }
    match scheme.to_ascii_lowercase().as_str() {
        "javascript" | "vbscript" => true,
        // `data:` can carry executable payloads (`text/html`, `image/svg+xml`
        // with embedded scripts). Allow only non-SVG image media types — raster
        // images can't run script — and block everything else.
        "data" => {
            let lower = normalized.to_ascii_lowercase();
            !lower.starts_with("data:image/") || lower.starts_with("data:image/svg")
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_http_url() {
        assert!(is_http_url("http://example.com"));
        assert!(is_http_url("https://example.com"));
        assert!(is_http_url("HTTP://example.com"));
        assert!(is_http_url("Https://example.com"));
        assert!(!is_http_url("httpserver.norg"));
        assert!(!is_http_url("https.norg"));
        assert!(!is_http_url("/absolute/path"));
        assert!(!is_http_url("./relative"));
    }

    #[test]
    fn test_has_unsafe_scheme() {
        // Dangerous schemes are blocked (case-insensitive, whitespace/case tricks).
        assert!(has_unsafe_scheme("javascript:alert(1)"));
        assert!(has_unsafe_scheme("JavaScript:alert(1)"));
        assert!(has_unsafe_scheme(" java\nscript:alert(1)"));
        assert!(has_unsafe_scheme("\tdata:text/html,<script>"));
        assert!(has_unsafe_scheme("data:text/html,<script>"));
        assert!(has_unsafe_scheme("vbscript:msgbox"));
        // `data:` is blocked except for non-SVG image media types. SVG can carry
        // scripts, so it stays blocked; raster images are allowed through.
        assert!(has_unsafe_scheme(
            "data:image/svg+xml,<svg onload=alert(1)>"
        ));
        assert!(has_unsafe_scheme("data:application/octet-stream,x"));
        assert!(!has_unsafe_scheme("data:image/png;base64,iVBORw0KGgo="));
        assert!(!has_unsafe_scheme("DATA:IMAGE/PNG;base64,iVBORw0KGgo="));
        assert!(!has_unsafe_scheme("data:image/gif;base64,R0lGOD"));
        // Benign schemes are NOT blocked — the check is a denylist of script
        // schemes, not an allowlist, so ordinary links keep working.
        assert!(!has_unsafe_scheme("http://example.com"));
        assert!(!has_unsafe_scheme("https://example.com"));
        assert!(!has_unsafe_scheme("mailto:a@b.com"));
        assert!(!has_unsafe_scheme("tel:+15551234567"));
        assert!(!has_unsafe_scheme("ftp://ftp.gnu.org/gnu/"));
        assert!(!has_unsafe_scheme("sms:+15551234567"));
        // No scheme: relative paths, absolute paths, fragments are safe.
        assert!(!has_unsafe_scheme("docs/readme.html"));
        assert!(!has_unsafe_scheme("./relative"));
        assert!(!has_unsafe_scheme("/absolute/path"));
        assert!(!has_unsafe_scheme("//protocol-relative.example.com"));
        assert!(!has_unsafe_scheme("#section"));
        // A colon after a path separator is not a scheme.
        assert!(!has_unsafe_scheme("path/to:file"));
        assert!(!has_unsafe_scheme("foo?x=a:b"));
    }

    #[test]
    fn test_is_external_url() {
        assert!(is_external_url("http://example.com"));
        assert!(is_external_url("https://example.com"));
        assert!(is_external_url("//cdn.example.com/x"));
        assert!(!is_external_url("docs/readme.norg"));
        assert!(!is_external_url("/absolute/path"));
        assert!(!is_external_url("#section"));
    }

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
