use std::collections::BTreeMap;
use serde_json::Value;
use crate::{BoxPair, ParseError, DslEntry};
use crate::ast::{GeomStmt, MetaStmt, SourceLocation};
use crate::parser::meta::MetadataStatement;

/// Metadata assignment with source tracking
#[derive(Debug, Clone, PartialEq)]
pub struct MetadataAssignment {
    /// The metadata value
    pub value: Value,
    /// Source location of this assignment
    pub source: SourceLocation,
}

/// Table of evaluated regions with their boxes and metadata
#[derive(Debug, Clone, PartialEq)]
pub struct EvaluatedRegionTable {
    /// Map from region name to boxes and metadata
    pub regions: BTreeMap<String, EvaluatedRegionData>,
}

/// Data for an evaluated region
#[derive(Debug, Clone, PartialEq)]
pub struct EvaluatedRegionData {
    /// Bounding boxes for this region (None for special entries like $global)
    pub boxes: Option<Vec<BoxPair>>,
    /// Metadata assignments for this region
    pub metadata: BTreeMap<String, MetadataAssignment>,
}

impl EvaluatedRegionTable {
    /// Create a new empty evaluated region table
    pub fn new() -> Self {
        Self {
            regions: BTreeMap::new(),
        }
    }
    
    /// Add or update a region's bounding boxes
    pub fn set_region_boxes(&mut self, region: String, boxes: Option<Vec<BoxPair>>) {
        let entry = self.regions.entry(region).or_insert_with(|| EvaluatedRegionData {
            boxes: None,
            metadata: BTreeMap::new(),
        });
        entry.boxes = boxes;
    }
    
    /// Add metadata to a region, checking for conflicts
    pub fn add_metadata(&mut self, region: String, key: String, assignment: MetadataAssignment) -> Result<(), ParseError> {
        let entry = self.regions.entry(region.clone()).or_insert_with(|| EvaluatedRegionData {
            boxes: None,
            metadata: BTreeMap::new(),
        });
        
        // Check for existing metadata with different value
        if let Some(existing) = entry.metadata.get(&key) {
            if existing.value != assignment.value {
                return Err(ParseError::MetadataConflict(Box::new(crate::MetadataConflictError {
                    region,
                    key,
                    first_value: existing.value.clone(),
                    first_source: existing.source.clone(),
                    conflict_value: assignment.value,
                    conflict_source: assignment.source,
                })));
            }
            // Identical values are allowed - just keep the existing one
        } else {
            // New metadata key
            entry.metadata.insert(key, assignment);
        }
        
        Ok(())
    }
}

impl Default for EvaluatedRegionTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Find the last geometry statement in a given tuple
fn find_last_geometry_in_tuple(geom_stmts: &[GeomStmt], tuple_idx: usize) -> Option<String> {
    let mut last_region: Option<String> = None;
    
    for stmt in geom_stmts {
        if stmt.tuple_idx == tuple_idx {
            if let Some(region) = stmt.region() {
                last_region = Some(region.to_string());
            } else {
                // Anonymous region
                last_region = Some(stmt.anonymous_key());
            }
        }
    }
    
    last_region
}

/// Determine if a target is a wildcard pattern
fn is_wildcard_pattern(target: &str) -> bool {
    target.contains('*')
}

/// Check if a region name matches a wildcard pattern
fn matches_wildcard(region_name: &str, pattern: &str) -> bool {
    // Simple wildcard matching: patterns like "cpu.*" match "cpu.core", "cpu.cache", etc.
    if pattern.ends_with('*') {
        let prefix = &pattern[..pattern.len() - 1];
        region_name.starts_with(prefix)
    } else if pattern.starts_with('*') {
        let suffix = &pattern[1..];
        region_name.ends_with(suffix)
    } else {
        // Exact match (no wildcard)
        region_name == pattern
    }
}

