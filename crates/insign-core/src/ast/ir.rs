use std::collections::BTreeMap;
use crate::{BoxPair, ParseError};
use crate::ast::{GeomStmt, MetaStmt, BooleanExpr};
use crate::parser::geom::GeometryStatement;

/// Source location information for error reporting
#[derive(Debug, Clone, PartialEq)]
pub struct SourceLocation {
    pub tuple_idx: usize,
    pub stmt_idx: usize,
}

impl std::fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "tuple {} statement {}", self.tuple_idx, self.stmt_idx)
    }
}

impl std::error::Error for SourceLocation {}

impl SourceLocation {
    pub fn new(tuple_idx: usize, stmt_idx: usize) -> Self {
        Self { tuple_idx, stmt_idx }
    }
}

/// Types of region entries in the intermediate representation
#[derive(Debug, Clone, PartialEq)]
pub enum RegionEntry {
    /// Accumulator region: collects multiple geometry statements into boxes
    Accumulator {
        /// Direct box pairs from rc/ac statements
        boxes: Vec<BoxPair>,
        /// Source locations of all contributing statements
        sources: Vec<SourceLocation>,
    },
    /// Defined region: has a single boolean expression
    Defined {
        /// The boolean expression
        expr: BooleanExpr,
        /// Source location of the defining statement
        source: SourceLocation,
    },
    /// Anonymous region: unnamed geometry statement
    Anonymous {
        /// The single box pair
        box_pair: BoxPair,
        /// Source location
        source: SourceLocation,
    },
}

impl RegionEntry {
    /// Get all source locations for this region entry
    pub fn sources(&self) -> Vec<&SourceLocation> {
        match self {
            RegionEntry::Accumulator { sources, .. } => sources.iter().collect(),
            RegionEntry::Defined { source, .. } | RegionEntry::Anonymous { source, .. } => vec![source],
        }
    }
}

/// Intermediate representation table mapping region keys to entries
#[derive(Debug, Clone, PartialEq)]
pub struct RegionTable {
    /// Map from region key to region entry
    pub regions: BTreeMap<String, RegionEntry>,
}

impl RegionTable {
    /// Create a new empty region table
    pub fn new() -> Self {
        Self {
            regions: BTreeMap::new(),
        }
    }
    
    /// Add a geometry statement to the region table
    pub fn add_geometry(&mut self, stmt: &GeomStmt, offset: [i32; 3]) -> Result<(), ParseError> {
        let source = SourceLocation::new(stmt.tuple_idx, stmt.stmt_idx);
        
        match &stmt.statement {
            GeometryStatement::Expression { region, expr } => {
                // This is a defined region
                self.add_defined_region(region.clone(), expr.clone(), source)?;
            },
            geom_stmt => {
                // This is an accumulator or anonymous region
                let box_pair = geom_stmt.to_box_pair(offset).ok_or_else(|| ParseError::Internal {
                    message: "Geometry statement should produce a box pair".to_string(),
                    position: 0,
                })?;
                
                if let Some(region) = stmt.region() {
                    // Named accumulator region
                    self.add_accumulator_box(region.to_string(), box_pair, source)?;
                } else {
                    // Anonymous region
                    let key = stmt.anonymous_key();
                    self.add_anonymous_region(key, box_pair, source);
                }
            }
        }
        
        Ok(())
    }
    
    /// Add a defined region with boolean expression
    fn add_defined_region(&mut self, region: String, expr: BooleanExpr, source: SourceLocation) -> Result<(), ParseError> {
        match self.regions.get(&region) {
            Some(RegionEntry::Accumulator { sources, .. }) => {
                // Conflict: region is both accumulator and defined
                return Err(ParseError::MixedRegionMode(Box::new(crate::MixedRegionModeError {
                    region,
                    accumulator_sources: sources.clone(),
                    defined_source: source,
                })));
            },
            Some(RegionEntry::Defined { source: existing_source, .. }) => {
                // Multiple definitions - use the first one found
                return Err(ParseError::DuplicateRegionDefinition(Box::new(crate::DuplicateRegionDefinitionError {
                    region,
                    first_source: existing_source.clone(),
                    duplicate_source: source,
                })));
            },
            Some(RegionEntry::Anonymous { .. }) => {
                // Should not happen - anonymous regions use generated keys
                return Err(ParseError::Internal {
                    message: "Anonymous region with named key".to_string(),
                    position: 0,
                });
            },
            None => {
                // New defined region
                self.regions.insert(region, RegionEntry::Defined { expr, source });
            }
        }
        Ok(())
    }
    
