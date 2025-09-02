use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

/// Get the path to the CLI binary
fn get_cli_binary() -> std::path::PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // remove test executable name
    if path.ends_with("deps") {
        path.pop(); // remove deps directory
    }
    path.push("insign-cli");
    if cfg!(windows) {
        path.set_extension("exe");
    }
    path
}

/// Helper to run the CLI binary with given input and args
fn run_cli_with_input(input: &str, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(get_cli_binary())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn CLI process");

    // Write input to stdin
    {
        let stdin = cmd.stdin.as_mut().expect("Failed to open stdin");
        stdin
            .write_all(input.as_bytes())
            .expect("Failed to write to stdin");
    }

    cmd.wait_with_output()
        .expect("Failed to wait for CLI process")
}

/// Helper to run the CLI binary with a file
fn run_cli_with_file(file_path: &str, args: &[&str]) -> std::process::Output {
    let mut full_args = vec![file_path];
    full_args.extend_from_slice(args);

    Command::new(get_cli_binary())
        .args(full_args)
        .output()
        .expect("Failed to run CLI process")
}

#[test]
fn test_cli_success_simple_input() {
    let input = r#"{"pos": [10, 64, 10], "text": "@rc([0,0,0],[3,2,1])\n#doc.label=\"test\""}
{"pos": [0, 64, 0], "text": "@cpu.core=ac([100,70,-20],[104,72,-18])\n#cpu.core:logic.clock_hz=4"}"#;

    let output = run_cli_with_input(input, &[]);

    assert_eq!(output.status.code().unwrap(), 0);

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"__anon_0_0\""));
    assert!(stdout.contains("\"cpu.core\""));
    assert!(stdout.contains("\"doc.label\":\"test\""));
    assert!(stdout.contains("\"logic.clock_hz\":4"));
}

#[test]
fn test_cli_pretty_print() {
    let input = r#"{"pos": [10, 64, 10], "text": "@rc([0,0,0],[3,2,1])\n#doc.label=\"test\""}"#;

    let output = run_cli_with_input(input, &["--pretty"]);

    assert_eq!(output.status.code().unwrap(), 0);

    let stdout = String::from_utf8(output.stdout).unwrap();
    // Check that it's pretty printed (contains newlines and indentation)
    assert!(stdout.contains('\n'));
    assert!(stdout.contains("    "));
}

#[test]
fn test_cli_error_case() {
    let input = r#"{"pos": [10, 64, 10], "text": "@test=rc([0,0,0],[3,2,1])\n#test:value=42"}
{"pos": [0, 64, 0], "text": "@test=ac([100,70,-20],[104,72,-18])\n#test:value=99"}"#;

    let output = run_cli_with_input(input, &["--pretty"]);

    assert_eq!(output.status.code().unwrap(), 2);

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Metadata conflict"));
    assert!(stderr.contains("\"error\""));
}

#[test]
fn test_cli_invalid_jsonl() {
    let input = r#"{"pos": [10, 64, 10], "text": "@rc([0,0,0],[3,2,1])"}
invalid json line"#;

    let output = run_cli_with_input(input, &[]);

    assert_eq!(output.status.code().unwrap(), 1);

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Error parsing JSONL input"));
    assert!(stderr.contains("Line 2: Invalid JSON"));
}

#[test]
fn test_cli_empty_input() {
    let output = run_cli_with_input("", &["--pretty"]);

    assert_eq!(output.status.code().unwrap(), 0);

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.trim(), "{}");
}

#[test]
fn test_cli_file_not_found() {
    let output = run_cli_with_file("nonexistent_file.jsonl", &[]);

    assert_eq!(output.status.code().unwrap(), 1);

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Error reading file"));
    assert!(stderr.contains("nonexistent_file.jsonl"));
}

#[test]
fn test_cli_with_tempfile() {
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let input = r###"{"pos": [5, 5, 5], "text": "@test.region=rc([1,1,1],[3,3,3])\n#test.region:type=\"example\""}
{"pos": [10, 10, 10], "text": "#$global:version=1"}"###;

    temp_file
        .write_all(input.as_bytes())
        .expect("Failed to write to temp file");

    let output = run_cli_with_file(temp_file.path().to_str().unwrap(), &["--pretty"]);

    assert_eq!(output.status.code().unwrap(), 0);

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"$global\""));
    assert!(stdout.contains("\"version\": 1"));
    assert!(stdout.contains("\"test.region\""));
    assert!(stdout.contains("\"type\": \"example\""));
}

#[test]
fn test_cli_empty_lines_handling() {
    let input =
        "{\"pos\": [10, 64, 10], \"text\": \"@rc([0,0,0],[3,2,1])\\n#doc.label=\\\"test\\\"\"}

{\"pos\": [0, 64, 0], \"text\": \"@rc([5,0,5],[8,3,8])\\n#region.note=\\\"second\\\"\"}";
    let output = run_cli_with_input(input, &[]);

    assert_eq!(output.status.code().unwrap(), 0);

    let stdout = String::from_utf8(output.stdout).unwrap();
    // Should handle empty lines correctly and include both anonymous regions since both have metadata
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.as_object().unwrap().contains_key("__anon_0_0"));
    assert!(parsed.as_object().unwrap().contains_key("__anon_1_0"));
}

#[test]
fn test_cli_help() {
    let output = Command::new(get_cli_binary())
        .args(["--help"])
        .output()
        .expect("Failed to run CLI process");

    assert_eq!(output.status.code().unwrap(), 0);

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Compiles Insign DSL from JSONL input"));
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("--pretty"));
}

#[test]
fn test_cli_version() {
    let output = Command::new(get_cli_binary())
        .args(["--version"])
        .output()
        .expect("Failed to run CLI process");

    assert_eq!(output.status.code().unwrap(), 0);

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("0.1.0"));
}

#[test]
fn test_cli_complex_multiline_dsl() {
    let input = "{\"pos\": [0, 64, 0], \"text\": \"@a=rc([0,0,0],\\n  [10,5,10])\\n@b=rc([15,0,0],\\n  [25,5,10])\\n@region.complex=a+b\\n#region.complex:description=\\\"Multi-line region\\\"\\n#region.complex:size=\\\"large\\\"\"}";
    let output = run_cli_with_input(input, &["--pretty"]);

    assert_eq!(output.status.code().unwrap(), 0);

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let region = &parsed["region.complex"];
    assert!(region["metadata"]["description"]
        .as_str()
        .unwrap()
        .contains("Multi-line region"));
    assert_eq!(region["metadata"]["size"].as_str().unwrap(), "large");

    // Should have two bounding boxes from the union
    let boxes = region["bounding_boxes"].as_array().unwrap();
    assert_eq!(boxes.len(), 2);
}
