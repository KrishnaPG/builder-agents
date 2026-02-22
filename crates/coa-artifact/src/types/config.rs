//! Config Artifact Type
//!
//! Schema-validated configuration with JSON/YAML support.

use std::collections::HashMap;

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value as JsonValue;

use crate::artifact_type::{ArtifactContent, ArtifactType};
use crate::hash::ContentHash;

/// Config artifact type marker
///
/// Uses JSON as the canonical representation for hashing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigArtifact;

impl ArtifactType for ConfigArtifact {
    type Content = ConfigContent;

    #[inline]
    fn hash(content: &Self::Content) -> ContentHash {
        // Use canonical JSON representation for consistent hashing
        content.canonical_hash()
    }

    const TYPE_ID: &'static str = "config";
}

/// Configuration content
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigContent {
    /// Parsed JSON value
    value: JsonValue,

    /// Schema reference (if any)
    schema: Option<String>,

    /// Cached hash
    hash: ContentHash,
}

impl ConfigContent {
    /// Create from JSON value
    #[inline]
    pub fn new(value: JsonValue) -> Self {
        let hash = Self::compute_hash(&value);
        Self {
            value,
            schema: None,
            hash,
        }
    }

    /// Parse from JSON string
    ///
    /// # Errors
    /// Returns error if JSON is invalid
    #[inline]
    pub fn from_json(json: &str) -> Result<Self, ConfigError> {
        let value: JsonValue = serde_json::from_str(json).map_err(ConfigError::InvalidJson)?;
        Ok(Self::new(value))
    }

    /// Parse from YAML string
    ///
    /// # Errors
    /// Returns error if YAML is invalid
    pub fn from_yaml(yaml: &str) -> Result<Self, ConfigError> {
        let value: JsonValue = serde_yaml::from_str(yaml).map_err(ConfigError::InvalidYaml)?;
        Ok(Self::new(value))
    }

    /// Get JSON value reference
    #[inline]
    #[must_use]
    pub fn value(&self) -> &JsonValue {
        &self.value
    }

    /// Convert to typed struct
    ///
    /// # Errors
    /// Returns error if value doesn't match type
    #[inline]
    pub fn to_typed<T: DeserializeOwned>(&self) -> Result<T, ConfigError> {
        serde_json::from_value(self.value.clone()).map_err(ConfigError::InvalidJson)
    }

    /// Create from typed struct
    #[inline]
    pub fn from_typed<T: Serialize>(value: &T) -> Result<Self, ConfigError> {
        let json = serde_json::to_value(value).map_err(|e| ConfigError::Serialization(e.to_string()))?;
        Ok(Self::new(json))
    }

    /// Get a value by JSON pointer
    ///
    /// # Examples
    /// ```
    /// # use coa_artifact::types::config::ConfigContent;
    /// # use serde_json::json;
    /// let content = ConfigContent::new(json!({"database": {"host": "localhost"}}));
    /// assert_eq!(content.get("/database/host"), Some(&json!("localhost")));
    /// ```
    #[inline]
    #[must_use]
    pub fn get(&self, pointer: &str) -> Option<&JsonValue> {
        self.value.pointer(pointer)
    }

    /// Set a value by JSON pointer
    ///
    /// Returns new ConfigContent with updated value.
    #[inline]
    #[must_use]
    pub fn set(mut self, pointer: &str, new_value: JsonValue) -> Self {
        if let Some(target) = self.value.pointer_mut(pointer) {
            *target = new_value;
        }
        // Recompute hash
        self.hash = Self::compute_hash(&self.value);
        self
    }

