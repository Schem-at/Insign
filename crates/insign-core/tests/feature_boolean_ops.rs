// Tests for boolean_ops feature when enabled
// This test file is only compiled when boolean_ops feature is enabled

#[cfg(feature = "boolean_ops")]
mod boolean_ops_enabled_tests {
    use insign_core::compile;

    #[test]
    fn test_boolean_ops_enabled_minus_works() {
        // With feature flag enabled, boolean operations should work
        let units = vec![
            ([0, 0, 0], "@a=rc([0,0,0],[4,4,4])".to_string()),
            ([0, 0, 0], "@b=rc([1,1,1],[3,3,3])".to_string()),
            ([0, 0, 0], "@result=a-b".to_string()),
        ];

        let result = compile(&units);
        assert!(
            result.is_ok(),
            "Difference operator should work with boolean_ops feature enabled"
        );

        let dsl_map = result.unwrap();
        assert!(dsl_map.contains_key("result"));

        // Verify that result has bounding boxes (difference of a and b)
        let result_entry = dsl_map.get("result").unwrap();
        let boxes = result_entry.bounding_boxes.as_ref().unwrap();
        assert!(
            !boxes.is_empty(),
            "Difference should produce at least one bounding box"
        );
    }

    #[test]
    fn test_boolean_ops_enabled_and_works() {
        let units = vec![
            ([0, 0, 0], "@a=rc([0,0,0],[4,4,4])".to_string()),
            ([0, 0, 0], "@b=rc([2,2,2],[6,6,6])".to_string()),
            ([0, 0, 0], "@result=a&b".to_string()),
        ];

        let result = compile(&units);
        assert!(
            result.is_ok(),
            "Intersection operator should work with boolean_ops feature enabled"
        );

        let dsl_map = result.unwrap();
        assert!(dsl_map.contains_key("result"));

        // Verify that result has intersection bounding box
        let result_entry = dsl_map.get("result").unwrap();
        let boxes = result_entry.bounding_boxes.as_ref().unwrap();
        assert_eq!(
            boxes.len(),
            1,
            "Intersection should produce one bounding box"
        );
        assert_eq!(
            boxes[0],
            ([2, 2, 2], [4, 4, 4]),
            "Intersection should be correct"
        );
    }

    #[test]
    fn test_boolean_ops_enabled_xor_works() {
        let units = vec![
            ([0, 0, 0], "@a=rc([0,0,0],[3,3,3])".to_string()),
            ([0, 0, 0], "@b=rc([1,1,1],[4,4,4])".to_string()),
            ([0, 0, 0], "@result=a^b".to_string()),
        ];

        let result = compile(&units);
        assert!(
            result.is_ok(),
            "XOR operator should work with boolean_ops feature enabled"
        );

        let dsl_map = result.unwrap();
        assert!(dsl_map.contains_key("result"));

        // Verify that result has XOR bounding boxes (a-b + b-a)
        let result_entry = dsl_map.get("result").unwrap();
        let boxes = result_entry.bounding_boxes.as_ref().unwrap();
        assert!(
            !boxes.is_empty(),
            "XOR should produce at least one bounding box"
        );
    }

    #[test]
    fn test_plus_operator_still_works_with_feature() {
        // Union operator should still work when boolean_ops is enabled
        let units = vec![
            ([0, 0, 0], "@a=rc([0,0,0],[1,1,1])".to_string()),
            ([0, 0, 0], "@b=rc([2,0,0],[3,1,1])".to_string()),
            ([0, 0, 0], "@result=a+b".to_string()),
        ];

        let result = compile(&units);
        assert!(
            result.is_ok(),
            "Union operator should work with boolean_ops feature enabled"
        );

        let dsl_map = result.unwrap();
        assert!(dsl_map.contains_key("result"));

        // Verify that result has two bounding boxes (union of a and b)
        let result_entry = dsl_map.get("result").unwrap();
        let boxes = result_entry.bounding_boxes.as_ref().unwrap();
        assert_eq!(boxes.len(), 2, "Union should produce two bounding boxes");
    }
}
