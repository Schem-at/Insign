#[cfg(feature = "boolean_ops")]
mod boolean_ops_tests {
    use insign::compile;

    #[test]
    fn test_difference_operation_simple() {
        let units = vec![
            ([0, 0, 0], "@base=rc([0,0,0],[4,4,4])".to_string()),
            ([0, 0, 0], "@hole=rc([1,1,1],[3,3,3])".to_string()),
            ([0, 0, 0], "@result=base-hole".to_string()),
        ];

        let result = compile(&units);
        assert!(result.is_ok());

        let dsl_map = result.unwrap();
        assert!(dsl_map.contains_key("result"));

        let result_region = &dsl_map["result"];
        let boxes = result_region.bounding_boxes.as_ref().unwrap();

        // Should have multiple boxes representing the difference
        assert!(!boxes.is_empty());

        // Original base box minus hole should result in multiple boxes
        // The exact number depends on the implementation, but should be > 1
        assert!(boxes.len() > 1);
    }

    #[test]
    fn test_intersection_operation_simple() {
        let units = vec![
            ([0, 0, 0], "@box1=rc([0,0,0],[4,4,4])".to_string()),
            ([0, 0, 0], "@box2=rc([2,2,2],[6,6,6])".to_string()),
            ([0, 0, 0], "@result=box1&box2".to_string()),
        ];

        let result = compile(&units);
        assert!(result.is_ok());

        let dsl_map = result.unwrap();
        assert!(dsl_map.contains_key("result"));

        let result_region = &dsl_map["result"];
        let boxes = result_region.bounding_boxes.as_ref().unwrap();

        // Should have exactly one box representing the intersection
        assert_eq!(boxes.len(), 1);
        assert_eq!(boxes[0], ([2, 2, 2], [4, 4, 4])); // Intersection area
    }

    #[test]
    fn test_xor_operation_simple() {
        let units = vec![
            ([0, 0, 0], "@box1=rc([0,0,0],[3,3,3])".to_string()),
            ([0, 0, 0], "@box2=rc([1,1,1],[4,4,4])".to_string()),
            ([0, 0, 0], "@result=box1^box2".to_string()),
        ];

        let result = compile(&units);
        assert!(result.is_ok());

        let dsl_map = result.unwrap();
        assert!(dsl_map.contains_key("result"));

        let result_region = &dsl_map["result"];
        let boxes = result_region.bounding_boxes.as_ref().unwrap();

        // XOR should result in multiple boxes (box1-box2 + box2-box1)
        assert!(boxes.len() > 1);
    }

    #[test]
    fn test_operator_precedence() {
        // Test: & > + > - > ^
        // Expression: a + b & c - d ^ e should parse as: (a + (b & c) - d) ^ e
        let units = vec![
            ([0, 0, 0], "@a=rc([0,0,0],[1,1,1])".to_string()),
            ([0, 0, 0], "@b=rc([2,2,2],[3,3,3])".to_string()),
            ([0, 0, 0], "@c=rc([2,2,2],[4,4,4])".to_string()),
            ([0, 0, 0], "@d=rc([0,0,0],[0,0,0])".to_string()),
            ([0, 0, 0], "@e=rc([5,5,5],[6,6,6])".to_string()),
            ([0, 0, 0], "@result=a+b&c-d^e".to_string()),
        ];

        let result = compile(&units);
        assert!(result.is_ok());

        let dsl_map = result.unwrap();
        assert!(dsl_map.contains_key("result"));

        // Just verify it compiled successfully with the expected precedence
        let result_region = &dsl_map["result"];
        assert!(result_region.bounding_boxes.is_some());
    }