    /// Add a box to an accumulator region
    fn add_accumulator_box(&mut self, region: String, box_pair: BoxPair, source: SourceLocation) -> Result<(), ParseError> {
        match self.regions.get_mut(&region) {
            Some(RegionEntry::Accumulator { boxes, sources }) => {
                // Add to existing accumulator
                boxes.push(box_pair);
                sources.push(source);
            },
            Some(RegionEntry::Defined { source: defined_source, .. }) => {
                // Conflict: region is both defined and accumulator
                return Err(ParseError::MixedRegionMode(Box::new(crate::MixedRegionModeError {
                    region,
                    accumulator_sources: vec![source],
                    defined_source: defined_source.clone(),
                })));
            },
            Some(RegionEntry::Anonymous { .. }) => {
                // Should not happen - anonymous regions use generated keys
                return Err(ParseError::Internal {
                    message: "Anonymous region with named key".to_string(),
                    position: 0,
                });
            },
            None => {
                // New accumulator region
                self.regions.insert(region, RegionEntry::Accumulator {
                    boxes: vec![box_pair],
                    sources: vec![source],
                });
            }
        }
        Ok(())
    }
    
    /// Add an anonymous region
    fn add_anonymous_region(&mut self, key: String, box_pair: BoxPair, source: SourceLocation) {
        // Anonymous regions should never conflict since they use generated keys
        self.regions.insert(key, RegionEntry::Anonymous { box_pair, source });
    }
}

