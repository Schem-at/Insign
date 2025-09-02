use std::fs;
use std::path::Path;
use serde_json;
use insign_core::{compile, DslMap};

/// Test fixture metadata
struct Fixture {
    name: String,
    input_path: String,
    expected_path: String,
    should_succeed: bool,
}

impl Fixture {
    fn new(name: &str, should_succeed: bool) -> Self {
        Self {
            name: name.to_string(),
            input_path: format!("tests/fixtures/inputs/{}.txt", name),
            expected_path: format!("tests/fixtures/expected/{}.json", name),
            should_succeed,
        }
    }
}

/// All test fixtures for the golden suite
fn get_fixtures() -> Vec<Fixture> {
    vec![
        Fixture::new("A_single_anon", true),
        Fixture::new("B_named_multi", true), 
        Fixture::new("C_boolean_union", true),
        Fixture::new("D_wildcards_global", true),
        Fixture::new("E_conflicts", false), // Should fail due to metadata conflicts
        Fixture::new("F_multiline_dense", true),
        Fixture::new("G_rc_ac_mixing", true),
        Fixture::new("H_anon_no_metadata", true),
        Fixture::new("I_complex_expressions", true),
        Fixture::new("J_edge_coordinates", true),
    ]
}

/// Parse fixture input and compile it
fn compile_fixture(input_path: &str) -> Result<DslMap, insign_core::Error> {
    let input_text = fs::read_to_string(input_path)
        .map_err(|e| insign_core::Error::Parser(insign_core::ParseError::Internal {
            message: format!("Failed to read fixture file: {}", e),
            position: 0,
        }))?;
    
    // Create units array with position [0,0,0] for single-tuple fixtures
    let units = vec![([0, 0, 0], input_text)];
    compile(&units)
}

/// Generate expected JSON files by running the compiler
/// This is a helper function to create the golden files initially
#[test]
#[ignore] // Run with: cargo test generate_expected_outputs -- --ignored
fn generate_expected_outputs() {
    for fixture in get_fixtures() {
        println!("Generating expected output for fixture: {}", fixture.name);
        
        let result = compile_fixture(&fixture.input_path);
        
        if fixture.should_succeed {
            match result {
                Ok(dsl_map) => {
                    let json = serde_json::to_string_pretty(&dsl_map).unwrap();
                    fs::write(&fixture.expected_path, json).unwrap();
                    println!("  ✅ Generated {}", fixture.expected_path);
                },
                Err(e) => {
                    panic!("Fixture {} was expected to succeed but failed with: {}", fixture.name, e);
                }
            }
        } else {
            // For fixtures that should fail, create an error expectation file
            match result {
                Ok(_) => {
                    panic!("Fixture {} was expected to fail but succeeded", fixture.name);
                },
                Err(e) => {
                    let error_json = serde_json::json!({
                        "error": format!("{}", e),
                        "error_type": "compilation_failure"
                    });
                    let json = serde_json::to_string_pretty(&error_json).unwrap();
                    fs::write(&fixture.expected_path, json).unwrap();
                    println!("  ✅ Generated error expectation {}", fixture.expected_path);
                }
            }
        }
    }
}

/// The main golden test - validates that each fixture produces byte-identical JSON
#[test]
fn golden_suite_validation() {
    let mut passed = 0;
    let mut failed = 0;
    
    for fixture in get_fixtures() {
        println!("Testing fixture: {}", fixture.name);
        
        // Check that expected file exists
        if !Path::new(&fixture.expected_path).exists() {
            println!("  ❌ Expected file missing: {}", fixture.expected_path);
            println!("     Run: cargo test generate_expected_outputs -- --ignored");
            failed += 1;
            continue;
        }
        
        // Compile the fixture
        let result = compile_fixture(&fixture.input_path);
        
        // Read expected output
        let expected_content = fs::read_to_string(&fixture.expected_path).unwrap();
        
        if fixture.should_succeed {
            match result {
                Ok(actual_dsl_map) => {
                    let actual_json = serde_json::to_string_pretty(&actual_dsl_map).unwrap();
                    
                    if actual_json == expected_content {
                        println!("  ✅ PASS");
                        passed += 1;
                    } else {
                        println!("  ❌ FAIL - JSON mismatch");
                        println!("     Expected: {}", expected_content);
                        println!("     Actual:   {}", actual_json);
                        failed += 1;
                    }
                },
                Err(e) => {
                    println!("  ❌ FAIL - Unexpected compilation error: {}", e);
                    failed += 1;
                }
            }
        } else {
            // Fixture should fail
            match result {
                Ok(_) => {
                    println!("  ❌ FAIL - Expected compilation to fail but it succeeded");
                    failed += 1;
                },
                Err(actual_error) => {
                    let expected_error: serde_json::Value = serde_json::from_str(&expected_content).unwrap();
                    let actual_error_json = serde_json::json!({
                        "error": format!("{}", actual_error),
                        "error_type": "compilation_failure"
                    });
                    
                    if actual_error_json == expected_error {
                        println!("  ✅ PASS (expected failure)");
                        passed += 1;
                    } else {
                        println!("  ❌ FAIL - Error mismatch");
                        println!("     Expected: {}", expected_error);
                        println!("     Actual:   {}", actual_error_json);
                        failed += 1;
                    }
                }
            }
        }
    }
    
    println!("\n📊 Golden Suite Results:");
    println!("   ✅ Passed: {}", passed);
    println!("   ❌ Failed: {}", failed);
    println!("   📝 Total:  {}", passed + failed);
    
    if failed > 0 {
        panic!("Golden suite validation failed! {} test(s) failed.", failed);
    }
}