    /// Merge with another config
    ///
    /// Returns new ConfigContent with merged values.
    /// Objects are deep-merged, arrays are concatenated.
    #[inline]
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        let merged = merge_json(&self.value, &other.value);
        Self::new(merged)
    }

    /// Check if config has schema
    #[inline]
    #[must_use]
    pub fn has_schema(&self) -> bool {
        self.schema.is_some()
    }

    /// Get schema reference
    #[inline]
    #[must_use]
    pub fn schema(&self) -> Option<&str> {
        self.schema.as_deref()
    }

    /// Set schema reference
    #[inline]
    #[must_use]
    pub fn with_schema(mut self, schema: impl Into<String>) -> Self {
        self.schema = Some(schema.into());
        self
    }

    /// Get canonical JSON string
    #[inline]
    #[must_use]
    pub fn to_canonical_json(&self) -> String {
        // Sort keys for canonical representation
        canonical_json(&self.value)
    }

    /// Get hash
    #[inline]
    #[must_use]
    pub fn hash(&self) -> ContentHash {
        self.hash
    }

    /// Compute canonical hash
    #[inline]
    fn compute_hash(value: &JsonValue) -> ContentHash {
        let canonical = canonical_json(value);
        ContentHash::compute(canonical.as_bytes())
    }

    /// Canonical hash accessor
    #[inline]
    fn canonical_hash(&self) -> ContentHash {
        self.hash
    }

    /// Serialize to JSON string
    ///
    /// # Errors
    /// Returns error if serialization fails (rare for JSON)
    #[inline]
    pub fn to_json(&self) -> Result<String, ConfigError> {
        serde_json::to_string_pretty(&self.value)
            .map_err(|e| ConfigError::Serialization(e.to_string()))
    }

    /// Serialize to YAML string
    ///
    /// # Errors
    /// Returns error if serialization fails
    #[inline]
    pub fn to_yaml(&self) -> Result<String, ConfigError> {
        serde_yaml::to_string(&self.value)
            .map_err(|e| ConfigError::Serialization(e.to_string()))
    }
}

impl ArtifactContent for ConfigContent {
    #[inline]
    fn approximate_size(&self) -> usize {
        std::mem::size_of::<Self>() + self.to_canonical_json().len()
    }
}

impl Default for ConfigContent {
    fn default() -> Self {
        Self::new(JsonValue::Object(serde_json::Map::new()))
    }
}

impl From<JsonValue> for ConfigContent {
    fn from(value: JsonValue) -> Self {
        Self::new(value)
    }
}

impl TryFrom<ConfigContent> for JsonValue {
    type Error = ConfigError;

    fn try_from(content: ConfigContent) -> Result<Self, Self::Error> {
        Ok(content.value)
    }
}

/// Config error types
#[derive(Debug, Clone, thiserror::Error)]
pub enum ConfigError {
    #[error("invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    #[error("invalid YAML: {0}")]
    InvalidYaml(#[from] serde_yaml::Error),

    #[error("serialization failed: {0}")]
    Serialization(String),

    #[error("schema validation failed: {0}")]
    SchemaValidation(String),

    #[error("config not found: {0}")]
    NotFound(String),
}

/// Merge two JSON values (deep merge for objects)
fn merge_json(a: &JsonValue, b: &JsonValue) -> JsonValue {
    match (a, b) {
        (JsonValue::Object(a_map), JsonValue::Object(b_map)) => {
            let mut result = a_map.clone();
            for (key, b_val) in b_map {
                result.insert(
                    key.clone(),
                    if let Some(a_val) = result.get(key) {
                        merge_json(a_val, b_val)
                    } else {
                        b_val.clone()
                    },
                );
            }
            JsonValue::Object(result)
        }
        (JsonValue::Array(a_arr), JsonValue::Array(b_arr)) => {
            let mut result = a_arr.clone();
            result.extend(b_arr.iter().cloned());
            JsonValue::Array(result)
        }
        // For primitives, b wins
        (_, b_val) => b_val.clone(),
    }
}

/// Generate canonical JSON (sorted keys)
fn canonical_json(value: &JsonValue) -> String {
    match value {
        JsonValue::Object(map) => {
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();

            let mut parts = Vec::new();
            for key in keys {
                if let Some(val) = map.get(key) {
                    parts.push(format!("{}:{}", key, canonical_json(val)));
                }
            }
            format!("{{{}}}", parts.join(","))
        }
        JsonValue::Array(arr) => {
            let parts: Vec<_> = arr.iter().map(canonical_json).collect();
            format!("[{}]", parts.join(","))
        }
        JsonValue::String(s) => format!("\"{}\"", s),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Null => "null".to_string(),
    }
}

/// Config schema validation
pub mod validation {
    use super::*;

    /// Validate config against JSON schema
    ///
    /// # Errors
    /// Returns error if validation fails
    pub fn validate_schema(content: &ConfigContent, schema: &JsonValue) -> Result<(), ConfigError> {
        // Basic type validation (simplified)
        // In production, use jsonschema crate
        validate_value(&content.value, schema).map_err(ConfigError::SchemaValidation)
    }

