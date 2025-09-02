use crate::parser::meta::MetadataStatement;

/// High-level AST node for metadata statements
#[derive(Debug, Clone, PartialEq)]
pub struct MetaStmt {
    /// The tuple index this statement belongs to
    pub tuple_idx: usize,
    /// The statement index within the tuple
    pub stmt_idx: usize,
    /// The parsed metadata statement
    pub statement: MetadataStatement,
}

impl MetaStmt {
    /// Create a new metadata statement
    pub fn new(tuple_idx: usize, stmt_idx: usize, statement: MetadataStatement) -> Self {
        Self { tuple_idx, stmt_idx, statement }
    }
    
    /// Get the target region for this metadata
    /// For current region metadata, returns None (needs to be resolved later)
    /// For targeted metadata, returns the explicit target
    pub fn target(&self) -> Option<&str> {
        match &self.statement {
            MetadataStatement::Current { .. } => None,
            MetadataStatement::Targeted { target, .. } => Some(target),
        }
    }
    
    /// Check if this is current region metadata
    pub fn is_current_region(&self) -> bool {
        matches!(self.statement, MetadataStatement::Current { .. })
    }
}
