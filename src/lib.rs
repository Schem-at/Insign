use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod lexer;
pub mod parser;
pub mod ast;
pub mod pipeline;

/// A pair of 3D coordinates representing an axis-aligned bounding box.
/// Both corners are inclusive, and the box should be normalized (min <= max per axis).
pub type BoxPair = ([i32; 3], [i32; 3]);

/// Entry in the DSL output map, containing bounding boxes and metadata for a region.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DslEntry {
    /// Optional bounding boxes for this region. None for special entries like $global.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounding_boxes: Option<Vec<BoxPair>>,
    /// Metadata key-value pairs attached to this region.
    pub metadata: BTreeMap<String, serde_json::Value>,
}

/// The complete DSL compilation output: a map from region IDs to their entries.
/// Uses BTreeMap for deterministic ordering of keys.
pub type DslMap = BTreeMap<String, DslEntry>;

/// Errors that can occur during DSL compilation.
#[derive(Error, Debug)]
pub enum Error {
    #[error("Parser error: {0}")]
    Parser(#[from] ParseError),
    
    #[error("Not implemented yet")]
    NotImplemented,
}

/// Specific parse errors with location information.
#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Expected {expected} at position {position}, but found '{found}'")]
    Expected {
        expected: &'static str,
        found: String,
        position: usize,
    },
    
    #[error("Expected {expected} at position {position}, but reached end of input")]
    UnexpectedEnd {
        expected: &'static str,
        position: usize,
    },
    
    #[error("Invalid integer at position {position}: {source}")]
    InvalidInteger {
        position: usize,
        source: std::num::ParseIntError,
    },
    
    #[error("Invalid vec3 at position {position}: {message}")]
    InvalidVec3 {
        position: usize,
        message: String,
    },
    
    #[error("Invalid box at position {position}: {message}")]
    InvalidBox {
        position: usize,
        message: String,
    },
    
    #[error("Phase 0 supports only '+' operator at position {position}. Found '{operator}'")]
    UnsupportedOperator {
        position: usize,
        operator: String,
    },
    
    #[error("Boolean operator '{operator}' at position {position} requires 'boolean_ops' feature. TODO: Full boolean operations not implemented yet.")]
    FeatureGated {
        position: usize,
        operator: String,
    },
    
    #[error("Empty expression at position {position}")]
    EmptyExpression {
        position: usize,
    },
    
    #[error("Region '{region}' cannot be both accumulator and defined. Accumulator sources: {accumulator_sources:?}, defined source: {defined_source:?}")]
    MixedRegionMode {
        region: String,
        accumulator_sources: Vec<crate::ast::SourceLocation>,
        defined_source: crate::ast::SourceLocation,
    },
    
    #[error("Region '{region}' defined multiple times. First at {first_source:?}, duplicate at {duplicate_source:?}")]
    DuplicateRegionDefinition {
        region: String,
        first_source: crate::ast::SourceLocation,
        duplicate_source: crate::ast::SourceLocation,
    },
    
    #[error("Internal error: {message}")]
    Internal {
        message: String,
        position: usize,
    },
    
    #[error("Unknown region '{region}' referenced in expression at {source:?}")]
    UnknownRegion {
        region: String,
        source: crate::ast::SourceLocation,
    },
    
    #[error("Self-reference detected: region '{region}' references itself at {source:?}")]
    SelfReference {
        region: String,
        source: crate::ast::SourceLocation,
    },
    
    #[error("Cycle detected in region dependencies: {cycle:?}")]
    CycleDetected {
        cycle: Vec<String>,
    },
    
    #[error("Metadata conflict for region '{region}' key '{key}': different values across tuples. First: {first_value} at {first_source}, Conflict: {conflict_value} at {conflict_source}")]
    MetadataConflict {
        region: String,
        key: String,
        first_value: serde_json::Value,
        first_source: crate::ast::SourceLocation,
        conflict_value: serde_json::Value,
        conflict_source: crate::ast::SourceLocation,
    },
    
    #[error("No current region found for metadata statement at {source}. Hint: Metadata statements like '#key=value' must be placed after a geometry statement (@rc, @ac, or @region=expr) in the same tuple.")]
    NoCurrentRegion {
        source: crate::ast::SourceLocation,
    },
}