/// Process metadata statements and apply them to the evaluated region table
pub fn apply_metadata_pass(
    evaluated_table: &mut EvaluatedRegionTable,
    geom_stmts: &[GeomStmt],
    meta_stmts: &[MetaStmt],
) -> Result<(), ParseError> {
    
    for meta_stmt in meta_stmts {
        let source = SourceLocation::new(meta_stmt.tuple_idx, meta_stmt.stmt_idx);
        
        match &meta_stmt.statement {
            MetadataStatement::Current { key, value } => {
                // Find the last geometry statement in the same tuple
                let target_region = find_last_geometry_in_tuple(geom_stmts, meta_stmt.tuple_idx)
                    .ok_or_else(|| ParseError::NoCurrentRegion { source: source.clone() })?;
                
                let assignment = MetadataAssignment {
                    value: value.clone(),
                    source: source.clone(),
                };
                
                evaluated_table.add_metadata(target_region, key.clone(), assignment)?;
            },
            MetadataStatement::Targeted { target, key, value } => {
                let assignment = MetadataAssignment {
                    value: value.clone(),
                    source: source.clone(),
                };
                
                if is_wildcard_pattern(target) {
                    // First, create the wildcard region entry itself
                    evaluated_table.add_metadata(target.clone(), key.clone(), assignment.clone())?;
                    
                    // Then apply to all matching regions
                    let matching_regions: Vec<String> = evaluated_table.regions.keys()
                        .filter(|region| matches_wildcard(region, target) && *region != target)
                        .cloned()
                        .collect();
                    
                    for region in matching_regions {
                        evaluated_table.add_metadata(region, key.clone(), assignment.clone())?;
                    }
                } else {
                    // Direct target
                    evaluated_table.add_metadata(target.clone(), key.clone(), assignment)?;
                }
            }
        }
    }
    
    Ok(())
}

