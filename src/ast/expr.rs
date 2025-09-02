/// Boolean expression for region definitions
#[derive(Debug, Clone, PartialEq)]
pub enum BooleanExpr {
    /// Reference to a region by name (e.g., "dataloop", "cpu.core")
    RegionRef(String),
    /// Union of two expressions (a + b)
    Union(Box<BooleanExpr>, Box<BooleanExpr>),
    /// Difference/subtraction of two expressions (a - b)
    #[cfg(feature = "boolean_ops")]
    Difference(Box<BooleanExpr>, Box<BooleanExpr>),
    /// Intersection of two expressions (a & b)
    #[cfg(feature = "boolean_ops")]
    Intersection(Box<BooleanExpr>, Box<BooleanExpr>),
    /// XOR of two expressions (a ^ b)
    #[cfg(feature = "boolean_ops")]
    Xor(Box<BooleanExpr>, Box<BooleanExpr>),
}

impl BooleanExpr {
    /// Create a union of two expressions
    pub fn union(left: BooleanExpr, right: BooleanExpr) -> Self {
        BooleanExpr::Union(Box::new(left), Box::new(right))
    }
    
    /// Create a difference of two expressions
    #[cfg(feature = "boolean_ops")]
    pub fn difference(left: BooleanExpr, right: BooleanExpr) -> Self {
        BooleanExpr::Difference(Box::new(left), Box::new(right))
    }
    
    /// Create an intersection of two expressions
    #[cfg(feature = "boolean_ops")]
    pub fn intersection(left: BooleanExpr, right: BooleanExpr) -> Self {
        BooleanExpr::Intersection(Box::new(left), Box::new(right))
    }
    
    /// Create an XOR of two expressions
    #[cfg(feature = "boolean_ops")]
    pub fn xor(left: BooleanExpr, right: BooleanExpr) -> Self {
        BooleanExpr::Xor(Box::new(left), Box::new(right))
    }
    
    /// Create a region reference
    pub fn region_ref(name: impl Into<String>) -> Self {
        BooleanExpr::RegionRef(name.into())
    }
    
    /// Get all region references mentioned in this expression
    pub fn region_refs(&self) -> Vec<&str> {
        let mut refs = Vec::new();
        self.collect_region_refs(&mut refs);
        refs
    }
    
    fn collect_region_refs<'a>(&'a self, refs: &mut Vec<&'a str>) {
        match self {
            BooleanExpr::RegionRef(name) => refs.push(name),
            BooleanExpr::Union(left, right) => {
                left.collect_region_refs(refs);
                right.collect_region_refs(refs);
            }
            #[cfg(feature = "boolean_ops")]
            BooleanExpr::Difference(left, right) |
            BooleanExpr::Intersection(left, right) |
            BooleanExpr::Xor(left, right) => {
                left.collect_region_refs(refs);
                right.collect_region_refs(refs);
            }
        }
    }
}

impl std::fmt::Display for BooleanExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BooleanExpr::RegionRef(name) => write!(f, "{}", name),
            BooleanExpr::Union(left, right) => write!(f, "({} + {})", left, right),
            #[cfg(feature = "boolean_ops")]
            BooleanExpr::Difference(left, right) => write!(f, "({} - {})", left, right),
            #[cfg(feature = "boolean_ops")]
            BooleanExpr::Intersection(left, right) => write!(f, "({} & {})", left, right),
            #[cfg(feature = "boolean_ops")]
            BooleanExpr::Xor(left, right) => write!(f, "({} ^ {})", left, right),
        }
    }
}