/// Compile DSL input units into a structured region map.
/// 
/// # Arguments
/// * `units` - Array of (position, text) tuples where position is [x,y,z] and text contains DSL statements
/// 
/// # Returns
/// A `DslMap` containing regions with their bounding boxes and metadata, or an error.
pub fn compile(units: &[([i32; 3], String)]) -> Result<DslMap, Error> {
    pipeline::compile_pipeline(units)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{self, json};

    #[test]
    fn test_empty_dsl_map_serialization() {
        let empty_map = DslMap::new();
        let json = serde_json::to_string(&empty_map).unwrap();
        assert_eq!(json, "{}");
        
        // Round-trip test
        let deserialized: DslMap = serde_json::from_str(&json).unwrap();
        assert_eq!(empty_map, deserialized);
    }
    
    #[test]
    fn test_minimal_dsl_map_serialization() {
        let mut map = DslMap::new();
        
        // Add a minimal entry
        let entry = DslEntry {
            bounding_boxes: Some(vec![([0, 0, 0], [1, 1, 1])]),
            metadata: BTreeMap::new(),
        };
        map.insert("test_region".to_string(), entry);
        
        let json = serde_json::to_string_pretty(&map).unwrap();
        
        // Round-trip test
        let deserialized: DslMap = serde_json::from_str(&json).unwrap();
        assert_eq!(map, deserialized);
    }
    
    #[test]
    fn test_deterministic_key_ordering() {
        let mut map = DslMap::new();
        
        // Insert keys in non-alphabetical order
        let keys = vec!["zebra", "apple", "banana"];
        for key in keys {
            let entry = DslEntry {
                bounding_boxes: None,
                metadata: BTreeMap::new(),
            };
            map.insert(key.to_string(), entry);
        }
        
        let json = serde_json::to_string(&map).unwrap();
        
        // BTreeMap should maintain alphabetical ordering
        assert!(json.find("apple").unwrap() < json.find("banana").unwrap());
        assert!(json.find("banana").unwrap() < json.find("zebra").unwrap());
    }
    
    #[test]
    fn test_compile_empty_input() {
        let result = compile(&[]);
        assert!(result.is_ok());
        let map = result.unwrap();
        assert!(map.is_empty());
    }
    
    #[test]
    fn test_compile_stub_returns_ok() {
        let units = vec![
            ([10, 64, 10], "@rc([0,0,0],[3,2,1])\n#doc.label=\"Patch A\"".to_string()),
        ];
        let result = compile(&units);
        assert!(result.is_ok());
        // For now, just verify it doesn't crash - full implementation in later milestones
    }
    
    #[test]
    fn test_compile_cpu_cache_region_name() {
        // Test the specific case that was failing: region name containing 'rc' substring
        let units = vec![
            ([0, 0, 0], "@cpu.cache=rc([0,0,0],[1,1,1])\n#doc.label=\"CPU Cache\"".to_string()),
        ];
        let result = compile(&units);
        assert!(result.is_ok(), "Should parse region names containing 'rc' or 'ac' substrings");
        
        let dsl_map = result.unwrap();
        // The region should be present in the output
        assert!(dsl_map.contains_key("cpu.cache"), "cpu.cache region should be present in output");
    }
    
    #[test]
    fn test_m8_complete_end_to_end() {
        // Test M8: Output Shaping - comprehensive end-to-end test
        let units = vec![
            ([10, 64, 10], "@rc([0,0,0],[3,2,1])\n#doc.label=\"Patch A\"".to_string()),
            ([0, 0, 0], "@cpu.cache=rc([0,0,0],[1,1,1])".to_string()),
            ([0, 0, 0], "#cpu.*:power=\"low\"".to_string()),
            ([0, 0, 0], "#$global:version=\"1.0\"".to_string()),
        ];
        
        let result = compile(&units);
        assert!(result.is_ok(), "M8 compilation should succeed");
        
        let dsl_map = result.unwrap();
        
        // Should include all expected entries
        assert!(dsl_map.contains_key("$global"), "Should have $global entry");
        assert!(dsl_map.contains_key("__anon_0_0"), "Should have anonymous region with metadata");
        assert!(dsl_map.contains_key("cpu.*"), "Should have wildcard entry");
        assert!(dsl_map.contains_key("cpu.cache"), "Should have named region");
        
        // Test $global entry (no boxes, has metadata)
        let global_entry = dsl_map.get("$global").unwrap();
        assert_eq!(global_entry.bounding_boxes, None);
        assert_eq!(global_entry.metadata["version"], json!("1.0"));
        
        // Test anonymous region (has boxes and metadata - should be included)
        let anon_entry = dsl_map.get("__anon_0_0").unwrap();
        assert!(anon_entry.bounding_boxes.is_some());
        assert_eq!(anon_entry.metadata["doc.label"], json!("Patch A"));
        
        // Test wildcard entry (no boxes, has metadata)
        let wildcard_entry = dsl_map.get("cpu.*").unwrap();
        assert_eq!(wildcard_entry.bounding_boxes, None);
        assert_eq!(wildcard_entry.metadata["power"], json!("low"));
        
        // Test named region (has boxes, inherited metadata from wildcard)
        let cache_entry = dsl_map.get("cpu.cache").unwrap();
        assert!(cache_entry.bounding_boxes.is_some());
        assert_eq!(cache_entry.metadata["power"], json!("low"));
        
        // Verify deterministic ordering (lexicographic via BTreeMap)
        let keys: Vec<&String> = dsl_map.keys().collect();
        assert_eq!(keys, vec![
            &"$global".to_string(),
            &"__anon_0_0".to_string(), 
            &"cpu.*".to_string(),
            &"cpu.cache".to_string(),
        ]);
    }
}

