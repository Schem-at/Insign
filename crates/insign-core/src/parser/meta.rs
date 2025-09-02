use crate::parser::json_value::JsonValueParser;
use crate::ParseError;
use serde_json::Value;

/// Types of metadata statements
#[derive(Debug, Clone, PartialEq)]
pub enum MetadataStatement {
    /// Current region metadata: #key=<json>
    Current {
        /// The metadata key
        key: String,
        /// The JSON value
        value: Value,
    },
    /// Targeted metadata: #<target>:key=<json>
    Targeted {
        /// The target (region ID, wildcard, or $global)
        target: String,
        /// The metadata key
        key: String,
        /// The JSON value
        value: Value,
    },
}

/// Parser for metadata statements
pub struct MetadataParser<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> MetadataParser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, position: 0 }
    }

    /// Parse a metadata statement from the input
    pub fn parse(&mut self) -> Result<MetadataStatement, ParseError> {
        self.skip_whitespace();

        // Expect '#' at the start
        if !self.consume_char('#') {
            return Err(ParseError::Expected {
                expected: "'#'",
                found: self.current_char().unwrap_or('\0').to_string(),
                position: self.position,
            });
        }

        // Check if this is targeted metadata (contains ':')
        let _checkpoint = self.position;
        let target = self.parse_optional_target()?;

        if let Some(target) = target {
            // Targeted metadata: expect ':' after target
            self.skip_whitespace();
            if !self.consume_char(':') {
                return Err(ParseError::Expected {
                    expected: "':'",
                    found: self.current_char().unwrap_or('\0').to_string(),
                    position: self.position,
                });
            }
            self.skip_whitespace();

            // Parse key
            let key = self.parse_key()?;

            // Expect '='
            self.skip_whitespace();
            if !self.consume_char('=') {
                return Err(ParseError::Expected {
                    expected: "'='",
                    found: self.current_char().unwrap_or('\0').to_string(),
                    position: self.position,
                });
            }
            self.skip_whitespace();

            // Parse JSON value
            let remaining_input = &self.input[self.position..];
            let mut json_parser = JsonValueParser::new(remaining_input);
            let value = json_parser.parse()?;

            Ok(MetadataStatement::Targeted { target, key, value })
        } else {
            // Current region metadata
            let key = self.parse_key()?;

            // Expect '='
            self.skip_whitespace();
            if !self.consume_char('=') {
                return Err(ParseError::Expected {
                    expected: "'='",
                    found: self.current_char().unwrap_or('\0').to_string(),
                    position: self.position,
                });
            }
            self.skip_whitespace();

            // Parse JSON value
            let remaining_input = &self.input[self.position..];
            let mut json_parser = JsonValueParser::new(remaining_input);
            let value = json_parser.parse()?;

            Ok(MetadataStatement::Current { key, value })
        }
    }

    /// Parse an optional target (up to ':')
    fn parse_optional_target(&mut self) -> Result<Option<String>, ParseError> {
        let start_pos = self.position;

        // Look for characters that could be part of a target
        while let Some(ch) = self.current_char() {
            if ch == ':' {
                // Found ':', so we have a target
                let target = self.input[start_pos..self.position].trim().to_string();
                if target.is_empty() {
                    return Err(ParseError::Expected {
                        expected: "target name",
                        found: "empty string".to_string(),
                        position: start_pos,
                    });
                }
                return Ok(Some(target));
            } else if ch == '=' {
                // Found '=' before ':', so no target
                self.position = start_pos; // Reset position
                return Ok(None);
            }
            self.advance();
        }

        // Reached end without finding ':' or '=', so no target
        self.position = start_pos; // Reset position
        Ok(None)
    }

    /// Parse a metadata key
    fn parse_key(&mut self) -> Result<String, ParseError> {
        let start_pos = self.position;

        // Parse key: [A-Za-z0-9_.]+
        if !self
            .current_char()
            .is_some_and(|ch| ch.is_alphanumeric() || ch == '_')
        {
            return Err(ParseError::Expected {
                expected: "metadata key",
                found: self.current_char().unwrap_or('\0').to_string(),
                position: self.position,
            });
        }

        while self
            .current_char()
            .is_some_and(|ch| ch.is_alphanumeric() || ch == '_' || ch == '.')
        {
            self.advance();
        }

        let key = self.input[start_pos..self.position].to_string();
        if key.is_empty() {
            return Err(ParseError::Expected {
                expected: "metadata key",
                found: "empty string".to_string(),
                position: start_pos,
            });
        }

        Ok(key)
    }

    /// Skip whitespace characters
    fn skip_whitespace(&mut self) {
        while self.current_char().is_some_and(|ch| ch.is_whitespace()) {
            self.advance();
        }
    }

    /// Get the current character
    fn current_char(&self) -> Option<char> {
        self.input.chars().nth(self.position)
    }

    /// Advance the position by one character
    fn advance(&mut self) {
        if self.position < self.input.len() {
            self.position += 1;
        }
    }

    /// Consume a specific character if it matches
    fn consume_char(&mut self, expected: char) -> bool {
        if self.current_char() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_current_metadata_string() {
        let mut parser = MetadataParser::new(r#"#doc.label="Patch A""#);
        let result = parser.parse().unwrap();

        match result {
            MetadataStatement::Current { key, value } => {
                assert_eq!(key, "doc.label");
                assert_eq!(value, json!("Patch A"));
            }
            _ => panic!("Expected Current metadata"),
        }
    }

    #[test]
    fn test_parse_current_metadata_number() {
        let mut parser = MetadataParser::new("#logic.clock_hz=4");
        let result = parser.parse().unwrap();

        match result {
            MetadataStatement::Current { key, value } => {
                assert_eq!(key, "logic.clock_hz");
                assert_eq!(value, json!(4));
            }
            _ => panic!("Expected Current metadata"),
        }
    }

    #[test]
    fn test_parse_targeted_metadata() {
        let mut parser = MetadataParser::new(r#"#cpu.core:logic.clock_hz=4"#);
        let result = parser.parse().unwrap();

        match result {
            MetadataStatement::Targeted { target, key, value } => {
                assert_eq!(target, "cpu.core");
                assert_eq!(key, "logic.clock_hz");
                assert_eq!(value, json!(4));
            }
            _ => panic!("Expected Targeted metadata"),
        }
    }

    #[test]
    fn test_parse_wildcard_metadata() {
        let mut parser = MetadataParser::new(r#"#cpu.*:power.budget="low""#);
        let result = parser.parse().unwrap();

        match result {
            MetadataStatement::Targeted { target, key, value } => {
                assert_eq!(target, "cpu.*");
                assert_eq!(key, "power.budget");
                assert_eq!(value, json!("low"));
            }
            _ => panic!("Expected Targeted metadata"),
        }
    }

    #[test]
    fn test_parse_global_metadata() {
        let mut parser = MetadataParser::new("#$global:io.bus_width=8");
        let result = parser.parse().unwrap();

        match result {
            MetadataStatement::Targeted { target, key, value } => {
                assert_eq!(target, "$global");
                assert_eq!(key, "io.bus_width");
                assert_eq!(value, json!(8));
            }
            _ => panic!("Expected Targeted metadata"),
        }
    }

    #[test]
    fn test_parse_with_whitespace() {
        let mut parser = MetadataParser::new(r#"#  cpu.core  :  logic.clock_hz  =  4  "#);
        let result = parser.parse().unwrap();

        match result {
            MetadataStatement::Targeted { target, key, value } => {
                assert_eq!(target, "cpu.core");
                assert_eq!(key, "logic.clock_hz");
                assert_eq!(value, json!(4));
            }
            _ => panic!("Expected Targeted metadata"),
        }
    }

    #[test]
    fn test_parse_complex_json() {
        let mut parser = MetadataParser::new(r#"#config={"enabled": true, "count": 42}"#);
        let result = parser.parse().unwrap();

        match result {
            MetadataStatement::Current { key, value } => {
                assert_eq!(key, "config");
                assert_eq!(value, json!({"enabled": true, "count": 42}));
            }
            _ => panic!("Expected Current metadata"),
        }
    }

    #[test]
    fn test_parse_error_missing_hash() {
        let mut parser = MetadataParser::new("key=value");
        let result = parser.parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_missing_equals() {
        let mut parser = MetadataParser::new("#key");
        let result = parser.parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_invalid_json() {
        let mut parser = MetadataParser::new("#key=invalid_json");
        let result = parser.parse();
        assert!(result.is_err());
    }
}
