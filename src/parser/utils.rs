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

/// How a link target addresses its destination. Rewriting, new-tab hardening
/// and scheme blocking all read from this one classification, so they cannot
/// disagree about what a given target is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UrlKind<'a> {
    /// `docs/readme.norg`, `/absolute`, `#section`.
    SiteRelative,
    /// `//host/…` — cross-origin, but with no scheme to inspect.
    ProtocolRelative,
    /// Borrowed verbatim, so compare case-insensitively.
    Scheme(&'a str),
}

impl<'a> UrlKind<'a> {
    /// A scheme is `ALPHA *( ALPHA / DIGIT / "+" / "-" / "." ) ":"` before any
    /// `/`, `?`, or `#` (RFC 3986); a colon anywhere else — `path/to:file`,
    /// `?x=a:b` — leaves the URL relative.
    pub fn of(url: &'a str) -> Self {
        match url.find([':', '/', '?', '#']) {
            Some(end) if url.as_bytes()[end] == b':' && is_scheme_shaped(&url[..end]) => {
                Self::Scheme(&url[..end])
            }
            _ if url.starts_with("//") => Self::ProtocolRelative,
            _ => Self::SiteRelative,
        }
    }

    /// Gets `target="_blank"` + `rel="noopener noreferrer"`. Only web URLs
    /// qualify: `mailto:`/`tel:` are handed to the OS, not opened as pages.
    pub fn is_external(self) -> bool {
        match self {
            Self::ProtocolRelative => true,
            Self::Scheme(scheme) => {
                scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https")
            }
            Self::SiteRelative => false,
        }
    }

    /// The only kind path rewriting (`.norg` → `.html`, a `./` prefix) may
    /// touch. `mailto:me@example.norg` ends in `.norg` but is an address, and
    /// rewriting it corrupts it.
    pub fn is_site_relative(self) -> bool {
        matches!(self, Self::SiteRelative)
    }
}

fn is_scheme_shaped(scheme: &str) -> bool {
    let mut chars = scheme.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

/// True for schemes that execute script when navigated. Denylist, not
/// allowlist, so ordinary links keep working. The normalisation mirrors browser
/// URL parsing — leading C0 controls stripped, tab/newline ignored anywhere —
/// so `" java\nscript:"` cannot slip past.
pub fn has_unsafe_scheme(href: &str) -> bool {
    let normalized: String = href
        .trim_start_matches(|c: char| c.is_ascii_control() || c == ' ')
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();

    let UrlKind::Scheme(scheme) = UrlKind::of(&normalized) else {
        return false;
    };
    match scheme.to_ascii_lowercase().as_str() {
        "javascript" | "vbscript" => true,
        // Raster images can't run script; SVG and everything else can.
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
    fn url_kind_classifies_by_scheme() {
        use UrlKind::*;

        // A scheme is only a scheme when it precedes any `/`, `?`, or `#`,
        // so a filename that merely starts with "http" stays a path.
        assert_eq!(UrlKind::of("http://example.com"), Scheme("http"));
        assert_eq!(UrlKind::of("HTTPS://example.com"), Scheme("HTTPS"));
        assert_eq!(UrlKind::of("mailto:me@example.norg"), Scheme("mailto"));
        assert_eq!(UrlKind::of("ftp://host/file.norg"), Scheme("ftp"));
        assert_eq!(UrlKind::of("httpserver.norg"), SiteRelative);
        assert_eq!(UrlKind::of("https.norg"), SiteRelative);
        assert_eq!(UrlKind::of("//cdn.example.com/x"), ProtocolRelative);
        assert_eq!(UrlKind::of("/absolute/path"), SiteRelative);
        assert_eq!(UrlKind::of("./relative"), SiteRelative);
        assert_eq!(UrlKind::of("#section"), SiteRelative);
        assert_eq!(UrlKind::of(""), SiteRelative);
        // A colon that isn't a scheme separator: after a path segment, inside a
        // query, leading, or in a prefix that isn't scheme-shaped.
        assert_eq!(UrlKind::of("path/to:file"), SiteRelative);
        assert_eq!(UrlKind::of("foo?x=a:b"), SiteRelative);
        assert_eq!(UrlKind::of(":leading"), SiteRelative);
        assert_eq!(UrlKind::of("1scheme:x"), SiteRelative);
    }

    #[test]
    fn only_web_urls_are_external() {
        // External == opens as a page elsewhere, so it gets `target="_blank"`
        // and `rel="noopener noreferrer"`.
        assert!(UrlKind::of("http://example.com").is_external());
        assert!(UrlKind::of("HTTPS://example.com").is_external());
        assert!(UrlKind::of("//cdn.example.com/x").is_external());
        // Handed to the OS rather than opened as a page — a new tab is wrong.
        assert!(!UrlKind::of("mailto:a@b.com").is_external());
        assert!(!UrlKind::of("tel:+15551234567").is_external());
        assert!(!UrlKind::of("ftp://host/pub").is_external());
        assert!(!UrlKind::of("docs/readme.norg").is_external());
        assert!(!UrlKind::of("#section").is_external());
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