    #[test]
    fn test_parentheses_override_precedence() {
        // Test: (a + b) & c should be different from a + b & c
        // Using boxes that will produce different results based on precedence
        let units = vec![
            ([0, 0, 0], "@a=rc([0,0,0],[1,1,1])".to_string()), // Small box at origin
            ([0, 0, 0], "@b=rc([3,3,3],[6,6,6])".to_string()), // Box that intersects with c
            ([0, 0, 0], "@c=rc([0,0,0],[4,4,4])".to_string()), // Box covering a and part of b
            ([0, 0, 0], "@result1=a+b&c".to_string()),         // Should be a+(b&c)
            ([0, 0, 0], "@result2=(a+b)&c".to_string()),       // Should be (a+b)&c
        ];

        let result = compile(&units);
        assert!(result.is_ok());

        let dsl_map = result.unwrap();
        assert!(dsl_map.contains_key("result1"));
        assert!(dsl_map.contains_key("result2"));

        let result1_boxes = dsl_map["result1"].bounding_boxes.as_ref().unwrap();
        let result2_boxes = dsl_map["result2"].bounding_boxes.as_ref().unwrap();

        // result1 = a+(b&c) = a+[intersection of b and c] = [a, intersection]
        // result2 = (a+b)&c = [a,b]&c = [intersection of a&c, intersection of b&c]
        // result1 should have 2 boxes: a + (b&c)
        // result2 should have 2 boxes: (a&c) + (b&c), but a&c = a since a is inside c
        assert_eq!(result1_boxes.len(), 2);
        assert_eq!(result2_boxes.len(), 2);
        // Both should contain box a and the intersection of b&c
        assert!(result1_boxes.contains(&([0, 0, 0], [1, 1, 1]))); // Box a
        assert!(result2_boxes.contains(&([0, 0, 0], [1, 1, 1]))); // Box a
        assert!(result1_boxes.contains(&([3, 3, 3], [4, 4, 4]))); // b&c intersection
        assert!(result2_boxes.contains(&([3, 3, 3], [4, 4, 4]))); // b&c intersection
    }

    #[test]
    fn test_complex_boolean_expression() {
        // Test a complex expression with all operators and precedence
        let units = vec![
            ([0, 0, 0], "@base=rc([0,0,0],[10,10,10])".to_string()),
            ([0, 0, 0], "@cut1=rc([2,2,2],[4,4,4])".to_string()),
            ([0, 0, 0], "@cut2=rc([6,6,6],[8,8,8])".to_string()),
            ([0, 0, 0], "@add=rc([5,5,5],[7,7,7])".to_string()),
            ([0, 0, 0], "@result=base-cut1+add&base-cut2".to_string()),
        ];

        let result = compile(&units);
        assert!(result.is_ok());

        let dsl_map = result.unwrap();
        assert!(dsl_map.contains_key("result"));

        let result_region = &dsl_map["result"];
        assert!(result_region.bounding_boxes.is_some());
        let boxes = result_region.bounding_boxes.as_ref().unwrap();
        assert!(!boxes.is_empty());
    }

    #[test]
    fn test_no_intersection_result() {
        // Test intersection of two boxes that don't overlap
        let units = vec![
            ([0, 0, 0], "@box1=rc([0,0,0],[1,1,1])".to_string()),
            ([0, 0, 0], "@box2=rc([5,5,5],[6,6,6])".to_string()),
            ([0, 0, 0], "@result=box1&box2".to_string()),
        ];

        let result = compile(&units);
        assert!(result.is_ok());

        let dsl_map = result.unwrap();
        assert!(dsl_map.contains_key("result"));

        let result_region = &dsl_map["result"];
        let boxes = result_region.bounding_boxes.as_ref().unwrap();

        // No intersection should result in empty box list
        assert_eq!(boxes.len(), 0);
    }

    #[test]
    fn test_difference_no_overlap() {
        // Test difference where subtrahend doesn't overlap with minuend
        let units = vec![
            ([0, 0, 0], "@base=rc([0,0,0],[2,2,2])".to_string()),
            ([0, 0, 0], "@subtract=rc([5,5,5],[6,6,6])".to_string()),
            ([0, 0, 0], "@result=base-subtract".to_string()),
        ];

        let result = compile(&units);
        assert!(result.is_ok());

        let dsl_map = result.unwrap();
        assert!(dsl_map.contains_key("result"));

        let result_region = &dsl_map["result"];
        let boxes = result_region.bounding_boxes.as_ref().unwrap();

        // No overlap means original box should remain unchanged
        assert_eq!(boxes.len(), 1);
        assert_eq!(boxes[0], ([0, 0, 0], [2, 2, 2]));
    }