// M12: Feature Gate for Booleans (Phase 1 stub)
#[cfg(test)]
mod m12_feature_gate {
    use super::*;
    
    #[test]
    #[cfg(not(feature = "boolean_ops"))]
    fn test_boolean_ops_disabled_by_default_minus() {
        // Without feature flag, should get UnsupportedOperator error
        let units = vec![
            ([0, 0, 0], "@result=a-b".to_string()),
        ];
        
        let result = compile(&units);
        assert!(result.is_err());
        
        let error_message = format!("{}", result.unwrap_err());
        assert!(error_message.contains("Phase 0 supports only '+' operator"));
        assert!(error_message.contains("Found '-'"));
        assert!(!error_message.contains("boolean_ops"));
        assert!(!error_message.contains("TODO"));
    }
    
    #[test]
    #[cfg(not(feature = "boolean_ops"))]
    fn test_boolean_ops_disabled_by_default_and() {
        let units = vec![
            ([0, 0, 0], "@result=a&b".to_string()),
        ];
        
        let result = compile(&units);
        assert!(result.is_err());
        
        let error_message = format!("{}", result.unwrap_err());
        assert!(error_message.contains("Phase 0 supports only '+' operator"));
        assert!(error_message.contains("Found '&'"));
        assert!(!error_message.contains("boolean_ops"));
        assert!(!error_message.contains("TODO"));
    }
    
    #[test]
    #[cfg(not(feature = "boolean_ops"))]
    fn test_boolean_ops_disabled_by_default_xor() {
        let units = vec![
            ([0, 0, 0], "@result=a^b".to_string()),
        ];
        
        let result = compile(&units);
        assert!(result.is_err());
        
        let error_message = format!("{}", result.unwrap_err());
        assert!(error_message.contains("Phase 0 supports only '+' operator"));
        assert!(error_message.contains("Found '^'"));
        assert!(!error_message.contains("boolean_ops"));
        assert!(!error_message.contains("TODO"));
    }
}

// M9: Diagnostics & UX - Snapshot Tests
#[cfg(test)]
mod m9_diagnostics {
    use super::*;
    use insta::assert_snapshot;
    
    #[test]
    fn test_mixed_mode_error() {
        // Test case: region used as both accumulator and defined expression
        let units = vec![
            ([0, 0, 0], "@cpu=rc([0,0,0],[1,1,1])".to_string()),     // Accumulator usage
            ([0, 0, 0], "@cpu=base+ext".to_string()),                 // Defined usage - conflict!
        ];
        
        let result = compile(&units);
        assert!(result.is_err());
        
        let error_message = format!("{}", result.unwrap_err());
        assert_snapshot!("mixed_mode_error", error_message);
    }
    
    #[test]
    fn test_unknown_region_error() {
        // Test case: expression references non-existent region
        let units = vec![
            ([0, 0, 0], "@result=nonexistent+alsomissing".to_string()),
        ];
        
        let result = compile(&units);
        assert!(result.is_err());
        
        let error_message = format!("{}", result.unwrap_err());
        assert_snapshot!("unknown_region_error", error_message);
    }
    
    #[test]
    fn test_no_current_region_error() {
        // Test case: metadata without prior geometry in tuple
        let units = vec![
            ([0, 0, 0], "#label=\"No region to attach to\"".to_string()),
        ];
        
        let result = compile(&units);
        assert!(result.is_err());
        
        let error_message = format!("{}", result.unwrap_err());
        assert_snapshot!("no_current_region_error", error_message);
    }
    
    #[test]
    #[cfg(not(feature = "boolean_ops"))]
    fn test_disallowed_boolean_operators() {
        // Test case: Phase 0 disallowed operators
        let test_cases = vec![
            ("subtraction", "@result=a-b"),
            ("intersection", "@result=a&b"),
            ("xor", "@result=a^b"),
        ];
        
        for (operator_name, input) in test_cases {
            let units = vec![
                ([0, 0, 0], input.to_string()),
            ];
            
            let result = compile(&units);
            assert!(result.is_err());
            
            let error_message = format!("{}", result.unwrap_err());
            assert_snapshot!(format!("disallowed_operator_{}", operator_name), error_message);
        }
    }
}
