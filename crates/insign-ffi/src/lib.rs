//! FFI bindings for insign-core
//!
//! This crate exports a C ABI for the Insign DSL compiler, allowing
//! integration with Kotlin/JVM applications like Spigot plugins.

use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::slice;

use insign_core::compile;
use serde_json;

/// Input format for JSON compilation
#[derive(serde::Deserialize)]
struct CompileInput {
    pos: [i32; 3],
    text: String,
}

/// Returns the ABI version of the library
#[no_mangle]
pub extern "C" fn insign_abi_version() -> u32 {
    1
}

/// Compiles input JSON to output JSON via C ABI
/// 
/// # Arguments
/// * `input_ptr` - Pointer to UTF-8 JSON input (array of {pos: [x,y,z], text: "..."})
/// * `input_len` - Length of input in bytes
/// * `output_ptr` - Pointer to receive allocated output string pointer
/// * `output_len` - Pointer to receive length of output string
/// 
/// # Returns
/// * 0 on success, non-zero on error
/// * Always allocates output (either success JSON or error JSON)
/// * Caller must free the output with insign_free
#[no_mangle]
pub extern "C" fn insign_compile_json(
    input_ptr: *const c_char,
    input_len: usize,
    output_ptr: *mut *mut c_char,
    output_len: *mut usize,
) -> c_int {
    // Validate input parameters
    if input_ptr.is_null() || output_ptr.is_null() || output_len.is_null() {
        return allocate_error_output(
            output_ptr,
            output_len,
            "Invalid null pointer parameters",
        );
    }

    // Convert input to Rust string
    let input_str = unsafe {
        let input_slice = slice::from_raw_parts(input_ptr as *const u8, input_len);
        match std::str::from_utf8(input_slice) {
            Ok(s) => s,
            Err(_) => {
                return allocate_error_output(
                    output_ptr,
                    output_len,
                    "Input is not valid UTF-8",
                )
            }
        }
    };

    // Parse JSON input
    let input_array: Vec<CompileInput> = match serde_json::from_str(input_str) {
        Ok(arr) => arr,
        Err(e) => {
            return allocate_error_output(
                output_ptr,
                output_len,
                &format!("JSON parse error: {}", e),
            )
        }
    };

    // Convert to insign-core format
    let units: Vec<([i32; 3], String)> = input_array
        .into_iter()
        .map(|input| (input.pos, input.text))
        .collect();

    // Compile using insign-core
    match compile(&units) {
        Ok(dsl_map) => {
            // Success - serialize output
            match serde_json::to_string(&dsl_map) {
                Ok(json) => allocate_success_output(output_ptr, output_len, &json),
                Err(e) => allocate_error_output(
                    output_ptr,
                    output_len,
                    &format!("JSON serialization error: {}", e),
                ),
            }
        }
        Err(e) => {
            // Compilation error - return structured error JSON
            let error_json = serde_json::json!({
                "status": "error",
                "code": "CompilationError",
                "message": format!("{}", e)
            });
            match serde_json::to_string(&error_json) {
                Ok(json) => {
                    allocate_output(output_ptr, output_len, &json);
                    1 // Return error code
                }
                Err(_) => allocate_error_output(
                    output_ptr,
                    output_len,
                    "Failed to serialize error response",
                ),
            }
        }
    }
}

/// Helper function to allocate successful output
fn allocate_success_output(
    output_ptr: *mut *mut c_char,
    output_len: *mut usize,
    json: &str,
) -> c_int {
    allocate_output(output_ptr, output_len, json);
    0 // Success code
}

/// Helper function to allocate error output
fn allocate_error_output(
    output_ptr: *mut *mut c_char,
    output_len: *mut usize,
    message: &str,
) -> c_int {
    let error_json = serde_json::json!({
        "status": "error",
        "code": "FFIError",
        "message": message
    });
    let json = serde_json::to_string(&error_json).unwrap_or_else(|_| {
        r#"{"status":"error","code":"FFIError","message":"Unknown error"}"#.to_string()
    });
    allocate_output(output_ptr, output_len, &json);
    1 // Error code
}

/// Helper function to allocate output string
fn allocate_output(output_ptr: *mut *mut c_char, output_len: *mut usize, content: &str) {
    unsafe {
        let c_string = CString::new(content).unwrap();
        let len = c_string.as_bytes().len();
        let ptr = libc::malloc(len + 1) as *mut c_char;
        ptr.copy_from(c_string.as_ptr(), len + 1);
        *output_ptr = ptr;
        *output_len = len;
    }
}

/// Frees memory allocated by insign_compile_json
/// 
/// # Arguments
/// * `ptr` - Pointer returned by insign_compile_json
/// * `len` - Length returned by insign_compile_json (ignored, but kept for API consistency)
#[no_mangle]
pub extern "C" fn insign_free(ptr: *mut c_void, _len: usize) {
    if !ptr.is_null() {
        unsafe {
            libc::free(ptr);
        }
    }
}