/// Convert an EvaluatedRegionTable to the final DslMap format
pub fn build_dsl_map(evaluated_table: EvaluatedRegionTable) -> BTreeMap<String, DslEntry> {
    let mut dsl_map = BTreeMap::new();
    
    for (region_name, region_data) in evaluated_table.regions {
        // Convert metadata assignments to simple key-value pairs
        let metadata: BTreeMap<String, Value> = region_data.metadata
            .into_iter()
            .map(|(key, assignment)| (key, assignment.value))
            .collect();
        
        // Skip anonymous regions without metadata
        let is_anonymous = region_name.starts_with("__anon_");
        if is_anonymous && metadata.is_empty() {
            continue;
        }
        
        let entry = DslEntry {
            bounding_boxes: region_data.boxes,
            metadata,
        };
        
        dsl_map.insert(region_name, entry);
    }
    
    dsl_map
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{GeomStmt, MetaStmt};
    use crate::parser::geom::GeometryStatement;
    use crate::parser::meta::MetadataStatement;
    use serde_json::json;
    
    /// Helper to create a geometry statement
    fn make_geom_stmt(tuple_idx: usize, stmt_idx: usize, statement: GeometryStatement) -> GeomStmt {
        GeomStmt::new(tuple_idx, stmt_idx, statement)
    }
    
    /// Helper to create a metadata statement
    fn make_meta_stmt(tuple_idx: usize, stmt_idx: usize, statement: MetadataStatement) -> MetaStmt {
        MetaStmt::new(tuple_idx, stmt_idx, statement)
    }
    
    /// Helper to create a RelativeCoordinate statement
    fn make_rc(region: Option<String>, corners: ([i32; 3], [i32; 3])) -> GeometryStatement {
        GeometryStatement::RelativeCoordinate { region, corners }
    }
    
    #[test]
    fn test_find_last_geometry_in_tuple() {
        let geom_stmts = vec![
            make_geom_stmt(0, 0, make_rc(Some("first".to_string()), ([0, 0, 0], [1, 1, 1]))),
            make_geom_stmt(0, 1, make_rc(Some("second".to_string()), ([2, 2, 2], [3, 3, 3]))),
            make_geom_stmt(1, 0, make_rc(Some("third".to_string()), ([4, 4, 4], [5, 5, 5]))),
        ];
        
        assert_eq!(find_last_geometry_in_tuple(&geom_stmts, 0), Some("second".to_string()));
        assert_eq!(find_last_geometry_in_tuple(&geom_stmts, 1), Some("third".to_string()));
        assert_eq!(find_last_geometry_in_tuple(&geom_stmts, 2), None);
    }
    
    #[test]
    fn test_find_last_geometry_anonymous() {
        let geom_stmts = vec![
            make_geom_stmt(0, 0, make_rc(None, ([0, 0, 0], [1, 1, 1]))),
            make_geom_stmt(0, 1, make_rc(Some("named".to_string()), ([2, 2, 2], [3, 3, 3]))),
            make_geom_stmt(0, 2, make_rc(None, ([4, 4, 4], [5, 5, 5]))),
        ];
        
        // Should find the last anonymous region
        assert_eq!(find_last_geometry_in_tuple(&geom_stmts, 0), Some("__anon_0_2".to_string()));
    }
    
    #[test]
    fn test_wildcard_matching() {
        assert!(matches_wildcard("cpu.core", "cpu.*"));
        assert!(matches_wildcard("cpu.cache", "cpu.*"));
        assert!(!matches_wildcard("gpu.core", "cpu.*"));
        assert!(!matches_wildcard("cpu", "cpu.*"));
        
        assert!(matches_wildcard("core.cpu", "*.cpu"));
        assert!(matches_wildcard("cache.cpu", "*.cpu"));
        assert!(!matches_wildcard("core.gpu", "*.cpu"));
        
        assert!(matches_wildcard("exact", "exact"));
        assert!(!matches_wildcard("exact2", "exact"));
    }
    
    #[test]
    fn test_current_region_metadata() {
        let mut evaluated_table = EvaluatedRegionTable::new();
        
        // Add a region with boxes
        evaluated_table.set_region_boxes("test_region".to_string(), Some(vec![([0, 0, 0], [1, 1, 1])]));
        
        let geom_stmts = vec![
            make_geom_stmt(0, 0, make_rc(Some("test_region".to_string()), ([0, 0, 0], [1, 1, 1]))),
        ];
        
        let meta_stmts = vec![
            make_meta_stmt(0, 1, MetadataStatement::Current {
                key: "label".to_string(),
                value: json!("Test Label"),
            }),
        ];
        
        apply_metadata_pass(&mut evaluated_table, &geom_stmts, &meta_stmts).unwrap();
        
        let region_data = evaluated_table.regions.get("test_region").unwrap();
        assert_eq!(region_data.metadata.len(), 1);
        assert_eq!(region_data.metadata["label"].value, json!("Test Label"));
    }
    
    #[test]
    fn test_explicit_target_metadata() {
        let mut evaluated_table = EvaluatedRegionTable::new();
        
        let geom_stmts = vec![];
        
        let meta_stmts = vec![
            make_meta_stmt(0, 0, MetadataStatement::Targeted {
                target: "new_region".to_string(),
                key: "type".to_string(),
                value: json!("special"),
            }),
        ];
        
        apply_metadata_pass(&mut evaluated_table, &geom_stmts, &meta_stmts).unwrap();
        
        // Should create new region with empty boxes
        let region_data = evaluated_table.regions.get("new_region").unwrap();
        assert_eq!(region_data.boxes, None);
        assert_eq!(region_data.metadata.len(), 1);
        assert_eq!(region_data.metadata["type"].value, json!("special"));
    }
    
    #[test]
    fn test_metadata_conflict_detection() {
        let mut evaluated_table = EvaluatedRegionTable::new();
        
        let geom_stmts = vec![
            make_geom_stmt(0, 0, make_rc(Some("test".to_string()), ([0, 0, 0], [1, 1, 1]))),
            make_geom_stmt(1, 0, make_rc(Some("test".to_string()), ([2, 2, 2], [3, 3, 3]))),
        ];
        
        let meta_stmts = vec![
            make_meta_stmt(0, 1, MetadataStatement::Current {
                key: "label".to_string(),
                value: json!("First"),
            }),
            make_meta_stmt(1, 1, MetadataStatement::Current {
                key: "label".to_string(),
                value: json!("Second"), // Conflict!
            }),
        ];
        
        let result = apply_metadata_pass(&mut evaluated_table, &geom_stmts, &meta_stmts);
        
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::MetadataConflict(err) => {
                assert_eq!(err.region, "test");
                assert_eq!(err.key, "label");
                assert_eq!(err.first_value, json!("First"));
                assert_eq!(err.conflict_value, json!("Second"));
            },
            _ => panic!("Expected MetadataConflict error"),
        }
    }
    
    #[test]
    fn test_identical_duplicate_allowed() {
        let mut evaluated_table = EvaluatedRegionTable::new();
        
        let geom_stmts = vec![
            make_geom_stmt(0, 0, make_rc(Some("test".to_string()), ([0, 0, 0], [1, 1, 1]))),
            make_geom_stmt(1, 0, make_rc(Some("test".to_string()), ([2, 2, 2], [3, 3, 3]))),
        ];
        
        let meta_stmts = vec![
            make_meta_stmt(0, 1, MetadataStatement::Current {
                key: "label".to_string(),
                value: json!("Same"),
            }),
            make_meta_stmt(1, 1, MetadataStatement::Current {
                key: "label".to_string(),
                value: json!("Same"), // Identical - should be OK
            }),
        ];
        
        let result = apply_metadata_pass(&mut evaluated_table, &geom_stmts, &meta_stmts);
        assert!(result.is_ok());
        
        let region_data = evaluated_table.regions.get("test").unwrap();
        assert_eq!(region_data.metadata["label"].value, json!("Same"));
    }
    
    #[test]
    fn test_wildcard_metadata() {
        let mut evaluated_table = EvaluatedRegionTable::new();
        
        // Add some regions
        evaluated_table.set_region_boxes("cpu.core".to_string(), Some(vec![([0, 0, 0], [1, 1, 1])]));
        evaluated_table.set_region_boxes("cpu.cache".to_string(), Some(vec![([2, 2, 2], [3, 3, 3])]));
        evaluated_table.set_region_boxes("gpu.core".to_string(), Some(vec![([4, 4, 4], [5, 5, 5])]));
        
        let geom_stmts = vec![];
        
        let meta_stmts = vec![
            make_meta_stmt(0, 0, MetadataStatement::Targeted {
                target: "cpu.*".to_string(),
                key: "power".to_string(),
                value: json!("low"),
            }),
        ];
        
        apply_metadata_pass(&mut evaluated_table, &geom_stmts, &meta_stmts).unwrap();
        
        // Should create the wildcard region entry itself
        assert!(evaluated_table.regions.contains_key("cpu.*"));
        assert_eq!(evaluated_table.regions["cpu.*"].boxes, None); // Wildcard entries have no boxes
        assert_eq!(evaluated_table.regions["cpu.*"].metadata["power"].value, json!("low"));
        
        // Should apply to cpu.core and cpu.cache but not gpu.core
        assert_eq!(evaluated_table.regions["cpu.core"].metadata["power"].value, json!("low"));
        assert_eq!(evaluated_table.regions["cpu.cache"].metadata["power"].value, json!("low"));
        assert!(!evaluated_table.regions["gpu.core"].metadata.contains_key("power"));
    }
    
    #[test]
    fn test_global_metadata() {
        let mut evaluated_table = EvaluatedRegionTable::new();
        
        let geom_stmts = vec![];
        
        let meta_stmts = vec![
            make_meta_stmt(0, 0, MetadataStatement::Targeted {
                target: "$global".to_string(),
                key: "version".to_string(),
                value: json!("1.0"),
            }),
        ];
        
        apply_metadata_pass(&mut evaluated_table, &geom_stmts, &meta_stmts).unwrap();
        
        // Should create $global region
        let global_data = evaluated_table.regions.get("$global").unwrap();
        assert_eq!(global_data.boxes, None); // Global has no boxes
        assert_eq!(global_data.metadata["version"].value, json!("1.0"));
    }
    
    #[test]
    fn test_build_dsl_map_excludes_empty_anonymous() {
        let mut evaluated_table = EvaluatedRegionTable::new();
        
        // Add anonymous region without metadata
        evaluated_table.set_region_boxes("__anon_0_0".to_string(), Some(vec![([0, 0, 0], [1, 1, 1])]));
        
        // Add anonymous region with metadata  
        evaluated_table.set_region_boxes("__anon_0_1".to_string(), Some(vec![([2, 2, 2], [3, 3, 3])]));
        evaluated_table.add_metadata("__anon_0_1".to_string(), "label".to_string(), MetadataAssignment {
            value: json!("labeled"),
            source: SourceLocation::new(0, 2),
        }).unwrap();
        
        // Add named region
        evaluated_table.set_region_boxes("named".to_string(), Some(vec![([4, 4, 4], [5, 5, 5])]));
        
        let dsl_map = build_dsl_map(evaluated_table);
        
        // Should exclude __anon_0_0 but include __anon_0_1 and named
        assert!(!dsl_map.contains_key("__anon_0_0"));
        assert!(dsl_map.contains_key("__anon_0_1"));
        assert!(dsl_map.contains_key("named"));
    }
    
    #[test]
    fn test_no_current_region_error() {
        let mut evaluated_table = EvaluatedRegionTable::new();
        
        let geom_stmts = vec![]; // No geometry statements
        
        let meta_stmts = vec![
            make_meta_stmt(0, 0, MetadataStatement::Current {
                key: "label".to_string(),
                value: json!("orphan"),
            }),
        ];
        
        let result = apply_metadata_pass(&mut evaluated_table, &geom_stmts, &meta_stmts);
        
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::NoCurrentRegion { source } => {
                assert_eq!(source.tuple_idx, 0);
                assert_eq!(source.stmt_idx, 0);
            },
            _ => panic!("Expected NoCurrentRegion error"),
        }
    }
}
