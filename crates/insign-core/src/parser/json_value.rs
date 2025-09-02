use crate::ParseError;
use serde_json::Value;

/// Parser for strict JSON values in metadata
pub struct JsonValueParser<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> JsonValueParser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, position: 0 }
    }
    
    /// Parse a JSON value from the input
    pub fn parse(&mut self) -> Result<Value, ParseError> {
        self.skip_whitespace();
        let _start_pos = self.position;
        
        // Find the end of the JSON value by parsing it with serde_json
        // We'll try parsing increasingly longer substrings until we find a valid JSON value
        let remaining_input = &self.input[self.position..];
        
        // Try to parse the JSON value
        match serde_json::from_str::<Value>(remaining_input) {
            Ok(value) => {
                // The entire remaining input was valid JSON, advance to the end
                self.position = self.input.len();
                Ok(value)
            },
            Err(_) => {
                // Try to find the boundary of the JSON value
                // This is tricky because JSON can have nested structures
                self.parse_json_value_boundary()
            }
        }
    }
    
    /// Parse JSON value by finding its boundary
    fn parse_json_value_boundary(&mut self) -> Result<Value, ParseError> {
        self.skip_whitespace();
        let _start_pos = self.position;
        
        match self.current_char() {
            Some('"') => self.parse_string_value(),
            Some(ch) if ch.is_ascii_digit() || ch == '-' => self.parse_number_value(),
            Some('t') | Some('f') => self.parse_boolean_value(),
            Some('n') => self.parse_null_value(),
            Some('[') => self.parse_array_value(),
            Some('{') => self.parse_object_value(),
            Some(ch) => Err(ParseError::Expected {
                expected: "JSON value",
                found: ch.to_string(),
                position: self.position,
            }),
            None => Err(ParseError::UnexpectedEnd {
                expected: "JSON value",
                position: self.position,
            }),
        }
    }
    
    /// Parse a JSON string value
    fn parse_string_value(&mut self) -> Result<Value, ParseError> {
        let start_pos = self.position;
        self.advance(); // Skip opening quote
        
        let mut escaped = false;
        while let Some(ch) = self.current_char() {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                self.advance(); // Skip closing quote
                let json_str = &self.input[start_pos..self.position];
                return serde_json::from_str(json_str).map_err(|_| ParseError::Expected {
                    expected: "valid JSON string",
                    found: json_str.to_string(),
                    position: start_pos,
                });
            }
            self.advance();
        }
        
        Err(ParseError::UnexpectedEnd {
            expected: "closing quote for JSON string",
            position: self.position,
        })
    }
    
    /// Parse a JSON number value
    fn parse_number_value(&mut self) -> Result<Value, ParseError> {
        let start_pos = self.position;
        
        // Handle negative sign
        if self.current_char() == Some('-') {
            self.advance();
        }
        
        // Parse digits
        while self.current_char().map_or(false, |ch| ch.is_ascii_digit() || ch == '.' || ch == 'e' || ch == 'E' || ch == '+' || ch == '-') {
            self.advance();
        }
        
        let json_str = &self.input[start_pos..self.position];
        serde_json::from_str(json_str).map_err(|_| ParseError::Expected {
            expected: "valid JSON number",
            found: json_str.to_string(),
            position: start_pos,
        })
    }
    
    /// Parse a JSON boolean value
    fn parse_boolean_value(&mut self) -> Result<Value, ParseError> {
        let start_pos = self.position;
        
        if self.consume_str("true") {
            Ok(Value::Bool(true))
        } else if self.consume_str("false") {
            Ok(Value::Bool(false))
        } else {
            Err(ParseError::Expected {
                expected: "'true' or 'false'",
                found: self.peek_str(5).to_string(),
                position: start_pos,
            })
        }
    }
    
    /// Parse a JSON null value
    fn parse_null_value(&mut self) -> Result<Value, ParseError> {
        let start_pos = self.position;
        
        if self.consume_str("null") {
            Ok(Value::Null)
        } else {
            Err(ParseError::Expected {
                expected: "'null'",
                found: self.peek_str(4).to_string(),
                position: start_pos,
            })
        }
    }
    
    /// Parse a JSON array value
    fn parse_array_value(&mut self) -> Result<Value, ParseError> {
        let start_pos = self.position;
        let mut depth = 0;
        
        while let Some(ch) = self.current_char() {
            match ch {
                '[' => depth += 1,
                ']' => {
                    depth -= 1;
                    self.advance();
                    if depth == 0 {
                        let json_str = &self.input[start_pos..self.position];
                        return serde_json::from_str(json_str).map_err(|_| ParseError::Expected {
                            expected: "valid JSON array",
                            found: json_str.to_string(),
                            position: start_pos,
                        });
                    }
                },
                '"' => {
                    self.advance();
                    self.skip_string_content();
                },
                _ => self.advance(),
            }
        }
        
        Err(ParseError::UnexpectedEnd {
            expected: "closing ']' for JSON array",
            position: self.position,
        })
    }
    
    /// Parse a JSON object value
    fn parse_object_value(&mut self) -> Result<Value, ParseError> {
        let start_pos = self.position;
        let mut depth = 0;
        
        while let Some(ch) = self.current_char() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    self.advance();
                    if depth == 0 {
                        let json_str = &self.input[start_pos..self.position];
                        return serde_json::from_str(json_str).map_err(|_| ParseError::Expected {
                            expected: "valid JSON object",
                            found: json_str.to_string(),
                            position: start_pos,
                        });
                    }
                },
                '"' => {
                    self.advance();
                    self.skip_string_content();
                },
                _ => self.advance(),
            }
        }
        
        Err(ParseError::UnexpectedEnd {
            expected: "closing '}' for JSON object",
            position: self.position,
        })
    }
    
    /// Skip over string content, handling escape sequences
    fn skip_string_content(&mut self) {
        let mut escaped = false;
        while let Some(ch) = self.current_char() {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                self.advance();
                return;
            }
            self.advance();
        }
    }
    
    /// Skip whitespace characters
    fn skip_whitespace(&mut self) {
        while self.current_char().map_or(false, |ch| ch.is_whitespace()) {
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
    
    /// Consume a specific string if it matches
    fn consume_str(&mut self, expected: &str) -> bool {
        if self.input[self.position..].starts_with(expected) {
            self.position += expected.len();
            true
        } else {
            false
        }
    }
    
    /// Peek at a string of the given length starting from current position
    fn peek_str(&self, len: usize) -> &str {
        let end = (self.position + len).min(self.input.len());
        &self.input[self.position..end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_parse_string() {
        let mut parser = JsonValueParser::new(r#""hello world""#);
        let result = parser.parse().unwrap();
        assert_eq!(result, json!("hello world"));
    }
    
    #[test]
    fn test_parse_number() {
        let mut parser = JsonValueParser::new("42");
        let result = parser.parse().unwrap();
        assert_eq!(result, json!(42));
        
        let mut parser = JsonValueParser::new("-3.14");
        let result = parser.parse().unwrap();
        assert_eq!(result, json!(-3.14));
    }
    
    #[test]
    fn test_parse_boolean() {
        let mut parser = JsonValueParser::new("true");
        let result = parser.parse().unwrap();
        assert_eq!(result, json!(true));
        
        let mut parser = JsonValueParser::new("false");
        let result = parser.parse().unwrap();
        assert_eq!(result, json!(false));
    }
    
    #[test]
    fn test_parse_null() {
        let mut parser = JsonValueParser::new("null");
        let result = parser.parse().unwrap();
        assert_eq!(result, json!(null));
    }
    
    #[test]
    fn test_parse_array() {
        let mut parser = JsonValueParser::new(r#"[1, "hello", true]"#);
        let result = parser.parse().unwrap();
        assert_eq!(result, json!([1, "hello", true]));
    }
    
    #[test]
    fn test_parse_object() {
        let mut parser = JsonValueParser::new(r#"{"key": "value", "num": 42}"#);
        let result = parser.parse().unwrap();
        assert_eq!(result, json!({"key": "value", "num": 42}));
    }
}
