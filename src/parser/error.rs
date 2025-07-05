use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NorgError {
    Parse(String),
    Meta(String),
    Html(String),
    Serial(String),
    Js(String),
    Io(String),
    Unsupported(String),
}

impl fmt::Display for NorgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NorgError::Parse(message) => write!(f, "Parse error: {}", message),
            NorgError::Meta(message) => write!(f, "Meta error: {}", message),
            NorgError::Html(message) => write!(f, "HTML error: {}", message),
            NorgError::Serial(message) => write!(f, "Serial error: {}", message),
            NorgError::Js(message) => write!(f, "JS error: {}", message),
            NorgError::Io(message) => write!(f, "IO error: {}", message),
            NorgError::Unsupported(message) => write!(f, "Unsupported: {}", message),
        }
    }
}

impl std::error::Error for NorgError {}

pub type NorgResult<T> = Result<T, NorgError>;

impl From<serde_json::Error> for NorgError {
    fn from(error: serde_json::Error) -> Self {
        NorgError::Serial(error.to_string())
    }
}

impl From<std::io::Error> for NorgError {
    fn from(error: std::io::Error) -> Self {
        NorgError::Io(error.to_string())
    }
}

impl From<NorgError> for wasm_bindgen::JsValue {
    fn from(error: NorgError) -> Self {
        wasm_bindgen::JsValue::from_str(&error.to_string())
    }
}
