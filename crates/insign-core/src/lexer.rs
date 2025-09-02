/// A slice of the input text representing a single DSL statement.
#[derive(Debug, Clone, PartialEq)]
pub struct StatementSlice<'a> {
    /// The text content of the statement
    pub text: &'a str,
    /// Starting byte offset in the original input
    pub start: usize,
    /// Ending byte offset in the original input
    pub end: usize,
}

/// Filter out comment lines from input text.
/// Comments start with ';' and extend to the end of the line.
/// This preserves line numbers by replacing comments with empty lines.
pub fn filter_comments(input: &str) -> String {
    input
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with(';') {
                "" // Replace comment line with empty line
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Split input text into DSL statement slices.
/// 
/// Statements start with '@' or '#' only when:
/// - We're at depth 0 (not inside brackets/parentheses)
/// - We're not inside a JSON string literal
/// 
/// # Arguments
/// * `input` - The DSL text to split
/// 
/// # Returns
/// A vector of statement slices covering the entire input
pub fn split_statements(input: &str) -> Vec<StatementSlice> {
    if input.is_empty() {
        return Vec::new();
    }

    let mut statements = Vec::new();
    let mut current_start = 0;
    let mut depth = 0;
    let mut in_string = false;
    let mut escape_next = false;
    
    let mut byte_pos = 0;
    let chars: Vec<char> = input.chars().collect();
    
    for (char_idx, &ch) in chars.iter().enumerate() {
        // Handle escape sequences inside strings
        if in_string && escape_next {
            escape_next = false;
            byte_pos += ch.len_utf8();
            continue;
        }
        
        match ch {
            '\\' if in_string => {
                escape_next = true;
            },
            '"' => {
                in_string = !in_string;
                escape_next = false;
            },
            '(' | '[' | '{' if !in_string => {
                depth += 1;
            },
            ')' | ']' | '}' if !in_string => {
                depth -= 1;
            },
            '@' | '#' if depth == 0 && !in_string && char_idx > 0 => {
                // Found the start of a new statement
                // End the previous statement at the current byte position
                let text = &input[current_start..byte_pos];
                statements.push(StatementSlice {
                    text,
                    start: current_start,
                    end: byte_pos,
                });
                current_start = byte_pos;
            },
            _ => {
                escape_next = false;
            }
        }
        
        byte_pos += ch.len_utf8();
    }
    
    // Add the final statement
    if current_start < input.len() {
        let text = &input[current_start..];
        statements.push(StatementSlice {
            text,
            start: current_start,
            end: input.len(),
        });
    }
    
    statements
}


#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_single_statement() {
        let input = "@rc([0,1,2],[3,4,5])";
        let statements = split_statements(input);
        
        assert_eq!(statements.len(), 1);
        assert_eq!(statements[0].text, input);
        assert_eq!(statements[0].start, 0);
        assert_eq!(statements[0].end, input.len());
    }

    #[test]
    fn test_multiple_statements() {
        let input = "@rc([0,1,2],[3,4,5])\n#key=\"value\"";
        let statements = split_statements(input);
        
        assert_eq!(statements.len(), 2);
        assert_eq!(statements[0].text, "@rc([0,1,2],[3,4,5])\n");
        assert_eq!(statements[1].text, "#key=\"value\"");
    }

    #[test]
    fn test_nested_brackets() {
        let input = "@region=rc([0,0,0],[1,1,1])+ac([2,2,2],[3,3,3])";
        let statements = split_statements(input);
        
        assert_eq!(statements.len(), 1);
        assert_eq!(statements[0].text, input);
    }

    #[test]
    fn test_multiline_statement() {
        let input = "@dataloop.registers=rc([2,64,2],\n                       [12,69,6])\n                    + rc([14,64,2],\n                         [24,69,6])";
        let statements = split_statements(input);
        
        assert_eq!(statements.len(), 1);
        assert_eq!(statements[0].text, input);
    }

    #[test]
    fn test_json_strings_with_at_hash() {
        let input = "#doc.note=\"Contains @ and # symbols\"\n@rc([0,0,0],[1,1,1])";
        let statements = split_statements(input);
        
        assert_eq!(statements.len(), 2);
        assert_eq!(statements[0].text, "#doc.note=\"Contains @ and # symbols\"\n");
        assert_eq!(statements[1].text, "@rc([0,0,0],[1,1,1])");
    }

    #[test]
    fn test_mixed_geometry_metadata() {
        let input = "@cpu.core=ac([100,70,-20],[104,72,-18])\n#cpu.core:logic.clock_hz=4\n#cpu.*:power.budget=\"low\"";
        let statements = split_statements(input);
        
        assert_eq!(statements.len(), 3);
        assert_eq!(statements[0].text, "@cpu.core=ac([100,70,-20],[104,72,-18])\n");
        assert_eq!(statements[1].text, "#cpu.core:logic.clock_hz=4\n");
        assert_eq!(statements[2].text, "#cpu.*:power.budget=\"low\"");
    }

    #[test]
    fn test_escaped_quotes_in_json() {
        let input = "#doc.label=\"Quote: \\\"Hello World\\\"\"\n@rc([0,0,0],[1,1,1])";
        let statements = split_statements(input);
        
        assert_eq!(statements.len(), 2);
        assert_eq!(statements[0].text, "#doc.label=\"Quote: \\\"Hello World\\\"\"\n");
        assert_eq!(statements[1].text, "@rc([0,0,0],[1,1,1])");
    }

    #[test]
    fn test_empty_input() {
        let statements = split_statements("");
        assert_eq!(statements.len(), 0);
    }
    
    // M14: Comment Support Tests
    
    #[test]
    fn test_filter_comments_simple() {
        let input = "; This is a comment\n@rc([0,0,0],[1,1,1])\n; Another comment";
        let filtered = filter_comments(input);
        assert_eq!(filtered, "\n@rc([0,0,0],[1,1,1])\n");
    }
    
    #[test]
    fn test_filter_comments_with_whitespace() {
        let input = "  ; Indented comment\n@rc([0,0,0],[1,1,1])\n\t; Tab comment";
        let filtered = filter_comments(input);
        assert_eq!(filtered, "\n@rc([0,0,0],[1,1,1])\n");
    }
    
    #[test]
    fn test_filter_comments_mixed_with_statements() {
        let input = "; Comment at start\n@rc([1,1,1],[2,2,2])\n; Middle comment\n#doc.label=\"test\"\n; End comment";
        let filtered = filter_comments(input);
        assert_eq!(filtered, "\n@rc([1,1,1],[2,2,2])\n\n#doc.label=\"test\"\n");
    }
    
    #[test]
    fn test_statements_with_comments_filtered() {
        let input = "; This is a comment\n@rc([0,0,0],[1,1,1])\n; Another comment\n#key=\"value\"";
        let filtered = filter_comments(input);
        let statements = split_statements(&filtered);
        
        // After filtering, we expect: "\n@rc([0,0,0],[1,1,1])\n\n#key=\"value\""
        // This contains leading whitespace, the @-statement, whitespace, then #-statement
        // split_statements will split on @ and # but also preserve full coverage
        // So we expect 3 statements: whitespace, @-statement, #-statement
        assert_eq!(statements.len(), 3);
        // First statement is the leading whitespace
        assert!(statements[0].text.trim().is_empty());
        // Second statement contains the @-statement
        assert!(statements[1].text.trim().contains("@rc([0,0,0],[1,1,1])"));
        // Third statement contains the #-statement
        assert!(statements[2].text.trim().contains("#key=\"value\""));
    }
    
    #[test]
    fn test_comment_in_json_string_not_filtered() {
        let input = "#doc.note=\"This ; is not a comment\"";
        let filtered = filter_comments(input);
        assert_eq!(filtered, input); // Should be unchanged
        
        let statements = split_statements(&filtered);
        assert_eq!(statements.len(), 1);
        assert_eq!(statements[0].text, input);
    }
    
    #[test]
    fn test_empty_comment_lines() {
        let input = ";\n@rc([0,0,0],[1,1,1])\n;   \n#key=\"value\"";
        let filtered = filter_comments(input);
        let statements = split_statements(&filtered);
        
        // After filtering: "\n@rc([0,0,0],[1,1,1])\n\n#key=\"value\""
        // This splits into: empty line, @-statement with trailing newline, empty line + #-statement
        assert_eq!(statements.len(), 3);
        // First statement is empty/whitespace
        assert!(statements[0].text.trim().is_empty());
        // Second statement contains @-statement
        assert!(statements[1].text.trim().contains("@rc([0,0,0],[1,1,1])"));
        // Third statement contains #-statement
        assert!(statements[2].text.trim().contains("#key=\"value\""));
    }
    
    #[test]
    fn test_comment_only_input() {
        let input = "; Just a comment\n; Another comment\n; Third comment";
        let filtered = filter_comments(input);
        let statements = split_statements(&filtered);
        
        // After filtering: "\n\n" (two empty lines)
        // This should result in a single statement of whitespace
        assert_eq!(statements.len(), 1);
        assert!(statements[0].text.trim().is_empty());
    }

    #[test]
    fn test_no_slice_overlaps_full_coverage() {
        let input = "@rc([0,0,0],[1,1,1])\n#key=\"value\"\n@another=rc([2,2,2],[3,3,3])";
        let statements = split_statements(input);
        
        // Verify no overlaps and full coverage
        let mut covered_bytes = vec![false; input.len()];
        
        for stmt in &statements {
            // Check each byte in this statement's range
            for i in stmt.start..stmt.end {
                assert!(!covered_bytes[i], "Byte {} covered by multiple statements", i);
                covered_bytes[i] = true;
            }
        }
        
        // Check that all bytes are covered
        for (i, &covered) in covered_bytes.iter().enumerate() {
            assert!(covered, "Byte {} not covered by any statement", i);
        }
    }

    proptest! {
        #[test]
        fn test_lexer_coverage_invariant(input in "[^\\x00-\\x08\\x0B\\x0C\\x0E-\\x1F\\x7F]*") {
            let statements = split_statements(&input);
            
            if input.is_empty() {
                prop_assert_eq!(statements.len(), 0);
                return Ok(());
            }
            
            // Property: All statements combined should cover the entire input with no gaps or overlaps
            let mut covered_bytes = vec![false; input.len()];
            
            for stmt in &statements {
                // Each statement should have valid bounds
                prop_assert!(stmt.start < input.len());
                prop_assert!(stmt.end <= input.len());
                prop_assert!(stmt.start < stmt.end);
                
                // Mark bytes as covered, ensuring no overlaps
                for i in stmt.start..stmt.end {
                    prop_assert!(!covered_bytes[i], "Byte {} covered by multiple statements", i);
                    covered_bytes[i] = true;
                }
            }
            
            // All bytes should be covered
            for (i, &covered) in covered_bytes.iter().enumerate() {
                prop_assert!(covered, "Byte {} not covered by any statement", i);
            }
        }
        
        #[test]
        fn test_lexer_no_panic(input in "[^\\x00-\\x08\\x0B\\x0C\\x0E-\\x1F\\x7F]*") {
            // Property: The lexer should never panic on any valid UTF-8 input
            let _ = split_statements(&input);
        }
    }
}
