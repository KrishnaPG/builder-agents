//! YAML configuration parser
//!
//! Uses serde_yaml for robust YAML parsing with support for:
//! - Multi-document YAML
//! - Anchors and aliases
//! - Custom tags

use crate::error::ParseError;
use crate::parsers::ArtifactParser;
use coa_artifact::{Artifact, ArtifactType, ContentHash};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::collections::HashMap;

/// YAML configuration content
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct YamlContent {
    /// Documents in the YAML file (usually one)
    pub documents: Vec<Value>,
    /// Schema reference (optional, from $schema or %YAML directive)
    pub schema: Option<String>,
    /// Preserved comments by path
    pub comments: HashMap<String, String>,
}

impl YamlContent {
    /// Create from single document
    #[inline]
    #[must_use]
    pub fn new(value: Value) -> Self {
        Self {
            documents: vec![value],
            schema: None,
            comments: HashMap::new(),
        }
    }

    /// Create from multiple documents
    #[inline]
    #[must_use]
    pub fn new_multi(documents: Vec<Value>) -> Self {
        Self {
            documents,
            schema: None,
            comments: HashMap::new(),
        }
    }

    /// Get first (or only) document
    #[inline]
    #[must_use]
    pub fn first(&self) -> Option<&Value> {
        self.documents.first()
    }

    /// Get value at path in first document (dot notation)
    #[must_use]
    pub fn get_path(&self, path: &str) -> Option<&Value> {
        let doc = self.first()?;
        let mut current = doc;
        for segment in path.split('.') {
            match current {
                Value::Mapping(map) => {
                    current = map.get(&Value::String(segment.to_string()))?
                }
                _ => return None,
            }
        }
        Some(current)
    }

    /// Set value at path in first document
    pub fn set_path(&mut self, path: &str, value: Value) {
        if self.documents.is_empty() {
            self.documents.push(Value::Mapping(serde_yaml::Mapping::new()));
        }

        let doc = &mut self.documents[0];
        let segments: Vec<_> = path.split('.').collect();
        if segments.is_empty() {
            return;
        }

        let mut current = doc;
        for segment in &segments[..segments.len() - 1] {
            match current {
                Value::Mapping(map) => {
                    current = map
                        .entry(Value::String(segment.to_string()))
                        .or_insert_with(|| {
                            Value::Mapping(serde_yaml::Mapping::new())
                        });
                }
                _ => {
                    *current = Value::Mapping(serde_yaml::Mapping::new());
                    if let Value::Mapping(map) = current {
                        current = map
                            .entry(Value::String(segment.to_string()))
                            .or_insert_with(|| {
                                Value::Mapping(serde_yaml::Mapping::new())
                            });
                    }
                }
            }
        }

        if let Value::Mapping(map) = current {
            map.insert(
                Value::String(segments.last().unwrap().to_string()),
                value,
            );
        }
    }

    /// Merge another YAML content into this one
    ///
    /// Values from other take precedence.
    pub fn merge(&mut self, other: &YamlContent) {
        if let (Some(self_doc), Some(other_doc)) =
            (self.first().cloned(), other.first().cloned())
        {
            let merged = merge_values(self_doc, other_doc);
            if !self.documents.is_empty() {
                self.documents[0] = merged;
            } else {
                self.documents.push(merged);
            }
        }
    }
}

/// Merge two YAML values recursively
fn merge_values(base: Value, override_val: Value) -> Value {
    match (base, override_val) {
        (Value::Mapping(mut base_map), Value::Mapping(override_map)) => {
            for (key, value) in override_map {
                let entry = base_map.entry(key).or_insert(Value::Null);
                *entry = merge_values(entry.clone(), value);
            }
            Value::Mapping(base_map)
        }
        (_, override_val) => override_val,
    }
}

/// YAML config artifact type
#[derive(Debug, Clone)]
pub struct YamlArtifact;

impl coa_artifact::__private::Sealed for YamlArtifact {}

impl ArtifactType for YamlArtifact {
    type Content = YamlContent;

    fn hash(content: &Self::Content) -> ContentHash {
        let yaml_str =
            serde_yaml::to_string(&content.documents).unwrap_or_default();
        ContentHash::compute(yaml_str.as_bytes())
    }

    const TYPE_ID: &'static str = "yaml";
}

/// YAML parser
#[derive(Debug, Clone, Copy, Default)]
pub struct YamlParser;

