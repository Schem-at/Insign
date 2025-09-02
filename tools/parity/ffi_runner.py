#!/usr/bin/env python3
"""
FFI runner for insign parity testing.
Loads the insign-ffi cdylib and calls the C ABI functions.
"""

import ctypes
import sys
import os
import platform
import json

def get_library_path():
    """Get the path to the insign-ffi library based on platform."""
    if platform.system() == "Darwin":  # macOS
        return "target/release/libinsign_ffi.dylib"
    elif platform.system() == "Linux":
        return "target/release/libinsign_ffi.so"
    elif platform.system() == "Windows":
        return "target/release/insign_ffi.dll"
    else:
        raise RuntimeError(f"Unsupported platform: {platform.system()}")

def main():
    if len(sys.argv) != 2:
        print("Usage: ffi_runner.py <input_file>", file=sys.stderr)
        sys.exit(1)
    
    input_file = sys.argv[1]
    
    # Read input file
    try:
        with open(input_file, 'r') as f:
            input_json = f.read()
    except Exception as e:
        print(f"Error reading input file: {e}", file=sys.stderr)
        sys.exit(1)
    
    # Load the library
    try:
        lib_path = get_library_path()
        if not os.path.exists(lib_path):
            print(f"Library not found at {lib_path}. Run 'cargo build -p insign-ffi --release' first.", file=sys.stderr)
            sys.exit(1)
            
        lib = ctypes.CDLL(lib_path)
    except Exception as e:
        print(f"Error loading library: {e}", file=sys.stderr)
        sys.exit(1)
    
    # Define function signatures
    lib.insign_abi_version.restype = ctypes.c_uint32
    lib.insign_abi_version.argtypes = []
    
    lib.insign_compile_json.restype = ctypes.c_int32
    lib.insign_compile_json.argtypes = [
        ctypes.c_char_p,      # input_ptr
        ctypes.c_size_t,      # input_len
        ctypes.POINTER(ctypes.c_char_p),  # output_ptr
        ctypes.POINTER(ctypes.c_size_t),  # output_len
    ]
    
    lib.insign_free.restype = None
    lib.insign_free.argtypes = [ctypes.c_void_p, ctypes.c_size_t]
    
    # Check ABI version
    abi_version = lib.insign_abi_version()
    if abi_version != 1:
        print(f"Warning: Expected ABI version 1, got {abi_version}", file=sys.stderr)
    
    # Prepare input
    input_bytes = input_json.encode('utf-8')
    input_ptr = ctypes.c_char_p(input_bytes)
    input_len = ctypes.c_size_t(len(input_bytes))
    
    # Prepare output pointers
    output_ptr = ctypes.c_char_p()
    output_len = ctypes.c_size_t()
    
    # Call the function
    try:
        result = lib.insign_compile_json(
            input_ptr,
            input_len,
            ctypes.byref(output_ptr),
            ctypes.byref(output_len)
        )
        
        # Get the output string
        if output_ptr:
            output_bytes = ctypes.string_at(output_ptr, output_len.value)
            output_json = output_bytes.decode('utf-8')
            
            # Free the memory
            lib.insign_free(output_ptr, output_len)
            
            # Print the result
            print(output_json)
            
            # Exit with the result code
            sys.exit(result)
        else:
            print("Error: No output received from FFI function", file=sys.stderr)
            sys.exit(1)
            
    except Exception as e:
        print(f"Error calling FFI function: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
