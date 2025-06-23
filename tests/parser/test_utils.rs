use std::fs;
use rust_norg::NorgAST;

pub fn load_fixture_and_parse(fixture_name: &str) -> Vec<NorgAST> {
    let fixture_path = format!("tests/fixtures/{}", fixture_name);
    let content = fs::read_to_string(&fixture_path)
        .unwrap_or_else(|_| panic!("Failed to read fixture file: {}", fixture_path));
    
    rust_norg::parse_tree(&content)
        .unwrap_or_else(|e| panic!("Failed to parse fixture {}: {:?}", fixture_name, e))
}

pub fn load_fixture_content(fixture_name: &str) -> String {
    let fixture_path = format!("tests/fixtures/{}", fixture_name);
    fs::read_to_string(&fixture_path)
        .unwrap_or_else(|_| panic!("Failed to read fixture file: {}", fixture_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_fixture_and_parse_basic() {
        let ast = load_fixture_and_parse("basic.norg");
        assert!(!ast.is_empty());
    }

    #[test]
    fn test_load_fixture_content() {
        let content = load_fixture_content("basic.norg");
        assert!(content.contains("@document.meta"));
        assert!(content.contains("* Main Title"));
    }
}