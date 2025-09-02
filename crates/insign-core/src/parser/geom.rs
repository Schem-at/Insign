use crate::{BoxPair, ParseError};
use crate::ast::BooleanExpr;

/// A 3D vector coordinate
pub type Vec3 = [i32; 3];

/// Types of geometry statements
#[derive(Debug, Clone, PartialEq)]
pub enum GeometryStatement {
    /// Relative coordinate box: rc([x1,y1,z1],[x2,y2,z2])
    RelativeCoordinate { 
        /// The region name if this is a named region (e.g., @region=rc(...))
        region: Option<String>,
        /// The two corner coordinates
        corners: (Vec3, Vec3) 
    },
    /// Absolute coordinate box: ac([x1,y1,z1],[x2,y2,z2])
    AbsoluteCoordinate { 
        /// The region name if this is a named region (e.g., @region=ac(...))
        region: Option<String>,
        /// The two corner coordinates
        corners: (Vec3, Vec3) 
    },
    /// Boolean expression: @region=expr
    Expression {
        /// The region name (required for expressions)
        region: String,
        /// The boolean expression
        expr: BooleanExpr,
    },
}

impl GeometryStatement {
    /// Convert this geometry statement to a normalized BoxPair.
    /// For RelativeCoordinate, applies the offset to make it absolute.
    /// Note: Expression variants don't have direct box pairs - they need evaluation first.
    pub fn to_box_pair(&self, offset: Vec3) -> Option<BoxPair> {
        match self {
            GeometryStatement::RelativeCoordinate { corners, .. } => {
                let (c1, c2) = *corners;
                let corner1 = [c1[0] + offset[0], c1[1] + offset[1], c1[2] + offset[2]];
                let corner2 = [c2[0] + offset[0], c2[1] + offset[1], c2[2] + offset[2]];
                Some(normalize_box(corner1, corner2))
            },
            GeometryStatement::AbsoluteCoordinate { corners, .. } => {
                Some(normalize_box(corners.0, corners.1))
            },
            GeometryStatement::Expression { .. } => None, // Expressions need evaluation
        }
    }
    
    /// Get the region name if this is a named geometry statement
    pub fn region(&self) -> Option<&str> {
        match self {
            GeometryStatement::RelativeCoordinate { region, .. } => region.as_deref(),
            GeometryStatement::AbsoluteCoordinate { region, .. } => region.as_deref(),
            GeometryStatement::Expression { region, .. } => Some(region),
        }
    }
}