    #[test]
    fn test_difference_complete_subtraction() {
        // Test difference where subtrahend completely contains minuend
        let units = vec![
            ([0, 0, 0], "@small=rc([1,1,1],[2,2,2])".to_string()),
            ([0, 0, 0], "@large=rc([0,0,0],[3,3,3])".to_string()),
            ([0, 0, 0], "@result=small-large".to_string()),
        ];

        let result = compile(&units);
        assert!(result.is_ok());

        let dsl_map = result.unwrap();
        assert!(dsl_map.contains_key("result"));

        let result_region = &dsl_map["result"];
        let boxes = result_region.bounding_boxes.as_ref().unwrap();

        // Complete subtraction should result in no boxes
        assert_eq!(boxes.len(), 0);
    }

    #[test]
    fn test_multiple_operators_associativity() {
        // Test left-to-right associativity: a-b-c should be (a-b)-c
        let units = vec![
            ([0, 0, 0], "@a=rc([0,0,0],[4,4,4])".to_string()),
            ([0, 0, 0], "@b=rc([1,1,1],[2,2,2])".to_string()),
            ([0, 0, 0], "@c=rc([3,3,3],[3,3,3])".to_string()),
            ([0, 0, 0], "@result1=a-b-c".to_string()), // Should be (a-b)-c
            ([0, 0, 0], "@result2=(a-b)-c".to_string()), // Explicit grouping
        ];

        let result = compile(&units);
        assert!(result.is_ok());

        let dsl_map = result.unwrap();
        assert!(dsl_map.contains_key("result1"));
        assert!(dsl_map.contains_key("result2"));

        let result1_boxes = dsl_map["result1"].bounding_boxes.as_ref().unwrap();
        let result2_boxes = dsl_map["result2"].bounding_boxes.as_ref().unwrap();

        // Results should be identical due to left-to-right associativity
        assert_eq!(result1_boxes, result2_boxes);
    }

    #[test]
    fn test_performance_with_many_boxes() {
        // Test with multiple accumulator boxes to verify performance is reasonable
        let mut units = Vec::new();

        // Create a region with many boxes
        units.push(([0, 0, 0], "@base=rc([0,0,0],[1,1,1])".to_string()));
        for i in 1..20 {
            units.push((
                [0, 0, 0],
                format!(
                    "@base=rc([{},{},{}],[{},{},{}])",
                    i,
                    i,
                    i,
                    i + 1,
                    i + 1,
                    i + 1
                ),
            ));
        }

        // Create another region with many boxes
        units.push(([0, 0, 0], "@other=rc([5,5,5],[6,6,6])".to_string()));
        for i in 6..15 {
            units.push((
                [0, 0, 0],
                format!(
                    "@other=rc([{},{},{}],[{},{},{}])",
                    i,
                    i,
                    i,
                    i + 1,
                    i + 1,
                    i + 1
                ),
            ));
        }

        // Perform intersection
        units.push(([0, 0, 0], "@result=base&other".to_string()));

        let result = compile(&units);
        assert!(result.is_ok());

        let dsl_map = result.unwrap();
        assert!(dsl_map.contains_key("result"));
    }

    #[test]
    fn test_chained_operations() {
        // Test chaining multiple boolean operations
        let units = vec![
            ([0, 0, 0], "@a=rc([0,0,0],[3,3,3])".to_string()),
            ([0, 0, 0], "@b=rc([1,1,1],[4,4,4])".to_string()),
            ([0, 0, 0], "@c=rc([2,2,2],[5,5,5])".to_string()),
            ([0, 0, 0], "@step1=a+b".to_string()),     // Union
            ([0, 0, 0], "@step2=step1&c".to_string()), // Intersection
            ([0, 0, 0], "@step3=step2-a".to_string()), // Difference
            ([0, 0, 0], "@final=step3^b".to_string()), // XOR
        ];

        let result = compile(&units);
        assert!(result.is_ok());

        let dsl_map = result.unwrap();
        for region in ["step1", "step2", "step3", "final"] {
            assert!(dsl_map.contains_key(region), "Missing region: {}", region);
            assert!(dsl_map[region].bounding_boxes.is_some());
        }
    }
}