    fn validate_value(value: &JsonValue, schema: &JsonValue) -> Result<(), String> {
        let schema_type = schema.get("type").and_then(|t| t.as_str());

        match schema_type {
            Some("object") => validate_object(value, schema),
            Some("array") => validate_array(value, schema),
            Some("string") => validate_string(value, schema),
            Some("number") => validate_number(value, schema),
            Some("integer") => validate_integer(value, schema),
            Some("boolean") => validate_boolean(value, schema),
            _ => Ok(()), // Unknown schema type, allow
        }
    }

    fn validate_object(value: &JsonValue, schema: &JsonValue) -> Result<(), String> {
        let obj = value.as_object().ok_or("expected object")?;

        if let Some(props) = schema.get("properties") {
            if let Some(props_obj) = props.as_object() {
                for (key, prop_schema) in props_obj {
                    if let Some(val) = obj.get(key) {
                        validate_value(val, prop_schema)?;
                    } else if schema.get("required").and_then(|r| r.as_array()).map_or(false, |req| req.iter().any(|v| v.as_str() == Some(key))) {
                        return Err(format!("missing required field: {}", key));
                    }
                }
            }
        }

        Ok(())
    }

    fn validate_array(value: &JsonValue, schema: &JsonValue) -> Result<(), String> {
        let arr = value.as_array().ok_or("expected array")?;

        if let Some(items_schema) = schema.get("items") {
            for (i, item) in arr.iter().enumerate() {
                validate_value(item, items_schema)
                    .map_err(|e| format!("item {}: {}", i, e))?;
            }
        }

        Ok(())
    }

    fn validate_string(value: &JsonValue, _schema: &JsonValue) -> Result<(), String> {
        if !value.is_string() {
            return Err("expected string".to_string());
        }
        Ok(())
    }

    fn validate_number(value: &JsonValue, _schema: &JsonValue) -> Result<(), String> {
        if !value.is_number() {
            return Err("expected number".to_string());
        }
        Ok(())
    }

    fn validate_integer(value: &JsonValue, _schema: &JsonValue) -> Result<(), String> {
        if !value.is_u64() && !value.is_i64() {
            return Err("expected integer".to_string());
        }
        Ok(())
    }

    fn validate_boolean(value: &JsonValue, _schema: &JsonValue) -> Result<(), String> {
        if !value.is_boolean() {
            return Err("expected boolean".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn config_from_json() {
        let content = ConfigContent::from_json(r#"{"name": "test", "value": 42}"#).unwrap();
        assert_eq!(content.get("/name"), Some(&json!("test")));
        assert_eq!(content.get("/value"), Some(&json!(42)));
    }

    #[test]
    fn config_from_typed() {
        #[derive(Serialize)]
        struct TestConfig {
            name: String,
            enabled: bool,
        }

        let config = TestConfig {
            name: "test".to_string(),
            enabled: true,
        };

        let content = ConfigContent::from_typed(&config).unwrap();
        assert_eq!(content.get("/name"), Some(&json!("test")));
        assert_eq!(content.get("/enabled"), Some(&json!(true)));
    }

    #[test]
    fn config_merge() {
        let a = ConfigContent::new(json!({"x": 1, "y": 2}));
        let b = ConfigContent::new(json!({"y": 3, "z": 4}));
        let merged = a.merge(&b);

        assert_eq!(merged.get("/x"), Some(&json!(1)));
        assert_eq!(merged.get("/y"), Some(&json!(3)));
        assert_eq!(merged.get("/z"), Some(&json!(4)));
    }

    #[test]
    fn config_set() {
        let content = ConfigContent::new(json!({"name": "old"}));
        let updated = content.set("/name", json!("new"));

        assert_eq!(updated.get("/name"), Some(&json!("new")));
    }

    #[test]
    fn config_canonical_hash_consistent() {
        let c1 = ConfigContent::from_json(r#"{"b": 1, "a": 2}"#).unwrap();
        let c2 = ConfigContent::from_json(r#"{"a": 2, "b": 1}"#).unwrap();

        // Different order, same content -> same hash
        assert_eq!(c1.hash(), c2.hash());
    }

    #[test]
    fn config_invalid_json() {
        let result = ConfigContent::from_json("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn config_to_typed() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct TestConfig {
            name: String,
        }

        let content = ConfigContent::new(json!({"name": "test"}));
        let typed: TestConfig = content.to_typed().unwrap();

        assert_eq!(typed.name, "test");
    }
}