/// Parser for geometry statements
pub struct GeometryParser<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> GeometryParser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, position: 0 }
    }
    
    /// Parse a geometry statement from the input
    pub fn parse(&mut self) -> Result<GeometryStatement, ParseError> {
        self.skip_whitespace();
        
        // Expect '@' at the start
        if !self.consume_char('@') {
            return Err(ParseError::Expected {
                expected: "'@'",
                found: self.current_char().unwrap_or('\0').to_string(),
                position: self.position,
            });
        }
        
        // Check if this is a named region (contains '=')
        let region_name = self.parse_optional_region_name()?;
        
        if region_name.is_some() {
            // Named region, expect '=' after the region name
            self.skip_whitespace();
            if !self.consume_char('=') {
                return Err(ParseError::Expected {
                    expected: "'='",
                    found: self.current_char().unwrap_or('\0').to_string(),
                    position: self.position,
                });
            }
            self.skip_whitespace();
        }
        
        // Parse the geometry function call
        if self.consume_str("rc(") {
            let corners = self.parse_box()?;
            self.skip_whitespace();
            if !self.consume_char(')') {
                return Err(ParseError::Expected {
                    expected: "')'",
                    found: self.current_char().unwrap_or('\0').to_string(),
                    position: self.position,
                });
            }
            Ok(GeometryStatement::RelativeCoordinate { 
                region: region_name, 
                corners 
            })
        } else if self.consume_str("ac(") {
            let corners = self.parse_box()?;
            self.skip_whitespace();
            if !self.consume_char(')') {
                return Err(ParseError::Expected {
                    expected: "')'",
                    found: self.current_char().unwrap_or('\0').to_string(),
                    position: self.position,
                });
            }
            Ok(GeometryStatement::AbsoluteCoordinate { 
                region: region_name, 
                corners 
            })
        } else if let Some(region) = region_name {
            // If we have a region name but no rc( or ac(, try parsing an expression
            let expr = self.parse_expression()?;
            Ok(GeometryStatement::Expression { region, expr })
        } else {
            Err(ParseError::Expected {
                expected: "'rc(' or 'ac(' or expression",
                found: self.peek_str(10).to_string(),
                position: self.position,
            })
        }
    }
    
    /// Parse an optional region name (up to '=')
    fn parse_optional_region_name(&mut self) -> Result<Option<String>, ParseError> {
        let start_pos = self.position;
        
        // Look for characters that could be part of a region name
        while let Some(ch) = self.current_char() {
            if ch == '=' {
                // Found '=', so we have a region name
                let name = self.input[start_pos..self.position].trim().to_string();
                if name.is_empty() {
                    return Err(ParseError::Expected {
                        expected: "region name",
                        found: "empty string".to_string(),
                        position: start_pos,
                    });
                }
                return Ok(Some(name));
            } else if ch == 'r' && self.position == start_pos {
                // Check if this is the very start and looks like "rc("
                if self.peek_str(3) == "rc(" {
                    // This looks like a function call, not a region name
                    self.position = start_pos; // Reset position
                    return Ok(None);
                }
            } else if ch == 'a' && self.position == start_pos {
                // Check if this is the very start and looks like "ac("
                if self.peek_str(3) == "ac(" {
                    // This looks like a function call, not a region name
                    self.position = start_pos; // Reset position
                    return Ok(None);
                }
            }
            self.advance();
        }
        
        // Reached end without finding '=', so no region name
        self.position = start_pos; // Reset position
        Ok(None)
    }
    
    /// Parse a box: two vec3 coordinates separated by comma
    fn parse_box(&mut self) -> Result<(Vec3, Vec3), ParseError> {
        self.skip_whitespace();
        let vec1 = self.parse_vec3()?;
        self.skip_whitespace();
        
        if !self.consume_char(',') {
            return Err(ParseError::Expected {
                expected: "','",
                found: self.current_char().unwrap_or('\0').to_string(),
                position: self.position,
            });
        }
        
        self.skip_whitespace();
        let vec2 = self.parse_vec3()?;
        
        Ok((vec1, vec2))
    }
    
    /// Parse a vec3: [x,y,z]
    fn parse_vec3(&mut self) -> Result<Vec3, ParseError> {
        self.skip_whitespace();
        
        if !self.consume_char('[') {
            return Err(ParseError::Expected {
                expected: "'['",
                found: self.current_char().unwrap_or('\0').to_string(),
                position: self.position,
            });
        }
        
        self.skip_whitespace();
        let x = self.parse_integer()?;
        self.skip_whitespace();
        
        if !self.consume_char(',') {
            return Err(ParseError::Expected {
                expected: "','",
                found: self.current_char().unwrap_or('\0').to_string(),
                position: self.position,
            });
        }
        
        self.skip_whitespace();
        let y = self.parse_integer()?;
        self.skip_whitespace();
        
        if !self.consume_char(',') {
            return Err(ParseError::Expected {
                expected: "','",
                found: self.current_char().unwrap_or('\0').to_string(),
                position: self.position,
            });
        }
        
        self.skip_whitespace();
        let z = self.parse_integer()?;
        self.skip_whitespace();
        
        if !self.consume_char(']') {
            return Err(ParseError::Expected {
                expected: "']'",
                found: self.current_char().unwrap_or('\0').to_string(),
                position: self.position,
            });
        }
        
        Ok([x, y, z])
    }
    
    /// Parse an integer (potentially negative)
    fn parse_integer(&mut self) -> Result<i32, ParseError> {
        let start_pos = self.position;
        
        // Handle negative sign
        self.consume_char('-');
        
        if !self.current_char().map_or(false, |ch| ch.is_ascii_digit()) {
            return Err(ParseError::Expected {
                expected: "digit",
                found: self.current_char().unwrap_or('\0').to_string(),
                position: self.position,
            });
        }
        
        // Parse digits
        while self.current_char().map_or(false, |ch| ch.is_ascii_digit()) {
            self.advance();
        }
        
        let num_str = &self.input[start_pos..self.position];
        num_str.parse::<i32>().map_err(|e| ParseError::InvalidInteger {
            position: start_pos,
            source: e,
        })
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
    
    /// Consume a specific character if it matches
    fn consume_char(&mut self, expected: char) -> bool {
        if self.current_char() == Some(expected) {
            self.advance();
            true
        } else {
            false
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
    
    /// Parse a boolean expression with proper precedence
    /// Precedence: & (highest) > + > - > ^ (lowest)
    /// Left-to-right associativity within same precedence level
    fn parse_expression(&mut self) -> Result<BooleanExpr, ParseError> {
        self.parse_xor()
    }
    
    /// Parse XOR expressions (lowest precedence)
    fn parse_xor(&mut self) -> Result<BooleanExpr, ParseError> {
        let left = self.parse_difference()?;
        
        loop {
            self.skip_whitespace();
            if self.current_char() == Some('^') {
                #[cfg(feature = "boolean_ops")]
                {
                    self.advance();
                    self.skip_whitespace();
                    let right = self.parse_difference()?;
                    left = BooleanExpr::xor(left, right);
                }
                
                #[cfg(not(feature = "boolean_ops"))]
                {
                    return Err(ParseError::UnsupportedOperator {
                        position: self.position,
                        operator: "^".to_string(),
                    });
                }
            } else {
                break;
            }
        }
        
        Ok(left)
    }
    
    /// Parse difference expressions
    fn parse_difference(&mut self) -> Result<BooleanExpr, ParseError> {
        let left = self.parse_union()?;
        
        loop {
            self.skip_whitespace();
            if self.current_char() == Some('-') {
                #[cfg(feature = "boolean_ops")]
                {
                    self.advance();
                    self.skip_whitespace();
                    let right = self.parse_union()?;
                    left = BooleanExpr::difference(left, right);
                }
                
                #[cfg(not(feature = "boolean_ops"))]
                {
                    return Err(ParseError::UnsupportedOperator {
                        position: self.position,
                        operator: "-".to_string(),
                    });
                }
            } else {
                break;
            }
        }
        
        Ok(left)
    }
    
    /// Parse union expressions
    fn parse_union(&mut self) -> Result<BooleanExpr, ParseError> {
        let mut left = self.parse_intersection()?;
        
        loop {
            self.skip_whitespace();
            if self.current_char() == Some('+') {
                self.advance();
                self.skip_whitespace();
                let right = self.parse_intersection()?;
                left = BooleanExpr::union(left, right);
            } else {
                break;
            }
        }
        
        Ok(left)
    }
    
    /// Parse intersection expressions (highest precedence)
    fn parse_intersection(&mut self) -> Result<BooleanExpr, ParseError> {
        let left = self.parse_term()?;
        
        loop {
            self.skip_whitespace();
            if self.current_char() == Some('&') {
                #[cfg(feature = "boolean_ops")]
                {
                    self.advance();
                    self.skip_whitespace();
                    let right = self.parse_term()?;
                    left = BooleanExpr::intersection(left, right);
                }
                
                #[cfg(not(feature = "boolean_ops"))]
                {
                    return Err(ParseError::UnsupportedOperator {
                        position: self.position,
                        operator: "&".to_string(),
                    });
                }
            } else {
                break;
            }
        }
        
        Ok(left)
    }
    
    /// Parse a term: region_ref | ( expression )
    fn parse_term(&mut self) -> Result<BooleanExpr, ParseError> {
        self.skip_whitespace();
        
        if self.consume_char('(') {
            // Parenthesized expression
            self.skip_whitespace();
            let expr = self.parse_expression()?;
            self.skip_whitespace();
            
            if !self.consume_char(')') {
                return Err(ParseError::Expected {
                    expected: "')'",
                    found: self.current_char().unwrap_or('\0').to_string(),
                    position: self.position,
                });
            }
            
            Ok(expr)
        } else {
            // Region reference
            self.parse_region_ref()
        }
    }
    
    /// Parse a region reference (identifier)
    fn parse_region_ref(&mut self) -> Result<BooleanExpr, ParseError> {
        let start_pos = self.position;
        
        // Parse identifier: [A-Za-z0-9_.]+
        if !self.current_char().map_or(false, |ch| ch.is_alphanumeric() || ch == '_') {
            return Err(ParseError::Expected {
                expected: "region name",
                found: self.current_char().unwrap_or('\0').to_string(),
                position: self.position,
            });
        }
        
        while self.current_char().map_or(false, |ch| ch.is_alphanumeric() || ch == '_' || ch == '.') {
            self.advance();
        }
        
        let name = self.input[start_pos..self.position].to_string();
        if name.is_empty() {
            return Err(ParseError::EmptyExpression {
                position: start_pos,
            });
        }
        
        Ok(BooleanExpr::region_ref(name))
    }
}

/// Normalize a box by ensuring min <= max for each axis
pub fn normalize_box(corner1: Vec3, corner2: Vec3) -> BoxPair {
    let min_x = corner1[0].min(corner2[0]);
    let max_x = corner1[0].max(corner2[0]);
    let min_y = corner1[1].min(corner2[1]);
    let max_y = corner1[1].max(corner2[1]);
    let min_z = corner1[2].min(corner2[2]);
    let max_z = corner1[2].max(corner2[2]);
    
    ([min_x, min_y, min_z], [max_x, max_y, max_z])
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_simple_rc() {
        let mut parser = GeometryParser::new("@rc([0,1,2],[3,4,5])");
        let result = parser.parse().unwrap();
        
        match result {
            GeometryStatement::RelativeCoordinate { region, corners } => {
                assert_eq!(region, None);
                assert_eq!(corners, ([0, 1, 2], [3, 4, 5]));
            }
            _ => panic!("Expected RelativeCoordinate"),
        }
    }
    
    #[test]
    fn test_parse_simple_ac() {
        let mut parser = GeometryParser::new("@ac([10,-5,0],[20,15,10])");
        let result = parser.parse().unwrap();
        
        match result {
            GeometryStatement::AbsoluteCoordinate { region, corners } => {
                assert_eq!(region, None);
                assert_eq!(corners, ([10, -5, 0], [20, 15, 10]));
            }
            _ => panic!("Expected AbsoluteCoordinate"),
        }
    }
    
    #[test]
    fn test_parse_named_region() {
        let mut parser = GeometryParser::new("@dataloop=rc([0,0,0],[31,7,15])");
        let result = parser.parse().unwrap();
        
        match result {
            GeometryStatement::RelativeCoordinate { region, corners } => {
                assert_eq!(region, Some("dataloop".to_string()));
                assert_eq!(corners, ([0, 0, 0], [31, 7, 15]));
            }
            _ => panic!("Expected RelativeCoordinate"),
        }
    }
    
    #[test]
    fn test_parse_with_whitespace() {
        let mut parser = GeometryParser::new("@  region  =  ac(  [ -10 , -20 , -30 ] , [ 10 , 20 , 30 ]  )  ");
        let result = parser.parse().unwrap();
        
        match result {
            GeometryStatement::AbsoluteCoordinate { region, corners } => {
                assert_eq!(region, Some("region".to_string()));
                assert_eq!(corners, ([-10, -20, -30], [10, 20, 30]));
            }
            _ => panic!("Expected AbsoluteCoordinate"),
        }
    }
    
    #[test]
    fn test_normalize_box() {
        let box_pair = normalize_box([5, 10, 15], [0, 5, 10]);
        assert_eq!(box_pair, ([0, 5, 10], [5, 10, 15]));
    }
    
    #[test]
    fn test_to_box_pair_relative() {
        let stmt = GeometryStatement::RelativeCoordinate {
            region: None,
            corners: ([0, 0, 0], [3, 2, 1]),
        };
        let box_pair = stmt.to_box_pair([10, 64, 10]).unwrap();
        assert_eq!(box_pair, ([10, 64, 10], [13, 66, 11]));
    }
    
    #[test]
    fn test_to_box_pair_absolute() {
        let stmt = GeometryStatement::AbsoluteCoordinate {
            region: None,
            corners: ([100, 70, -20], [104, 72, -18]),
        };
        let box_pair = stmt.to_box_pair([0, 0, 0]).unwrap(); // Offset ignored for absolute
        assert_eq!(box_pair, ([100, 70, -20], [104, 72, -18]));
    }
    
    #[test]
    fn test_parse_error_missing_at() {
        let mut parser = GeometryParser::new("rc([0,0,0],[1,1,1])");
        let result = parser.parse();
        assert!(result.is_err());
    }
    
    #[test]
    fn test_parse_error_invalid_function() {
        let mut parser = GeometryParser::new("@invalid([0,0,0],[1,1,1])");
        let result = parser.parse();
        assert!(result.is_err());
    }
    
    #[test]
    fn test_parse_error_malformed_vec3() {
        let mut parser = GeometryParser::new("@rc([0,1],[3,4,5])");
        let result = parser.parse();
        assert!(result.is_err());
    }
    
    #[test]
    fn test_parse_error_malformed_integer() {
        let mut parser = GeometryParser::new("@rc([0,not_a_number,2],[3,4,5])");
        let result = parser.parse();
        assert!(result.is_err());
    }
    
    // M3 Expression parsing tests
    
    #[test]
    fn test_parse_simple_expression() {
        let mut parser = GeometryParser::new("@core=dataloop");
        let result = parser.parse().unwrap();
        
        match result {
            GeometryStatement::Expression { region, expr } => {
                assert_eq!(region, "core");
                assert_eq!(expr, BooleanExpr::region_ref("dataloop"));
            }
            _ => panic!("Expected Expression"),
        }
    }
    
    #[test]
    fn test_parse_union_expression() {
        let mut parser = GeometryParser::new("@core=dataloop.alu+dataloop.registers");
        let result = parser.parse().unwrap();
        
        match result {
            GeometryStatement::Expression { region, expr } => {
                assert_eq!(region, "core");
                let expected = BooleanExpr::union(
                    BooleanExpr::region_ref("dataloop.alu"),
                    BooleanExpr::region_ref("dataloop.registers")
                );
                assert_eq!(expr, expected);
            }
            _ => panic!("Expected Expression"),
        }
    }
    
    #[test]
    fn test_parse_parenthesized_expression() {
        let mut parser = GeometryParser::new("@result=(a+b)+c");
        let result = parser.parse().unwrap();
        
        match result {
            GeometryStatement::Expression { region, expr } => {
                assert_eq!(region, "result");
                let expected = BooleanExpr::union(
                    BooleanExpr::union(
                        BooleanExpr::region_ref("a"),
                        BooleanExpr::region_ref("b")
                    ),
                    BooleanExpr::region_ref("c")
                );
                assert_eq!(expr, expected);
            }
            _ => panic!("Expected Expression"),
        }
    }
    
    #[test]
    fn test_parse_expression_with_whitespace() {
        let mut parser = GeometryParser::new("@  result  =  ( a + b ) + c  ");
        let result = parser.parse().unwrap();
        
        match result {
            GeometryStatement::Expression { region, expr } => {
                assert_eq!(region, "result");
                let expected = BooleanExpr::union(
                    BooleanExpr::union(
                        BooleanExpr::region_ref("a"),
                        BooleanExpr::region_ref("b")
                    ),
                    BooleanExpr::region_ref("c")
                );
                assert_eq!(expr, expected);
            }
            _ => panic!("Expected Expression"),
        }
    }
    
    #[test]
    #[cfg(not(feature = "boolean_ops"))]
    fn test_parse_expression_reject_minus() {
        let mut parser = GeometryParser::new("@result=a-b");
        let result = parser.parse();
        assert!(result.is_err());
        
        match result.unwrap_err() {
            ParseError::UnsupportedOperator { operator, .. } => {
                assert_eq!(operator, "-");
            }
            _ => panic!("Expected UnsupportedOperator error"),
        }
    }
    
    #[test]
    #[cfg(not(feature = "boolean_ops"))]
    fn test_parse_expression_reject_and() {
        let mut parser = GeometryParser::new("@result=a&b");
        let result = parser.parse();
        assert!(result.is_err());
        
        match result.unwrap_err() {
            ParseError::UnsupportedOperator { operator, .. } => {
                assert_eq!(operator, "&");
            }
            _ => panic!("Expected UnsupportedOperator error"),
        }
    }
    
    #[test]
    #[cfg(not(feature = "boolean_ops"))]
    fn test_parse_expression_reject_xor() {
        let mut parser = GeometryParser::new("@result=a^b");
        let result = parser.parse();
        assert!(result.is_err());
        
        match result.unwrap_err() {
            ParseError::UnsupportedOperator { operator, .. } => {
                assert_eq!(operator, "^");
            }
            _ => panic!("Expected UnsupportedOperator error"),
        }
    }
    
    #[test]
    fn test_parse_error_empty_parens() {
        let mut parser = GeometryParser::new("@result=()");
        let result = parser.parse();
        assert!(result.is_err());
    }
    
    #[test]
    fn test_parse_error_trailing_operator() {
        let mut parser = GeometryParser::new("@result=a+");
        let result = parser.parse();
        assert!(result.is_err());
    }
    
    // Test for the "cpu.cache" bug fix
    #[test]
    fn test_parse_region_name_with_rc_substring() {
        // This should parse as a region name "cpu.cache" followed by rc(...)
        let mut parser = GeometryParser::new("@cpu.cache=rc([0,0,0],[1,1,1])");
        let result = parser.parse().unwrap();
        
        match result {
            GeometryStatement::RelativeCoordinate { region, corners } => {
                assert_eq!(region, Some("cpu.cache".to_string()));
                assert_eq!(corners, ([0, 0, 0], [1, 1, 1]));
            }
            _ => panic!("Expected RelativeCoordinate"),
        }
    }
    
    #[test]
    fn test_parse_region_name_with_ac_substring() {
        // This should parse as a region name "cpu.cache" followed by ac(...)
        let mut parser = GeometryParser::new("@cpu.cache=ac([0,0,0],[1,1,1])");
        let result = parser.parse().unwrap();
        
        match result {
            GeometryStatement::AbsoluteCoordinate { region, corners } => {
                assert_eq!(region, Some("cpu.cache".to_string()));
                assert_eq!(corners, ([0, 0, 0], [1, 1, 1]));
            }
            _ => panic!("Expected AbsoluteCoordinate"),
        }
    }
}