impl YamlParser {
    /// Create new YAML parser
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl ArtifactParser for YamlParser {
    type Output = YamlArtifact;

    fn parse(&self, content: &str) -> Result<Artifact<Self::Output>, ParseError> {
        // Parse YAML documents
        let de = serde_yaml::Deserializer::from_str(content);
        let mut documents = Vec::new();
        
        for doc in de {
            let value = Value::deserialize(doc).map_err(|e| ParseError::SyntaxError {
                path: std::path::PathBuf::from("input.yaml"),
                message: format!("YAML parse error: {}", e),
            })?;
            documents.push(value);
        }

        if documents.is_empty()
            || documents.iter().all(|doc| matches!(doc, Value::Null))
        {
            return Err(ParseError::SyntaxError {
                path: std::path::PathBuf::from("input.yaml"),
                message: "empty YAML document".to_string(),
            });
        }

        // Extract schema from first document
        let schema = documents
            .first()
            .and_then(|doc: &Value| doc.get("$schema"))
            .and_then(|v| v.as_str())
            .map(|s: &str| s.to_string());

        // Create content
        let yaml_content = YamlContent {
            documents,
            schema,
            comments: HashMap::new(),
        };

        // Create artifact
        Artifact::new(yaml_content).map_err(|e| {
            ParseError::ValidationError(format!("artifact creation failed: {}", e))
        })
    }

    fn extensions(&self) -> &[&str] {
        &["yaml", "yml"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaml_parser_valid() {
        let parser = YamlParser;
        let content = r#"
name: test
value: 42
nested:
  key: value
"#;

        let result = parser.parse(content);
        assert!(result.is_ok());

        let artifact = result.unwrap();
        assert_eq!(artifact.content().documents.len(), 1);
    }

    #[test]
    fn yaml_parser_multi_document() {
        let parser = YamlParser;
        let content = r#"
---
name: doc1
---
name: doc2
"#;

        let result = parser.parse(content);
        assert!(result.is_ok());

        let artifact = result.unwrap();
        assert_eq!(artifact.content().documents.len(), 2);
    }

    #[test]
    fn yaml_parser_empty() {
        let parser = YamlParser;
        let content = "";

        let result = parser.parse(content);
        assert!(result.is_err()); // Empty is not valid YAML
    }

    #[test]
    fn yaml_content_get_path() {
        let value: Value = serde_yaml::from_str(r#"
server:
  host: localhost
  port: 8080
debug: true
"#).unwrap();
        let content = YamlContent::new(value);

        assert_eq!(
            content.get_path("server.host"),
            Some(&Value::String("localhost".to_string()))
        );
        assert_eq!(
            content.get_path("debug"),
            Some(&Value::Bool(true))
        );
        assert_eq!(content.get_path("missing"), None);
    }

    #[test]
    fn yaml_content_set_path() {
        let mut content = YamlContent::new(Value::Mapping(serde_yaml::Mapping::new()));

        content.set_path("server.port", Value::Number(8080.into()));
        assert_eq!(
            content.get_path("server.port"),
            Some(&Value::Number(8080.into()))
        );
    }

    #[test]
    fn yaml_merge() {
        let base: Value = serde_yaml::from_str(r#"
server:
  host: localhost
  port: 8080
debug: false
"#).unwrap();
        let mut base = YamlContent::new(base);

        let override_val: Value = serde_yaml::from_str(r#"
server:
  port: 9090
debug: true
"#).unwrap();
        let override_content = YamlContent::new(override_val);

        base.merge(&override_content);

        assert_eq!(
            base.get_path("server.host"),
            Some(&Value::String("localhost".to_string()))
        );
        assert_eq!(
            base.get_path("server.port"),
            Some(&Value::Number(9090.into()))
        );
        assert_eq!(base.get_path("debug"), Some(&Value::Bool(true)));
    }

    #[test]
    fn yaml_artifact_type_id() {
        assert_eq!(YamlArtifact::TYPE_ID, "yaml");
    }

    #[test]
    fn yaml_parser_extensions() {
        let parser = YamlParser;
        assert!(parser.extensions().contains(&"yaml"));
        assert!(parser.extensions().contains(&"yml"));
    }

    #[test]
    fn yaml_content_first() {
        let content = YamlContent::new(Value::String("test".to_string()));
        assert_eq!(content.first(), Some(&Value::String("test".to_string())));

        let empty = YamlContent::new_multi(vec![]);
        assert_eq!(empty.first(), None);
    }
}
