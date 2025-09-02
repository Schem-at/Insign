# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial implementation of Insign DSL compiler
- Multi-target architecture: Core library, FFI, and WASM
- Comprehensive test suite with parity validation
- GitHub Actions CI/CD pipeline
- Security auditing and dependency management

### Changed
- N/A

### Deprecated
- N/A

### Removed
- N/A

### Fixed
- N/A

### Security
- Added cargo-deny configuration for supply chain security
- Added regular security audits via GitHub Actions

## [0.1.0] - 2024-XX-XX

### Added
- **Core Library (`insign-core`)**:
  - Complete DSL parser for Minecraft build regions and metadata
  - Support for relative coordinate (`rc`) and absolute coordinate (`ac`) boxes
  - Boolean expressions with union operations (`+`)
  - Feature-gated advanced boolean operations (`-`, `&`, `^`) behind `boolean_ops` feature
  - Metadata system with wildcards and global scope
  - Deterministic JSON output with stable ordering
  - Comprehensive error handling with diagnostic information

- **FFI Library (`insign-ffi`)**:
  - C ABI interface for Kotlin/JVM integration
  - JSON-in → JSON-out contract
  - Memory-safe allocation and deallocation
  - Cross-platform builds (Linux, macOS, Windows)

- **WASM Library (`insign-wasm`)**:
  - WebAssembly bindings for browser and Node.js
  - Same JSON interface as FFI for consistency
  - Built with wasm-bindgen and wasm-pack
  - Console logging for debugging

- **Testing & Quality Assurance**:
  - 125+ unit tests covering all functionality
  - Integration tests for CLI tool
  - Parity tests ensuring FFI and WASM produce identical outputs
  - Golden tests with fixture validation
  - Property-based testing with proptest
  - Snapshot testing with insta

- **Build & Development Tools**:
  - Comprehensive parity test suite
  - Cross-platform build scripts
  - CLI tool for development and testing
  - Multiple fixture test cases

### Technical Details
- **Supported Platforms**: Linux (x86_64), macOS (x86_64, ARM64), Windows (x86_64)
- **Minimum Rust Version**: 1.70.0
- **Dependencies**: Minimal and carefully selected
- **Output Format**: Deterministic JSON with lexicographic key ordering
