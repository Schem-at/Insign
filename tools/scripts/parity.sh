#!/bin/bash
set -euo pipefail

# Insign Parity Test Script
# Tests that FFI and WASM produce identical JSON output for all fixtures

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Counters
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

echo -e "${BLUE}🔧 Insign Parity Test Suite${NC}"
echo "========================================"

# Function to print status
print_status() {
    local status=$1
    local message=$2
    case $status in
        "PASS")
            echo -e "  ${GREEN}✅ PASS${NC} - $message"
            ((PASSED_TESTS++))
            ;;
        "FAIL")
            echo -e "  ${RED}❌ FAIL${NC} - $message"
            ((FAILED_TESTS++))
            ;;
        "INFO")
            echo -e "  ${BLUE}ℹ️  INFO${NC} - $message"
            ;;
        "WARN")
            echo -e "  ${YELLOW}⚠️  WARN${NC} - $message"
            ;;
    esac
}

# Function to run a single parity test
run_parity_test() {
    local fixture_name=$1
    local input_file="fixtures/inputs/${fixture_name}.json"
    local expected_file="fixtures/expected/${fixture_name}.json"
    
    echo -e "${YELLOW}Testing fixture: $fixture_name${NC}"
    
    ((TOTAL_TESTS++))
    
    # Check if input file exists
    if [[ ! -f "$input_file" ]]; then
        print_status "FAIL" "Input file '$input_file' not found"
        return 1
    fi
    
    # Create temporary files for outputs
    local ffi_output=$(mktemp)
    local wasm_output=$(mktemp)
    local ffi_canon=$(mktemp)
    local wasm_canon=$(mktemp)
    
    # Cleanup function
    cleanup_test() {
        rm -f "$ffi_output" "$wasm_output" "$ffi_canon" "$wasm_canon"
    }
    trap cleanup_test RETURN
    
    # Run FFI
    local ffi_exit=0
    if ! python3 tools/parity/ffi_runner.py "$input_file" > "$ffi_output" 2>/dev/null; then
        ffi_exit=$?
    fi
    
    # Run WASM
    local wasm_exit=0
    if ! node tools/parity/wasm_runner.js "$input_file" > "$wasm_output" 2>/dev/null; then
        wasm_exit=$?
    fi
    
    # Check if exit codes match
    if [[ $ffi_exit -ne $wasm_exit ]]; then
        print_status "FAIL" "Exit code mismatch - FFI: $ffi_exit, WASM: $wasm_exit"
        return 1
    fi
    
    # Canonicalize outputs using jq
    if ! jq -S . "$ffi_output" > "$ffi_canon" 2>/dev/null; then
        print_status "FAIL" "FFI output is not valid JSON"
        echo "FFI output:"
        cat "$ffi_output"
        return 1
    fi
    
    if ! jq -S . "$wasm_output" > "$wasm_canon" 2>/dev/null; then
        print_status "FAIL" "WASM output is not valid JSON"
        echo "WASM output:"
        cat "$wasm_output"
        return 1
    fi
    
    # Compare canonical outputs
    if ! diff -u "$ffi_canon" "$wasm_canon" >/dev/null; then
        print_status "FAIL" "Output mismatch between FFI and WASM"
        echo "FFI output:"
        cat "$ffi_canon"
        echo "WASM output:"
        cat "$wasm_canon"
        echo "Diff:"
        diff -u "$ffi_canon" "$wasm_canon" || true
        return 1
    fi
    
    # If expected file exists, compare against it
    if [[ -f "$expected_file" ]]; then
        local expected_canon=$(mktemp)
        trap "rm -f $expected_canon; cleanup_test" RETURN
        
        if ! jq -S . "$expected_file" > "$expected_canon" 2>/dev/null; then
            print_status "WARN" "Expected file is not valid JSON"
        elif ! diff -u "$expected_canon" "$ffi_canon" >/dev/null; then
            print_status "FAIL" "Output doesn't match expected result"
            echo "Expected:"
            cat "$expected_canon"
            echo "Actual:"
            cat "$ffi_canon"
            echo "Diff:"
            diff -u "$expected_canon" "$ffi_canon" || true
            return 1
        else
            print_status "PASS" "Output matches expected result"
        fi
    else
        print_status "PASS" "FFI and WASM outputs are identical"
    fi
    
    return 0
}

echo -e "${BLUE}Step 1: Building all targets${NC}"

# Build core
print_status "INFO" "Building insign-core..."
if ! cargo build -p insign-core --release >/dev/null 2>&1; then
    print_status "FAIL" "Failed to build insign-core"
    exit 1
fi
print_status "PASS" "insign-core built successfully"

# Build FFI
print_status "INFO" "Building insign-ffi..."
if ! cargo build -p insign-ffi --release >/dev/null 2>&1; then
    print_status "FAIL" "Failed to build insign-ffi"
    exit 1
fi
print_status "PASS" "insign-ffi built successfully"

# Build WASM
print_status "INFO" "Building insign-wasm..."
if ! (cd crates/insign-wasm && wasm-pack build --release --target nodejs --out-dir pkg --out-name insign >/dev/null 2>&1); then
    print_status "FAIL" "Failed to build insign-wasm"
    exit 1
fi
print_status "PASS" "insign-wasm built successfully"

echo
echo -e "${BLUE}Step 2: Running parity tests${NC}"

# Test fixtures
FIXTURES=(
    "a_basic"
    "b_named_multi"
    "c_wildcards_global"
    "d_union_expr"
    "e_error_conflict"
    "f_multiline"
)

for fixture in "${FIXTURES[@]}"; do
    run_parity_test "$fixture"
    echo
done

echo "========================================"
echo -e "${BLUE}📊 Parity Test Results${NC}"
echo -e "   ${GREEN}✅ Passed:${NC} $PASSED_TESTS"
echo -e "   ${RED}❌ Failed:${NC} $FAILED_TESTS"
echo -e "   ${BLUE}📝 Total:${NC}  $TOTAL_TESTS"

if [[ $FAILED_TESTS -eq 0 ]]; then
    echo -e "${GREEN}🎉 All parity tests passed!${NC}"
    exit 0
else
    echo -e "${RED}💥 $FAILED_TESTS test(s) failed.${NC}"
    exit 1
fi
