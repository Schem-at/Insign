#!/usr/bin/env node
/**
 * WASM runner for insign parity testing.
 * Loads the insign-wasm module and calls the compile_json function.
 */

const fs = require('fs');
const path = require('path');

// Check if we have the right number of arguments
if (process.argv.length !== 3) {
    console.error('Usage: wasm_runner.js <input_file>');
    process.exit(1);
}

const inputFile = process.argv[2];

// Check if input file exists
if (!fs.existsSync(inputFile)) {
    console.error(`Error: Input file '${inputFile}' not found`);
    process.exit(1);
}

// Path to the WASM package
const wasmPkgPath = path.join(__dirname, '../../crates/insign-wasm/pkg');

// Check if WASM package exists
if (!fs.existsSync(wasmPkgPath)) {
    console.error(`Error: WASM package not found at '${wasmPkgPath}'`);
    console.error('Run the following commands first:');
    console.error('  cd crates/insign-wasm');
    console.error('  wasm-pack build --release --target nodejs --out-dir pkg --out-name insign');
    process.exit(1);
}

try {
    // Load the WASM module
    const insign = require(wasmPkgPath);
    
    // Check ABI version
    const abiVersion = insign.abi_version();
    if (abiVersion !== 1) {
        console.warn(`Warning: Expected ABI version 1, got ${abiVersion}`);
    }
    
    // Read input file
    const inputJson = fs.readFileSync(inputFile, 'utf8');
    
    // Call the compile_json function
    const result = insign.compile_json(inputJson);
    
    // Print the result
    console.log(result);
    
    // Parse the result to determine exit code
    try {
        const parsed = JSON.parse(result);
        if (parsed.status === 'error') {
            process.exit(1);
        } else {
            process.exit(0);
        }
    } catch (e) {
        // If we can't parse the result, assume success (result is the DSL map)
        process.exit(0);
    }
    
} catch (error) {
    console.error(`Error: ${error.message}`);
    process.exit(1);
}
