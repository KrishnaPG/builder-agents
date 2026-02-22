//! JSON configuration parser
//!
//! Uses serde_json for robust JSON parsing into typed config artifacts.

use crate::error::ParseError;
use crate::parsers::ArtifactParser;
use coa_artifact::{Artifact, ArtifactType, ContentHash};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// JSON configuration content
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonContent {
    /// Root JSON value
    pub root: Value,
    /// Schema reference (optional)
    pub schema: Option<String>,
    /// Preserved formatting comments (if any)
    pub comments: HashMap<String, String>,
}

impl JsonContent {
    /// Create from serde_json::Value
    #[inline]
    #[must_use]
    pub fn new(value: Value) -> Self {
        Self {
            root: value,
            schema: None,
            comments: HashMap::new(),
        }
    }

    /// Get value at path (dot notation)
    #[must_use]
    pub fn get_path(&self, path: &str) -> Option<&Value> {
        let mut current = &self.root;
        for segment in path.split('.') {
            match current {
                Value::Object(map) => current = map.get(segment)?,
                _ => return None,
            }
        }
        Some(current)
    }

    /// Set value at path (dot notation)
    ///
    /// Creates intermediate objects as needed.
    pub fn set_path(&mut self, path: &str, value: Value) {
        let segments: Vec<_> = path.split('.').collect();
        if segments.is_empty() {
            return;
        }

        let mut current = &mut self.root;
        for segment in &segments[..segments.len() - 1] {
            match current {
                Value::Object(map) => {
                    current = map
                        .entry(segment.to_string())
                        .or_insert_with(|| Value::Object(serde_json::Map::new()));
                }
                _ => {
                    // Replace with object if not object
                    *current = Value::Object(serde_json::Map::new());
                    if let Value::Object(map) = current {
                        current = map
                            .entry(segment.to_string())
                            .or_insert_with(|| Value::Object(serde_json::Map::new()));
                    }
                }
            }
        }

        if let Value::Object(map) = current {
            map.insert(segments.last().unwrap().to_string(), value);
        }
    }
}

/// JSON config artifact type
#[derive(Debug, Clone)]
pub struct JsonArtifact;

impl coa_artifact::__private::Sealed for JsonArtifact {}

impl ArtifactType for JsonArtifact {
    type Content = JsonContent;

    fn hash(content: &Self::Content) -> ContentHash {
        let json_str =
            serde_json::to_string(&content.root).unwrap_or_default();
        ContentHash::compute(json_str.as_bytes())
    }

    const TYPE_ID: &'static str = "json";
}

/// JSON parser
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonParser;

impl JsonParser {
    /// Create new JSON parser
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl ArtifactParser for JsonParser {
    type Output = JsonArtifact;

    fn parse(&self, content: &str) -> Result<Artifact<Self::Output>, ParseError> {
        // Parse JSON
        let value: Value = serde_json::from_str(content).map_err(|e| {
            ParseError::SyntaxError {
                path: std::path::PathBuf::from("input.json"),
                message: format!("JSON parse error: {}", e),
            }
        })?;

        // Extract schema if present
        let schema = value
            .get("$schema")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Create content
        let json_content = JsonContent {
            root: value,
            schema,
            comments: HashMap::new(),
        };

        // Create artifact
        Artifact::new(json_content).map_err(|e| {
            ParseError::ValidationError(format!("artifact creation failed: {}", e))
        })
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_parser_valid() {
        let parser = JsonParser;
        let content = r#"{"name": "test", "value": 42}"#;

        let result = parser.parse(content);
        assert!(result.is_ok());

        let artifact = result.unwrap();
        assert_eq!(artifact.content().root["name"], "test");
        assert_eq!(artifact.content().root["value"], 42);
    }

    #[test]
    fn json_parser_invalid() {
        let parser = JsonParser;
        let content = r#"{"name": "test", "value":}"#; // Invalid JSON

        let result = parser.parse(content);
        assert!(result.is_err());
    }

    #[test]
    fn json_parser_empty() {
        let parser = JsonParser;
        let content = "";

        let result = parser.parse(content);
        assert!(result.is_err()); // Empty is not valid JSON
    }

    #[test]
    fn json_parser_extracts_schema() {
        let parser = JsonParser;
        let content = r#"{"$schema": "http://example.com/schema.json", "name": "test"}"#;

        let result = parser.parse(content);
        assert!(result.is_ok());

        let artifact = result.unwrap();
        assert_eq!(
            artifact.content().schema,
            Some("http://example.com/schema.json".to_string())
        );
    }

    #[test]
    fn json_content_get_path() {
        let content = JsonContent::new(serde_json::json!({
            "server": {
                "host": "localhost",
                "port": 8080
            },
            "debug": true
        }));

        assert_eq!(content.get_path("server.host"), Some(&Value::String("localhost".to_string())));
        assert_eq!(content.get_path("server.port"), Some(&Value::Number(8080.into())));
        assert_eq!(content.get_path("debug"), Some(&Value::Bool(true)));
        assert_eq!(content.get_path("missing"), None);
        assert_eq!(content.get_path("server.missing"), None);
    }

    #[test]
    fn json_content_set_path() {
        let mut content = JsonContent::new(serde_json::json!({}));

        content.set_path("server.port", Value::Number(8080.into()));
        assert_eq!(content.get_path("server.port"), Some(&Value::Number(8080.into())));

        content.set_path("debug", Value::Bool(true));
        assert_eq!(content.get_path("debug"), Some(&Value::Bool(true)));
    }

    #[test]
    fn json_artifact_type_id() {
        assert_eq!(JsonArtifact::TYPE_ID, "json");
    }

    #[test]
    fn json_artifact_hash_deterministic() {
        let content1 = JsonContent::new(serde_json::json!({"a": 1, "b": 2}));
        let content2 = JsonContent::new(serde_json::json!({"a": 1, "b": 2}));

        assert_eq!(JsonArtifact::hash(&content1), JsonArtifact::hash(&content2));
    }

    #[test]
    fn json_parser_extensions() {
        let parser = JsonParser;
        assert_eq!(parser.extensions(), &["json"]);
    }
}
