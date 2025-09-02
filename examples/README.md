# Insign CLI Examples

This directory contains example JSONL input files for testing the Insign CLI tool.

## Usage

```bash
# Build the CLI
cargo build --bin insign-cli

# Compile JSONL input to regions+metadata JSON
./target/debug/insign-cli [OPTIONS] [FILE]

# If no FILE is provided, reads from stdin
cat input.jsonl | ./target/debug/insign-cli --pretty
```

## Options

- `--pretty` / `-p`: Pretty-print JSON output
- `--help` / `-h`: Show help information
- `--version` / `-V`: Show version

## Exit Codes

- `0`: Success
- `1`: Input parsing error (file not found, invalid JSONL format)
- `2`: Compilation error (DSL syntax errors, conflicts, etc.)

## Example Files

### `simple_signs.jsonl`
Basic example with anonymous region + metadata and named region with global metadata.

```bash
./target/debug/insign-cli --pretty examples/simple_signs.jsonl
```

### `complex_example.jsonl`
More complex example with:
- Named regions and accumulators
- Boolean union expressions
- Wildcard and global metadata
- Multi-tuple coordination

```bash
./target/debug/insign-cli --pretty examples/complex_example.jsonl
```

### `error_case.jsonl`
Example that demonstrates error handling with metadata conflicts.

```bash
./target/debug/insign-cli --pretty examples/error_case.jsonl
```

Expected to fail with exit code 2 and structured error output.

## JSONL Format

Each line must be valid JSON with this structure:

```json
{"pos": [x, y, z], "text": "DSL content"}
```

- `pos`: 3D coordinate array representing the sign/block position
- `text`: Insign DSL statements (may contain newlines)

Empty lines are ignored.
