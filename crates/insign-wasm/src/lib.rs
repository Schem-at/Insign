//! WASM bindings for insign-core
//!
//! This crate exports a WASM interface for the Insign DSL compiler,
//! allowing integration with web browsers and Node.js applications.

use wasm_bindgen::prelude::*;
use insign_core::compile;
use serde_json;

// Import the `console.log` function from the `console` module
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

/// Input format for JSON compilation
#[derive(serde::Deserialize)]
struct CompileInput {
    pos: [i32; 3],
    text: String,
}

/// Returns the ABI version of the WASM module
#[wasm_bindgen]
pub fn abi_version() -> u32 {
    1
}

/// Compiles input JSON string to output JSON string
/// 
/// # Arguments
/// * `input` - UTF-8 JSON string (array of {pos: [x,y,z], text: "..."})
/// 
/// # Returns
/// * JSON string - either success result or structured error JSON
/// * Never throws exceptions - all errors are returned as JSON
#[wasm_bindgen]
pub fn compile_json(input: String) -> String {
    console_log!("WASM compile_json called with {} bytes of input", input.len());
    
    // Parse JSON input
    let input_array: Vec<CompileInput> = match serde_json::from_str(&input) {
        Ok(arr) => arr,
        Err(e) => {
            console_log!("JSON parse error: {}", e);
            return create_error_json("JSONParseError", &format!("JSON parse error: {}", e));
        }
    };

    console_log!("Parsed {} input entries", input_array.len());
    
    // Convert to insign-core format
    let units: Vec<([i32; 3], String)> = input_array
        .into_iter()
        .map(|input| (input.pos, input.text))
        .collect();

    // Compile using insign-core
    match compile(&units) {
        Ok(dsl_map) => {
            console_log!("Compilation successful, serializing result");
            // Success - serialize output
            match serde_json::to_string(&dsl_map) {
                Ok(json) => {
                    console_log!("Serialization successful, returning {} bytes", json.len());
                    json
                }
                Err(e) => {
                    console_log!("Serialization error: {}", e);
                    create_error_json("SerializationError", &format!("JSON serialization error: {}", e))
                }
            }
        }
        Err(e) => {
            console_log!("Compilation error: {}", e);
            // Compilation error - return structured error JSON
            create_error_json("CompilationError", &format!("{}", e))
        }
    }
}

/// Helper function to create structured error JSON
fn create_error_json(code: &str, message: &str) -> String {
    let error_json = serde_json::json!({
        "status": "error",
        "code": code,
        "message": message
    });
    
    serde_json::to_string(&error_json).unwrap_or_else(|_| {
        r#"{"status":"error","code":"UnknownError","message":"Failed to serialize error response"}"#.to_string()
    })
}
