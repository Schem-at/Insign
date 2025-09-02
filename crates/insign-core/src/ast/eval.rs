use crate::ast::{BooleanExpr, RegionEntry, RegionTable, SourceLocation};
use crate::{BoxPair, ParseError};
use std::collections::BTreeMap;

/// Result of evaluating a region's geometry
#[derive(Debug, Clone, PartialEq)]
pub struct EvaluatedRegion {
    /// The final bounding boxes for this region
    pub boxes: Vec<BoxPair>,
    /// Whether this region was successfully evaluated
    pub evaluated: bool,
}

/// Context for geometry evaluation
#[derive(Debug)]
struct EvaluationContext<'a> {
    /// The region table to evaluate
    table: &'a RegionTable,
    /// Cache of already evaluated regions
    cache: BTreeMap<String, EvaluatedRegion>,
    /// Current evaluation path for cycle detection
    evaluation_path: Vec<String>,
}

impl<'a> EvaluationContext<'a> {
    fn new(table: &'a RegionTable) -> Self {
        Self {
            table,
            cache: BTreeMap::new(),
            evaluation_path: Vec::new(),
        }
    }

    /// Evaluate a region and return its bounding boxes
    fn evaluate_region(&mut self, region_name: &str) -> Result<Vec<BoxPair>, ParseError> {
        // Check if already cached
        if let Some(cached) = self.cache.get(region_name) {
            return Ok(cached.boxes.clone());
        }

        // Check for cycle detection
        if self.evaluation_path.contains(&region_name.to_string()) {
            let cycle_start = self
                .evaluation_path
                .iter()
                .position(|r| r == region_name)
                .unwrap_or(0);
            let cycle = self.evaluation_path[cycle_start..].to_vec();
            return Err(ParseError::CycleDetected { cycle });
        }

        // Add to evaluation path
        self.evaluation_path.push(region_name.to_string());

        let result = self.evaluate_region_impl(region_name);

        // Remove from evaluation path
        self.evaluation_path.pop();

        // Cache the result if successful
        if let Ok(boxes) = &result {
            self.cache.insert(
                region_name.to_string(),
                EvaluatedRegion {
                    boxes: boxes.clone(),
                    evaluated: true,
                },
            );
        }

        result
    }

    /// Internal implementation of region evaluation
    fn evaluate_region_impl(&mut self, region_name: &str) -> Result<Vec<BoxPair>, ParseError> {
        match self.table.regions.get(region_name) {
            Some(RegionEntry::Accumulator { boxes, .. }) => {
                // Accumulator regions directly return their boxes
                Ok(boxes.clone())
            }
            Some(RegionEntry::Defined { expr, source }) => {
                // Defined regions need expression evaluation
                self.evaluate_expression(expr, region_name, source)
            }
            Some(RegionEntry::Anonymous { box_pair, .. }) => {
                // Anonymous regions return their single box
                Ok(vec![*box_pair])
            }
            None => {
                // Unknown region - we need a source location for the error
                // For now, use a default source. In a full implementation,
                // we'd track where each reference comes from
                Err(ParseError::UnknownRegion {
                    region: region_name.to_string(),
                    source: SourceLocation::new(0, 0), // TODO: Track actual source
                })
            }
        }
    }

    /// Evaluate a boolean expression
    fn evaluate_expression(
        &mut self,
        expr: &BooleanExpr,
        current_region: &str,
        source: &SourceLocation,
    ) -> Result<Vec<BoxPair>, ParseError> {
        match expr {
            BooleanExpr::RegionRef(ref_name) => {
                // Check for self-reference
                if ref_name == current_region {
                    return Err(ParseError::SelfReference {
                        region: current_region.to_string(),
                        source: source.clone(),
                    });
                }

                // Evaluate the referenced region
                self.evaluate_region(ref_name)
            }
            BooleanExpr::Union(left, right) => {
                // Evaluate both sides and concatenate boxes
                let mut left_boxes = self.evaluate_expression(left, current_region, source)?;
                let mut right_boxes = self.evaluate_expression(right, current_region, source)?;

                // Check for coordinate overflow during union
                check_boxes_bounds(&left_boxes)?;
                check_boxes_bounds(&right_boxes)?;

                left_boxes.append(&mut right_boxes);
                Ok(left_boxes)
            }
            #[cfg(feature = "boolean_ops")]
            BooleanExpr::Difference(left, right) => {
                let left_boxes = self.evaluate_expression(left, current_region, source)?;
                let right_boxes = self.evaluate_expression(right, current_region, source)?;

                check_boxes_bounds(&left_boxes)?;
                check_boxes_bounds(&right_boxes)?;

                Ok(compute_difference(&left_boxes, &right_boxes))
            }
            #[cfg(feature = "boolean_ops")]
            BooleanExpr::Intersection(left, right) => {
                let left_boxes = self.evaluate_expression(left, current_region, source)?;
                let right_boxes = self.evaluate_expression(right, current_region, source)?;

                check_boxes_bounds(&left_boxes)?;
                check_boxes_bounds(&right_boxes)?;

                Ok(compute_intersection(&left_boxes, &right_boxes))
            }
            #[cfg(feature = "boolean_ops")]
            BooleanExpr::Xor(left, right) => {
                let left_boxes = self.evaluate_expression(left, current_region, source)?;
                let right_boxes = self.evaluate_expression(right, current_region, source)?;

                check_boxes_bounds(&left_boxes)?;
                check_boxes_bounds(&right_boxes)?;

                Ok(compute_xor(&left_boxes, &right_boxes))
            }
        }
    }
}

