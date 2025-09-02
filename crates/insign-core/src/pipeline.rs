use crate::ast::{
    apply_metadata_pass, assemble_region_table, evaluate_geometry, shape_final_output,
    EvaluatedRegionTable, GeomStmt, MetaStmt,
};
use crate::lexer::{filter_comments, split_statements};
use crate::parser::geom::GeometryParser;
use crate::parser::meta::MetadataParser;
use crate::{DslMap, Error, ParseError};
use std::collections::BTreeMap;

/// Parse all statements from a tuple's text into geometry and metadata statements
fn parse_tuple_statements(
    tuple_idx: usize,
    text: &str,
) -> Result<(Vec<GeomStmt>, Vec<MetaStmt>), ParseError> {
    // Filter out comments before processing
    let filtered_text = filter_comments(text);
    let statement_slices = split_statements(&filtered_text);

    let mut geom_stmts = Vec::new();
    let mut meta_stmts = Vec::new();

    for (stmt_idx, statement_slice) in statement_slices.iter().enumerate() {
        let stmt_text = statement_slice.text.trim();
        if stmt_text.is_empty() {
            continue;
        }

        if stmt_text.starts_with('@') {
            // Geometry statement
            let mut geom_parser = GeometryParser::new(stmt_text);
            let parsed_stmt = geom_parser.parse()?;
            geom_stmts.push(GeomStmt::new(tuple_idx, stmt_idx, parsed_stmt));
        } else if stmt_text.starts_with('#') {
            // Metadata statement
            let mut meta_parser = MetadataParser::new(stmt_text);
            let parsed_stmt = meta_parser.parse()?;
            meta_stmts.push(MetaStmt::new(tuple_idx, stmt_idx, parsed_stmt));
        }
        // Skip any other statements (shouldn't happen with proper lexer)
    }

    Ok((geom_stmts, meta_stmts))
}

