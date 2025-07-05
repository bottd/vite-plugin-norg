use rust_norg::NorgAST;
use std::fs;

pub fn load_parse(name: &str) -> Result<Vec<NorgAST>, String> {
    let path = format!("tests/fixtures/{}", name);
    let content =
        fs::read_to_string(&path).map_err(|e| format!("Read fixture failed {}: {}", path, e))?;

    let ast = rust_norg::parse_tree(&content)
        .map_err(|e| format!("Parse fixture failed {}: {:?}", name, e))?;

    Ok(ast)
}

pub fn load_content(name: &str) -> Result<String, String> {
    let path = format!("tests/fixtures/{}", name);
    let content =
        fs::read_to_string(&path).map_err(|e| format!("Read fixture failed {}: {}", path, e))?;

    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_parse() {
        let ast = load_parse("basic.norg").expect("Failed to load and parse basic.norg");
        assert!(!ast.is_empty());
    }

    #[test]
    fn test_load_content() {
        let content = load_content("basic.norg").expect("Failed to load basic.norg content");
        assert!(content.contains("@document.meta"));
        assert!(content.contains("* Main Title"));
    }
}