/// Check that all boxes are within i32 bounds
fn check_boxes_bounds(boxes: &[BoxPair]) -> Result<(), ParseError> {
    for (min_corner, max_corner) in boxes {
        // Check each coordinate component
        for i in 0..3 {
            if min_corner[i] == i32::MIN
                || min_corner[i] == i32::MAX
                || max_corner[i] == i32::MIN
                || max_corner[i] == i32::MAX
            {
                return Err(ParseError::Internal {
                    message: format!(
                        "Coordinate overflow detected: box ({:?}, {:?})",
                        min_corner, max_corner
                    ),
                    position: 0,
                });
            }
        }
    }
    Ok(())
}

/// Evaluate all defined regions in a RegionTable and materialize their bounding boxes
pub fn evaluate_geometry(
    table: &RegionTable,
) -> Result<BTreeMap<String, Vec<BoxPair>>, ParseError> {
    let mut context = EvaluationContext::new(table);
    let mut results = BTreeMap::new();

    // Collect all region names for evaluation
    let region_names: Vec<String> = table.regions.keys().cloned().collect();

    for region_name in region_names {
        let boxes = context.evaluate_region(&region_name)?;
        results.insert(region_name, boxes);
    }

    Ok(results)
}

/// Evaluate a specific region and return its bounding boxes
pub fn evaluate_region_boxes(
    table: &RegionTable,
    region_name: &str,
) -> Result<Vec<BoxPair>, ParseError> {
    let mut context = EvaluationContext::new(table);
    context.evaluate_region(region_name)
}

// Boolean operation implementations (feature-gated)

#[cfg(feature = "boolean_ops")]
/// Compute the difference of two sets of boxes: left - right
/// Returns all parts of left that don't overlap with any box in right
fn compute_difference(left: &[BoxPair], right: &[BoxPair]) -> Vec<BoxPair> {
    if right.is_empty() {
        return left.to_vec();
    }

    let mut result = Vec::new();

    for left_box in left {
        let mut remaining = vec![*left_box];

        // Subtract each box from right
        for right_box in right {
            let mut new_remaining = Vec::new();

            for current_box in remaining {
                new_remaining.extend(subtract_box(current_box, *right_box));
            }

            remaining = new_remaining;
        }

        result.extend(remaining);
    }

    result
}

#[cfg(feature = "boolean_ops")]
/// Compute the intersection of two sets of boxes
/// Returns all parts where left and right overlap
fn compute_intersection(left: &[BoxPair], right: &[BoxPair]) -> Vec<BoxPair> {
    let mut result = Vec::new();

    for left_box in left {
        for right_box in right {
            if let Some(intersection) = intersect_boxes(*left_box, *right_box) {
                result.push(intersection);
            }
        }
    }

    result
}

#[cfg(feature = "boolean_ops")]
/// Compute the XOR of two sets of boxes
/// Returns (left - right) + (right - left)
fn compute_xor(left: &[BoxPair], right: &[BoxPair]) -> Vec<BoxPair> {
    let mut result = compute_difference(left, right);
    result.extend(compute_difference(right, left));
    result
}

