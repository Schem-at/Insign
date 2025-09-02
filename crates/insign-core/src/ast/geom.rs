use crate::parser::geom::GeometryStatement;

/// High-level AST node for geometry statements
#[derive(Debug, Clone, PartialEq)]
pub struct GeomStmt {
    /// The tuple index this statement belongs to
    pub tuple_idx: usize,
    /// The statement index within the tuple
    pub stmt_idx: usize,
    /// The parsed geometry statement
    pub statement: GeometryStatement,
}

impl GeomStmt {
    /// Create a new geometry statement
    pub fn new(tuple_idx: usize, stmt_idx: usize, statement: GeometryStatement) -> Self {
        Self {
            tuple_idx,
            stmt_idx,
            statement,
        }
    }

    /// Get the region name if this is a named statement
    pub fn region(&self) -> Option<&str> {
        self.statement.region()
    }

    /// Check if this statement is anonymous (has no region name)
    pub fn is_anonymous(&self) -> bool {
        self.statement.region().is_none()
    }

    /// Get a unique key for anonymous regions
    pub fn anonymous_key(&self) -> String {
        format!("__anon_{}_{}", self.tuple_idx, self.stmt_idx)
    }
}