impl Default for RegionTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Assemble a RegionTable from parsed geometry and metadata statements
pub fn assemble_region_table(
    geom_stmts: Vec<GeomStmt>,
    _meta_stmts: Vec<MetaStmt>, // TODO: Handle metadata in M7
    units: &[([i32; 3], String)],
) -> Result<RegionTable, ParseError> {
    let mut table = RegionTable::new();
    
    // Add all geometry statements
    for stmt in geom_stmts {
        // Get the offset for this tuple
        let offset = units.get(stmt.tuple_idx)
            .map(|(pos, _)| *pos)
            .unwrap_or([0, 0, 0]); // Default offset if tuple_idx is out of bounds
        
        table.add_geometry(&stmt, offset)?;
    }
    
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::geom::GeometryStatement;
    use crate::ast::BooleanExpr;

    /// Helper to create a geometry statement
    fn make_geom_stmt(tuple_idx: usize, stmt_idx: usize, statement: GeometryStatement) -> GeomStmt {
        GeomStmt::new(tuple_idx, stmt_idx, statement)
    }
    
    /// Helper to create a RelativeCoordinate statement
    fn make_rc(region: Option<String>, corners: ([i32; 3], [i32; 3])) -> GeometryStatement {
        GeometryStatement::RelativeCoordinate { region, corners }
    }
    
    /// Helper to create an AbsoluteCoordinate statement
    fn make_ac(region: Option<String>, corners: ([i32; 3], [i32; 3])) -> GeometryStatement {
        GeometryStatement::AbsoluteCoordinate { region, corners }
    }
    
    /// Helper to create an Expression statement
    fn make_expr(region: String, expr: BooleanExpr) -> GeometryStatement {
        GeometryStatement::Expression { region, expr }
    }
    
    #[test]
    fn test_accumulate_multiple_boxes() {
        let mut table = RegionTable::new();
        
        // Add first box to region "test"
        let stmt1 = make_geom_stmt(0, 0, make_rc(Some("test".to_string()), ([0, 0, 0], [1, 1, 1])));
        table.add_geometry(&stmt1, [10, 20, 30]).unwrap();
        
        // Add second box to same region
        let stmt2 = make_geom_stmt(0, 1, make_ac(Some("test".to_string()), ([5, 5, 5], [6, 6, 6])));
        table.add_geometry(&stmt2, [0, 0, 0]).unwrap();
        
        // Check that we have an accumulator with both boxes
        assert_eq!(table.regions.len(), 1);
        match table.regions.get("test").unwrap() {
            RegionEntry::Accumulator { boxes, sources } => {
                assert_eq!(boxes.len(), 2);
                assert_eq!(sources.len(), 2);
                
                // First box should be offset: [0+10, 0+20, 0+30] to [1+10, 1+20, 1+30]
                assert_eq!(boxes[0], ([10, 20, 30], [11, 21, 31]));
                // Second box should not be offset (ac)
                assert_eq!(boxes[1], ([5, 5, 5], [6, 6, 6]));
                
                // Check source locations
                assert_eq!(sources[0], SourceLocation::new(0, 0));
                assert_eq!(sources[1], SourceLocation::new(0, 1));
            }
            _ => panic!("Expected accumulator region"),
        }
    }
    
    #[test]
    fn test_mixed_region_mode_error() {
        let mut table = RegionTable::new();
        
        // Add accumulator box first
        let stmt1 = make_geom_stmt(0, 0, make_rc(Some("test".to_string()), ([0, 0, 0], [1, 1, 1])));
        table.add_geometry(&stmt1, [0, 0, 0]).unwrap();
        
        // Try to add expression to same region - should fail
        let expr = BooleanExpr::RegionRef("other".to_string());
        let stmt2 = make_geom_stmt(1, 0, make_expr("test".to_string(), expr));
        let result = table.add_geometry(&stmt2, [0, 0, 0]);
        
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::MixedRegionMode(err) => {
                assert_eq!(err.region, "test");
                assert_eq!(err.accumulator_sources.len(), 1);
                assert_eq!(err.accumulator_sources[0], SourceLocation::new(0, 0));
                assert_eq!(err.defined_source, SourceLocation::new(1, 0));
            }
            _ => panic!("Expected MixedRegionMode error"),
        }
    }
    
    #[test]
    fn test_mixed_region_mode_reverse_order() {
        let mut table = RegionTable::new();
        
        // Add expression first
        let expr = BooleanExpr::RegionRef("other".to_string());
        let stmt1 = make_geom_stmt(0, 0, make_expr("test".to_string(), expr));
        table.add_geometry(&stmt1, [0, 0, 0]).unwrap();
        
        // Try to add accumulator box to same region - should fail
        let stmt2 = make_geom_stmt(1, 0, make_rc(Some("test".to_string()), ([0, 0, 0], [1, 1, 1])));
        let result = table.add_geometry(&stmt2, [0, 0, 0]);
        
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::MixedRegionMode(err) => {
                assert_eq!(err.region, "test");
                assert_eq!(err.accumulator_sources.len(), 1);
                assert_eq!(err.accumulator_sources[0], SourceLocation::new(1, 0));
                assert_eq!(err.defined_source, SourceLocation::new(0, 0));
            }
            _ => panic!("Expected MixedRegionMode error"),
        }
    }
    
    #[test]
    fn test_anonymous_region_stability() {
        let mut table = RegionTable::new();
        
        // Add anonymous geometry statements
        let stmt1 = make_geom_stmt(0, 0, make_rc(None, ([0, 0, 0], [1, 1, 1])));
        let stmt2 = make_geom_stmt(0, 1, make_ac(None, ([5, 5, 5], [6, 6, 6])));
        let stmt3 = make_geom_stmt(1, 0, make_rc(None, ([10, 10, 10], [11, 11, 11])));
        
        table.add_geometry(&stmt1, [0, 0, 0]).unwrap();
        table.add_geometry(&stmt2, [0, 0, 0]).unwrap();
        table.add_geometry(&stmt3, [0, 0, 0]).unwrap();
        
        // Check that we have three separate anonymous regions with stable keys
        assert_eq!(table.regions.len(), 3);
        
        let keys: Vec<_> = table.regions.keys().collect();
        assert!(keys.contains(&&"__anon_0_0".to_string()));
        assert!(keys.contains(&&"__anon_0_1".to_string()));
        assert!(keys.contains(&&"__anon_1_0".to_string()));
        
        // Check the content of each anonymous region
        match table.regions.get("__anon_0_0").unwrap() {
            RegionEntry::Anonymous { box_pair, source } => {
                assert_eq!(*box_pair, ([0, 0, 0], [1, 1, 1]));
                assert_eq!(*source, SourceLocation::new(0, 0));
            }
            _ => panic!("Expected anonymous region"),
        }
        
        match table.regions.get("__anon_0_1").unwrap() {
            RegionEntry::Anonymous { box_pair, source } => {
                assert_eq!(*box_pair, ([5, 5, 5], [6, 6, 6]));
                assert_eq!(*source, SourceLocation::new(0, 1));
            }
            _ => panic!("Expected anonymous region"),
        }
        
        match table.regions.get("__anon_1_0").unwrap() {
            RegionEntry::Anonymous { box_pair, source } => {
                assert_eq!(*box_pair, ([10, 10, 10], [11, 11, 11]));
                assert_eq!(*source, SourceLocation::new(1, 0));
            }
            _ => panic!("Expected anonymous region"),
        }
    }
    
    #[test]
    fn test_assemble_region_table() {
        let units = vec![
            ([10, 20, 30], "@test=rc([0,0,0],[1,1,1])".to_string()),
            ([5, 10, 15], "@ac([0,0,0],[2,2,2])".to_string()),
        ];
        
        let geom_stmts = vec![
            make_geom_stmt(0, 0, make_rc(Some("test".to_string()), ([0, 0, 0], [1, 1, 1]))),
            make_geom_stmt(1, 0, make_ac(None, ([0, 0, 0], [2, 2, 2]))),
        ];
        
        let table = assemble_region_table(geom_stmts, vec![], &units).unwrap();
        
        assert_eq!(table.regions.len(), 2);
        
        // Check accumulator region "test" with offset applied
        match table.regions.get("test").unwrap() {
            RegionEntry::Accumulator { boxes, .. } => {
                assert_eq!(boxes.len(), 1);
                assert_eq!(boxes[0], ([10, 20, 30], [11, 21, 31]));
            }
            _ => panic!("Expected accumulator region"),
        }
        
        // Check anonymous region without offset (ac)
        match table.regions.get("__anon_1_0").unwrap() {
            RegionEntry::Anonymous { box_pair, .. } => {
                assert_eq!(*box_pair, ([0, 0, 0], [2, 2, 2]));
            }
            _ => panic!("Expected anonymous region"),
        }
    }
    
    #[test]
    fn test_duplicate_region_definition() {
        let mut table = RegionTable::new();
        
        // Add first expression definition
        let expr1 = BooleanExpr::RegionRef("other1".to_string());
        let stmt1 = make_geom_stmt(0, 0, make_expr("test".to_string(), expr1));
        table.add_geometry(&stmt1, [0, 0, 0]).unwrap();
        
        // Try to add second expression definition - should fail
        let expr2 = BooleanExpr::RegionRef("other2".to_string());
        let stmt2 = make_geom_stmt(1, 0, make_expr("test".to_string(), expr2));
        let result = table.add_geometry(&stmt2, [0, 0, 0]);
        
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::DuplicateRegionDefinition(err) => {
                assert_eq!(err.region, "test");
                assert_eq!(err.first_source, SourceLocation::new(0, 0));
                assert_eq!(err.duplicate_source, SourceLocation::new(1, 0));
            }
            _ => panic!("Expected DuplicateRegionDefinition error"),
        }
    }
}