#[cfg(feature = "boolean_ops")]
/// Subtract one box from another, returning the remaining pieces
fn subtract_box(from: BoxPair, subtract: BoxPair) -> Vec<BoxPair> {
    let (from_min, from_max) = from;
    let (sub_min, sub_max) = subtract;

    // Check if there's any intersection at all
    if !boxes_intersect(from, subtract) {
        return vec![from];
    }

    let mut result = Vec::new();

    // Generate up to 6 boxes representing the parts of 'from' that don't overlap with 'subtract'

    // Left side (x < sub_min[0])
    if from_min[0] < sub_min[0] {
        result.push((from_min, [sub_min[0] - 1, from_max[1], from_max[2]]));
    }

    // Right side (x > sub_max[0])
    if from_max[0] > sub_max[0] {
        result.push(([sub_max[0] + 1, from_min[1], from_min[2]], from_max));
    }

    // Calculate the intersection boundaries for y and z cuts
    let x_min = from_min[0].max(sub_min[0]);
    let x_max = from_max[0].min(sub_max[0]);

    // Front side (y < sub_min[1])
    if from_min[1] < sub_min[1] {
        result.push((
            [x_min, from_min[1], from_min[2]],
            [x_max, sub_min[1] - 1, from_max[2]],
        ));
    }

    // Back side (y > sub_max[1])
    if from_max[1] > sub_max[1] {
        result.push((
            [x_min, sub_max[1] + 1, from_min[2]],
            [x_max, from_max[1], from_max[2]],
        ));
    }

    // Calculate the intersection boundaries for z cuts
    let y_min = from_min[1].max(sub_min[1]);
    let y_max = from_max[1].min(sub_max[1]);

    // Bottom side (z < sub_min[2])
    if from_min[2] < sub_min[2] {
        result.push(([x_min, y_min, from_min[2]], [x_max, y_max, sub_min[2] - 1]));
    }

    // Top side (z > sub_max[2])
    if from_max[2] > sub_max[2] {
        result.push(([x_min, y_min, sub_max[2] + 1], [x_max, y_max, from_max[2]]));
    }

    result
}

#[cfg(feature = "boolean_ops")]
/// Check if two boxes intersect
fn boxes_intersect(box1: BoxPair, box2: BoxPair) -> bool {
    let (min1, max1) = box1;
    let (min2, max2) = box2;

    // Boxes intersect if they overlap on all three axes
    min1[0] <= max2[0]
        && max1[0] >= min2[0]
        && min1[1] <= max2[1]
        && max1[1] >= min2[1]
        && min1[2] <= max2[2]
        && max1[2] >= min2[2]
}