/// Complete compilation pipeline from input units to final DslMap
pub fn compile_pipeline(units: &[([i32; 3], String)]) -> Result<DslMap, Error> {
    if units.is_empty() {
        return Ok(BTreeMap::new());
    }

    // Step 1: Parse all statements from all tuples
    let mut all_geom_stmts = Vec::new();
    let mut all_meta_stmts = Vec::new();

    for (tuple_idx, (_position, text)) in units.iter().enumerate() {
        let (geom_stmts, meta_stmts) = parse_tuple_statements(tuple_idx, text)?;
        all_geom_stmts.extend(geom_stmts);
        all_meta_stmts.extend(meta_stmts);
    }

    // Step 2: Assemble RegionTable from geometry statements
    let region_table =
        assemble_region_table(all_geom_stmts.clone(), all_meta_stmts.clone(), units)?;

    // Step 3: Evaluate geometry to get bounding boxes
    let evaluated_boxes = evaluate_geometry(&region_table)?;

    // Step 4: Build EvaluatedRegionTable with boxes
    let mut evaluated_table = EvaluatedRegionTable::new();
    for (region_name, boxes) in evaluated_boxes {
        evaluated_table.set_region_boxes(region_name, Some(boxes));
    }

    // Step 5: Apply metadata pass
    apply_metadata_pass(&mut evaluated_table, &all_geom_stmts, &all_meta_stmts)?;

    // Step 6: Convert to DslMap format
    let dsl_map = crate::ast::metadata::build_dsl_map(evaluated_table);

    // Step 7: Apply final output shaping (ordering and filtering)
    let final_map = shape_final_output(dsl_map);

    Ok(final_map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_simple_compilation_pipeline() {
        let units = vec![
            ([10, 20, 30], "@test=rc([0,0,0],[1,1,1])".to_string()),
            ([0, 0, 0], "#test:label=\"Test Region\"".to_string()),
        ];

        let result = compile_pipeline(&units);
        assert!(result.is_ok());

        let dsl_map = result.unwrap();
        assert_eq!(dsl_map.len(), 1);

        let test_entry = dsl_map.get("test").unwrap();

        // Should have the box with offset applied
        let boxes = test_entry.bounding_boxes.as_ref().unwrap();
        assert_eq!(boxes.len(), 1);
        assert_eq!(boxes[0], ([10, 20, 30], [11, 21, 31]));

        // Should have the metadata
        assert_eq!(test_entry.metadata["label"], json!("Test Region"));
    }

    #[test]
    fn test_compilation_with_anonymous_and_global() {
        let units = vec![
            (
                [0, 0, 0],
                "@rc([0,0,0],[1,1,1])\n#label=\"Anonymous with metadata\"".to_string(),
            ),
            ([5, 5, 5], "@rc([0,0,0],[2,2,2])".to_string()), // Anonymous without metadata
            ([0, 0, 0], "#$global:version=\"1.0\"".to_string()),
            ([0, 0, 0], "@named=ac([10,10,10],[11,11,11])".to_string()),
        ];

        let result = compile_pipeline(&units);
        assert!(result.is_ok());

        let dsl_map = result.unwrap();
        let keys: Vec<&String> = dsl_map.keys().collect();

        // Should be ordered: lexicographically by BTreeMap
        // Anonymous without metadata should be filtered out
        assert_eq!(
            keys,
            vec![
                &"$global".to_string(),    // Lexicographically first
                &"__anon_0_0".to_string(), // Anonymous before 'n'
                &"named".to_string(),      // Named region
            ]
        );

        // Check $global
        let global_entry = dsl_map.get("$global").unwrap();
        assert_eq!(global_entry.bounding_boxes, None);
        assert_eq!(global_entry.metadata["version"], json!("1.0"));

        // Check named region
        let named_entry = dsl_map.get("named").unwrap();
        let boxes = named_entry.bounding_boxes.as_ref().unwrap();
        assert_eq!(boxes[0], ([10, 10, 10], [11, 11, 11])); // ac, no offset

        // Check anonymous with metadata
        let anon_entry = dsl_map.get("__anon_0_0").unwrap();
        let anon_boxes = anon_entry.bounding_boxes.as_ref().unwrap();
        assert_eq!(anon_boxes[0], ([0, 0, 0], [1, 1, 1])); // rc with [0,0,0] offset
        assert_eq!(
            anon_entry.metadata["label"],
            json!("Anonymous with metadata")
        );

        // Verify __anon_1_0 (no metadata) was filtered out
        assert!(!dsl_map.contains_key("__anon_1_0"));
    }

    #[test]
    fn test_compilation_with_expression() {
        let units = vec![
            ([0, 0, 0], "@base=rc([0,0,0],[1,1,1])".to_string()),
            ([5, 5, 5], "@ext=rc([0,0,0],[2,2,2])".to_string()),
            ([0, 0, 0], "@combined=base+ext".to_string()),
            ([0, 0, 0], "#combined:type=\"union\"".to_string()),
        ];

        let result = compile_pipeline(&units);
        assert!(result.is_ok());

        let dsl_map = result.unwrap();
        assert_eq!(dsl_map.len(), 3); // base, ext, combined

        // Check combined region has union of boxes
        let combined_entry = dsl_map.get("combined").unwrap();
        let boxes = combined_entry.bounding_boxes.as_ref().unwrap();
        assert_eq!(boxes.len(), 2); // Union of base + ext
        assert_eq!(combined_entry.metadata["type"], json!("union"));
    }

    #[test]
    fn test_compilation_with_wildcards() {
        let units = vec![
            ([0, 0, 0], "@cpu.core=rc([0,0,0],[1,1,1])".to_string()),
            ([0, 0, 0], "@cpu.cache=rc([2,2,2],[3,3,3])".to_string()),
            ([0, 0, 0], "@gpu.core=rc([4,4,4],[5,5,5])".to_string()),
            ([0, 0, 0], "#cpu.*:power=\"low\"".to_string()),
        ];

        let result = compile_pipeline(&units);
        if let Err(e) = &result {
            eprintln!("Compilation failed with error: {:?}", e);
        }
        assert!(result.is_ok());

        let dsl_map = result.unwrap();
        let keys: Vec<&String> = dsl_map.keys().collect();

        // Should be ordered: wildcards, then regions
        assert_eq!(
            keys,
            vec![
                &"cpu.*".to_string(),
                &"cpu.cache".to_string(),
                &"cpu.core".to_string(),
                &"gpu.core".to_string(),
            ]
        );

        // cpu.* should be a wildcard region with no boxes
        let cpu_wildcard = dsl_map.get("cpu.*").unwrap();
        assert_eq!(cpu_wildcard.bounding_boxes, None);
        assert_eq!(cpu_wildcard.metadata["power"], json!("low"));

        // cpu.core and cpu.cache should also have the power metadata
        assert_eq!(
            dsl_map.get("cpu.core").unwrap().metadata["power"],
            json!("low")
        );
        assert_eq!(
            dsl_map.get("cpu.cache").unwrap().metadata["power"],
            json!("low")
        );

        // gpu.core should not have the power metadata
        assert!(!dsl_map
            .get("gpu.core")
            .unwrap()
            .metadata
            .contains_key("power"));
    }

    #[test]
    fn test_empty_compilation() {
        let units = vec![];
        let result = compile_pipeline(&units);
        assert!(result.is_ok());

        let dsl_map = result.unwrap();
        assert!(dsl_map.is_empty());
    }

    #[test]
    fn test_compilation_error_propagation() {
        // Test with conflicting metadata
        let units = vec![
            (
                [0, 0, 0],
                "@test=rc([0,0,0],[1,1,1])\n#label=\"First\"".to_string(),
            ),
            (
                [5, 5, 5],
                "@test=rc([2,2,2],[3,3,3])\n#label=\"Second\"".to_string(),
            ), // Conflict!
        ];

        let result = compile_pipeline(&units);
        assert!(result.is_err());

        match result.unwrap_err() {
            Error::Parser(ParseError::MetadataConflict { .. }) => {
                // Expected error type
            }
            other => panic!("Unexpected error type: {:?}", other),
        }
    }

    #[test]
    fn test_comment_filtering_in_pipeline() {
        // Test that comments are properly filtered out during compilation
        let units = vec![
            (
                [0, 0, 0],
                "; This is a comment\n@test=rc([0,0,0],[1,1,1])\n; Another comment".to_string(),
            ),
            (
                [0, 0, 0],
                "; Comment before metadata\n#test:label=\"Test Region\"\n; Comment after"
                    .to_string(),
            ),
        ];

        let result = compile_pipeline(&units);
        assert!(result.is_ok());

        let dsl_map = result.unwrap();
        assert_eq!(dsl_map.len(), 1);

        let test_entry = dsl_map.get("test").unwrap();

        // Should have the box
        let boxes = test_entry.bounding_boxes.as_ref().unwrap();
        assert_eq!(boxes.len(), 1);
        assert_eq!(boxes[0], ([0, 0, 0], [1, 1, 1]));

        // Should have the metadata
        assert_eq!(test_entry.metadata["label"], json!("Test Region"));
    }

    #[test]
    fn test_mixed_statements_with_comments() {
        // Test complex mixing of statements and comments
        let units = vec![
            (
                [0, 0, 0],
                "; Define a base region\n@base=rc([0,0,0],[1,1,1])\n; End base definition"
                    .to_string(),
            ),
            (
                [5, 5, 5],
                "; Another region\n@ext=rc([0,0,0],[2,2,2])\n; Extension complete".to_string(),
            ),
            (
                [0, 0, 0],
                "; Combine regions\n@combined=base+ext\n; Combined region created".to_string(),
            ),
            (
                [0, 0, 0],
                "; Add metadata\n#combined:type=\"union\"\n; Metadata added".to_string(),
            ),
        ];

        let result = compile_pipeline(&units);
        assert!(result.is_ok());

        let dsl_map = result.unwrap();
        assert_eq!(dsl_map.len(), 3); // base, ext, combined

        // Check combined region has union of boxes
        let combined_entry = dsl_map.get("combined").unwrap();
        let boxes = combined_entry.bounding_boxes.as_ref().unwrap();
        assert_eq!(boxes.len(), 2); // Union of base + ext
        assert_eq!(combined_entry.metadata["type"], json!("union"));
    }

    #[test]
    fn test_empty_lines_and_comments() {
        // Test that empty lines and comments don't break compilation
        let units = vec![
            ([0, 0, 0], "; Comment only".to_string()),
            (
                [0, 0, 0],
                "\n; Empty line above\n\n@test=rc([0,0,0],[1,1,1])\n\n; Empty lines around"
                    .to_string(),
            ),
            (
                [0, 0, 0],
                "\n\n; Just comments and empty lines\n\n".to_string(),
            ),
        ];

        let result = compile_pipeline(&units);
        assert!(result.is_ok());

        let dsl_map = result.unwrap();
        assert_eq!(dsl_map.len(), 1);

        let test_entry = dsl_map.get("test").unwrap();
        let boxes = test_entry.bounding_boxes.as_ref().unwrap();
        assert_eq!(boxes.len(), 1);
        assert_eq!(boxes[0], ([0, 0, 0], [1, 1, 1]));
    }
}
