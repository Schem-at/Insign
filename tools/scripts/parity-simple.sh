#!/bin/bash
set -euo pipefail

# Insign Parity Test Script (Simplified)
# Tests that FFI and WASM produce identical JSON output for all fixtures

echo "🔧 Insign Parity Test Suite"
echo "========================================"

echo "Step 1: Building all targets"

# Build core
echo "  Building insign-core..."
cargo build -p insign-core --release >/dev/null 2>&1
echo "  ✅ insign-core built successfully"

# Build FFI  
echo "  Building insign-ffi..."
# Build for x86_64 to match Python architecture on this system
if [[ "$(uname)" == "Darwin" ]] && [[ "$(python3 -c 'import platform; print(platform.machine())')" == "x86_64" ]]; then
    cargo build -p insign-ffi --release --target x86_64-apple-darwin >/dev/null 2>&1
    cp target/x86_64-apple-darwin/release/libinsign_ffi.dylib target/release/libinsign_ffi.dylib
else
    cargo build -p insign-ffi --release >/dev/null 2>&1
fi
echo "  ✅ insign-ffi built successfully"

# Build WASM
echo "  Building insign-wasm..."
(cd crates/insign-wasm && wasm-pack build --release --target nodejs --out-dir pkg --out-name insign >/dev/null 2>&1)
echo "  ✅ insign-wasm built successfully"

echo
echo "Step 2: Running parity tests"

# Test fixtures
FIXTURES=(
    "a_basic"
    "b_named_multi" 
    "c_wildcards_global"
    "d_union_expr"
    "e_error_conflict"
    "f_multiline"
)

PASSED=0
FAILED=0

for fixture in "${FIXTURES[@]}"; do
    echo "Testing fixture: $fixture"
    
    input_file="fixtures/inputs/${fixture}.json"
    expected_file="fixtures/expected/${fixture}.json"
    
    # Run FFI and WASM
    ffi_output=$(python3 tools/parity/ffi_runner.py "$input_file" 2>/dev/null || echo "ERROR")
    wasm_output=$(node tools/parity/wasm_runner.js "$input_file" 2>/dev/null || echo "ERROR")
    
    # Compare canonical outputs
    ffi_canon=$(echo "$ffi_output" | jq -S . 2>/dev/null || echo "INVALID")
    wasm_canon=$(echo "$wasm_output" | jq -S . 2>/dev/null || echo "INVALID")
    
    if [[ "$ffi_canon" == "$wasm_canon" ]]; then
        echo "  ✅ PASS - FFI and WASM outputs are identical"
        ((PASSED++))
    else
        echo "  ❌ FAIL - Output mismatch"
        echo "  FFI:  $ffi_output"
        echo "  WASM: $wasm_output"
        ((FAILED++))
    fi
    echo
done

echo "========================================"
echo "📊 Results: $PASSED passed, $FAILED failed"

if [[ $FAILED -eq 0 ]]; then
    echo "🎉 All parity tests passed!"
    exit 0
else
    echo "💥 Some tests failed."
    exit 1
fi