#[cfg(feature = "boolean_ops")]
/// Compute the intersection of two boxes, returning None if they don't intersect
fn intersect_boxes(box1: BoxPair, box2: BoxPair) -> Option<BoxPair> {
    let (min1, max1) = box1;
    let (min2, max2) = box2;

    let intersection_min = [
        min1[0].max(min2[0]),
        min1[1].max(min2[1]),
        min1[2].max(min2[2]),
    ];

    let intersection_max = [
        max1[0].min(max2[0]),
        max1[1].min(max2[1]),
        max1[2].min(max2[2]),
    ];

    // Check if intersection is valid (min <= max on all axes)
    if intersection_min[0] <= intersection_max[0]
        && intersection_min[1] <= intersection_max[1]
        && intersection_min[2] <= intersection_max[2]
    {
        Some((intersection_min, intersection_max))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{BooleanExpr, RegionEntry};

    /// Helper to create a test RegionTable
    fn make_test_table() -> RegionTable {
        let mut table = RegionTable::new();

        // Add accumulator region "base" with two boxes
        table.regions.insert(
            "base".to_string(),
            RegionEntry::Accumulator {
                boxes: vec![([0, 0, 0], [1, 1, 1]), ([2, 2, 2], [3, 3, 3])],
                sources: vec![SourceLocation::new(0, 0), SourceLocation::new(0, 1)],
            },
        );

        // Add accumulator region "ext" with one box
        table.regions.insert(
            "ext".to_string(),
            RegionEntry::Accumulator {
                boxes: vec![([10, 10, 10], [11, 11, 11])],
                sources: vec![SourceLocation::new(1, 0)],
            },
        );

        // Add defined region "combined" that references "base + ext"
        table.regions.insert(
            "combined".to_string(),
            RegionEntry::Defined {
                expr: BooleanExpr::Union(
                    Box::new(BooleanExpr::RegionRef("base".to_string())),
                    Box::new(BooleanExpr::RegionRef("ext".to_string())),
                ),
                source: SourceLocation::new(1, 1),
            },
        );

        // Add anonymous region
        table.regions.insert(
            "__anon_0_2".to_string(),
            RegionEntry::Anonymous {
                box_pair: ([5, 5, 5], [6, 6, 6]),
                source: SourceLocation::new(0, 2),
            },
        );

        table
    }

    #[test]
    fn test_evaluate_accumulator_region() {
        let table = make_test_table();
        let boxes = evaluate_region_boxes(&table, "base").unwrap();

        assert_eq!(boxes.len(), 2);
        assert_eq!(boxes[0], ([0, 0, 0], [1, 1, 1]));
        assert_eq!(boxes[1], ([2, 2, 2], [3, 3, 3]));
    }

    #[test]
    fn test_evaluate_anonymous_region() {
        let table = make_test_table();
        let boxes = evaluate_region_boxes(&table, "__anon_0_2").unwrap();

        assert_eq!(boxes.len(), 1);
        assert_eq!(boxes[0], ([5, 5, 5], [6, 6, 6]));
    }

    #[test]
    fn test_evaluate_union_expression() {
        let table = make_test_table();
        let boxes = evaluate_region_boxes(&table, "combined").unwrap();

        // Should concatenate boxes from "base" and "ext"
        assert_eq!(boxes.len(), 3);
        assert_eq!(boxes[0], ([0, 0, 0], [1, 1, 1])); // from base
        assert_eq!(boxes[1], ([2, 2, 2], [3, 3, 3])); // from base
        assert_eq!(boxes[2], ([10, 10, 10], [11, 11, 11])); // from ext
    }

    #[test]
    fn test_evaluate_unknown_region_error() {
        let table = make_test_table();
        let result = evaluate_region_boxes(&table, "nonexistent");

        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::UnknownRegion { region, .. } => {
                assert_eq!(region, "nonexistent");
            }
            _ => panic!("Expected UnknownRegion error"),
        }
    }

    #[test]
    fn test_self_reference_error() {
        let mut table = RegionTable::new();

        // Add a region that references itself
        table.regions.insert(
            "self_ref".to_string(),
            RegionEntry::Defined {
                expr: BooleanExpr::RegionRef("self_ref".to_string()),
                source: SourceLocation::new(0, 0),
            },
        );

        let result = evaluate_region_boxes(&table, "self_ref");

        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::SelfReference { region, .. } => {
                assert_eq!(region, "self_ref");
            }
            _ => panic!("Expected SelfReference error"),
        }
    }

    #[test]
    fn test_cycle_detection() {
        let mut table = RegionTable::new();

        // Create a cycle: a -> b -> a
        table.regions.insert(
            "a".to_string(),
            RegionEntry::Defined {
                expr: BooleanExpr::RegionRef("b".to_string()),
                source: SourceLocation::new(0, 0),
            },
        );

        table.regions.insert(
            "b".to_string(),
            RegionEntry::Defined {
                expr: BooleanExpr::RegionRef("a".to_string()),
                source: SourceLocation::new(0, 1),
            },
        );

        let result = evaluate_region_boxes(&table, "a");

        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::CycleDetected { cycle } => {
                // Should detect the cycle
                assert!(cycle.contains(&"a".to_string()));
            }
            _ => panic!("Expected CycleDetected error"),
        }
    }

    #[test]
    fn test_evaluate_all_regions() {
        let table = make_test_table();
        let results = evaluate_geometry(&table).unwrap();

        assert_eq!(results.len(), 4);

        // Check base region
        let base_boxes = &results["base"];
        assert_eq!(base_boxes.len(), 2);

        // Check ext region
        let ext_boxes = &results["ext"];
        assert_eq!(ext_boxes.len(), 1);

        // Check combined region
        let combined_boxes = &results["combined"];
        assert_eq!(combined_boxes.len(), 3);

        // Check anonymous region
        let anon_boxes = &results["__anon_0_2"];
        assert_eq!(anon_boxes.len(), 1);
    }

    #[test]
    fn test_coordinate_bounds_checking() {
        let boxes = vec![([i32::MAX, 0, 0], [i32::MAX, 1, 1])];

        let result = check_boxes_bounds(&boxes);
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_coordinate_bounds() {
        let boxes = vec![
            ([0, 0, 0], [1000, 1000, 1000]),
            ([-1000, -1000, -1000], [0, 0, 0]),
        ];

        let result = check_boxes_bounds(&boxes);
        assert!(result.is_ok());
    }
}
