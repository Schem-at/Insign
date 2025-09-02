use serde_json::json;

#[test]
fn test_m8_complete_end_to_end() {
    let units = vec![
        ([10, 64, 10], "@rc([0,0,0],[3,2,1])\n#doc.label=\"Patch A\"".to_string()),
        ([0, 0, 0], "@cpu.cache=rc([0,0,0],[1,1,1])".to_string()),
        ([0, 0, 0], "#cpu.*:power=\"low\"".to_string()),
        ([0, 0, 0], "#$global:version=\"1.0\"".to_string()),
    ];
    
    let result = insign::compile(&units);
    assert!(result.is_ok());
    
    let dsl_map = result.unwrap();
    
    // Should have deterministic ordering
    let keys: Vec<&String> = dsl_map.keys().collect();
    println!("Keys: {:?}", keys);
    
    // Should include:
    // - $global (with version metadata, no boxes)
    // - __anon_0_0 (anonymous with metadata, with boxes)
    // - cpu.* (wildcard with power metadata, no boxes) 
    // - cpu.cache (named region with power metadata from wildcard, with boxes)
    
    assert!(dsl_map.contains_key("$global"));
    assert!(dsl_map.contains_key("__anon_0_0"));
    assert!(dsl_map.contains_key("cpu.*"));
    assert!(dsl_map.contains_key("cpu.cache"));
    
    // Test $global entry
    let global_entry = dsl_map.get("$global").unwrap();
    assert_eq!(global_entry.bounding_boxes, None);
    assert_eq!(global_entry.metadata["version"], json!("1.0"));
    
    // Test anonymous region (should be included because it has metadata)
    let anon_entry = dsl_map.get("__anon_0_0").unwrap();
    assert!(anon_entry.bounding_boxes.is_some());
    assert_eq!(anon_entry.metadata["doc.label"], json!("Patch A"));
    
    // Test wildcard entry (should exist with metadata, no boxes)
    let wildcard_entry = dsl_map.get("cpu.*").unwrap();
    assert_eq!(wildcard_entry.bounding_boxes, None);
    assert_eq!(wildcard_entry.metadata["power"], json!("low"));
    
    // Test named region (should have metadata from wildcard and boxes)
    let cache_entry = dsl_map.get("cpu.cache").unwrap();
    assert!(cache_entry.bounding_boxes.is_some());
    assert_eq!(cache_entry.metadata["power"], json!("low"));
    
    println!("M8 End-to-end test passed!");
}
