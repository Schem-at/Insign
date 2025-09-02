# Insign — a tiny DSL for Minecraft build regions & metadata

[![CI](https://github.com/Schem-at/Insign/workflows/CI/badge.svg)](https://github.com/Schem-at/Insign/actions/workflows/ci.yml)
[![Security](https://github.com/Schem-at/Insign/workflows/Security/badge.svg)](https://github.com/Schem-at/Insign/actions/workflows/security.yml)
[![Crates.io](https://img.shields.io/crates/v/insign-core.svg)](https://crates.io/crates/insign-core)
[![Documentation](https://docs.rs/insign-core/badge.svg)](https://docs.rs/insign-core)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

> **Goal:** author compact annotations on **signs/books** (or any text source) that compile into a deterministic **regions + metadata** structure you can ship with schematics and tools.

* **Regions** = unions of axis-aligned boxes (AABBs) keyed by IDs like `cpu.core`
* **Metadata** = namespaced key/values attached to regions (plus `$global` and wildcards like `cpu.*`)
* **Authoring constraints:** terse on signs, multi-line friendly, stable & deterministic

This repository currently contains the **spec and examples**. Implementation comes next.

---

## Why?

* **Portable context** for builds: label sub-assemblies, bus lines, rooms—precisely and repeatably.
* **Machine-readable**: consumers (renderers, validators, pipelines) can query regions and inherited metadata.
* **Sign-friendly**: minimal punctuation, statement markers, multi-line allowed.

---

## Library I/O (language-agnostic)

### Input

An **ordered array** of tuples: `([sx, sy, sz], text)`.

```json
[
  {
    "pos": [10, 64, 10],
    "text": "@rc([0,0,0],[3,2,1])\n#doc.label=\"Patch A\""
  },
  {
    "pos": [0, 64, 0],
    "text": "@cpu.core=ac([100,70,-20],[104,72,-18])\n#cpu.core:logic.clock_hz=4\n#cpu.*:power.budget=\"low\"\n#$global:io.bus_width=8"
  }
]
```

* `pos` is the source block’s world coords (sign/lectern).
* `text` is raw DSL; newlines allowed.
* Order is used **only** to derive stable IDs for anonymous regions; it must not affect semantics.

### Output (conceptual JSON)

```json
{
  "$global": { "metadata": { "io.bus_width": 8 } },
  "cpu.*":   { "metadata": { "power.budget": "low" } },

  "__anon:0:0": {
    "bounding_boxes": [[[10,64,10],[13,66,11]]],
    "metadata": { "doc.label": "Patch A" }
  },

  "cpu.core": {
    "bounding_boxes": [[[100,70,-20],[104,72,-18]]],
    "metadata": { "logic.clock_hz": 4 }
  }
}
```

**Notes**

* `rc` is **relative** to `pos`; `ac` is **absolute**.
* Bounds are **inclusive**; each box is stored as a **pair of vec3** corners, normalized per axis.
* Anonymous regions appear **only if** they received metadata (recommended).

---

## Authoring Syntax (sign-friendly)

* **Geometry starts with `@`**
* **Metadata starts with `#`**

A statement **starts** at `@` or `#` and **ends** right before the next `@`/`#` (or end of text). Newlines are allowed *inside* a statement.

### Geometry (`@…`)

**Named regions (unified form)**

```
@<region>=rc([x1,y1,z1],[x2,y2,z2])   // relative to this tuple’s pos
@<region>=ac([x1,y1,z1],[x2,y2,z2])   // absolute world coords
@<region>=<expr>                       // boolean expression over regions
```

**Anonymous regions (no `=`; sign-local only)**

```
@rc([x1,y1,z1],[x2,y2,z2])
@ac([x1,y1,z1],[x2,y2,z2])
@def(<expr>)
```

**Booleans (Phase 0 / MVP)**

```
<expr> := term { + term }*
term   := <region> | ( <expr> )
```

* Operators: **`+` union** only in Phase 0.
* Other ops (`-`, `&`, `^`) are reserved for a later phase.

**Current region `.`**

* Within a **single tuple**, the most recent geometry statement (named or anonymous).
* No current region across tuples.

### Metadata (`#…`)

**Attach to current region (this tuple):**

```
#key=<json>
```

**Attach to an explicit target (dense):**

```
#<target>:<key>=<json>
```

Where `<target>` is:

* a region ID (`foo` or `foo.bar`)
* a wildcard prefix (`prefix.*`)
* `$global`

**Values:** strict JSON (string/number/bool/null/array/object).
*(No computed value functions in v0.1; reserved for future.)*

---

## Inheritance & Determinism

* **Read-time precedence per key:** **Exact region** > **longest matching wildcard(s)** > **\$global**.
* **Conflicts:** different values for the same `<named target, key>` across tuples → **compile error** (identical duplicates allowed).
* A named region is either:

    * **Accumulator** (one or more `rc/ac` append boxes), **or**
    * **Defined** (`@<region>=<expr>`).
      Mixing both modes is an error.
* Anonymous region IDs derive from `(tuple_index, statement_index)`—never random UUIDs.
* Deterministic output ordering is recommended: `$global`, then wildcards (lexicographic), then regions (lexicographic).

---

## Examples

### 1) Anonymous region + inline metadata (relative)

```
@rc([0,64,0],
    [31,72,15])
#doc.label="Data Loop"
#logic.clock_hz=4
```

### 2) Named accumulators split across signs

```
@dataloop=rc([0,64,0],[31,72,15])
@dataloop.alu=rc([8,64,8],[22,70,14])
@dataloop.registers=rc([2,64,2],[12,69,6])+rc([14,64,2],[24,69,6])

#dataloop:doc.label="Data Loop"
#dataloop.registers:logic.word_size=8
```

### 3) Boolean union (Phase 0)

```
@core=dataloop.alu+dataloop.registers
#core:doc.note="ALU ∪ registers"
```

### 4) Absolute span + targeted metadata

```
@cpu.bus=ac([100,70,-20],[132,78,-5])
#cpu.bus:io.direction="east-west"
```

### 5) Wildcards & global metadata

```
#cpu.*:logic.clock_hz=4
#$global:io.bus_width=8
```

### 6) Multi-line statements are fine

```
@dataloop.registers=rc([2,64,2],
                       [12,69,6])
                    + rc([14,64,2],
                         [24,69,6])
#dataloop.registers:doc.note="two banks"
```

---

## Formal Grammar (EBNF, v0.1)

**Lexical**

```
digit      = "0"…"9" ;
int        = ["-"], digit, { digit } ;
region-id  = 1*( ALNUM | "_" | "." ) ;      // [A-Za-z0-9_.]+
key        = region-id ;
vec3       = "[", int, ",", int, ",", int, "]" ;
box        = vec3, ",", vec3 ;
```

**Program**

```
input      = { stmt } ;
stmt       = geom | meta ;
```

**Geometry**

```
geom       = "@", ( named-geom | anon-geom ) ;

named-geom = region-id, "=", ( "rc(", box, ")"
                             | "ac(", box, ")"
                             | expr ) ;

anon-geom  = "rc(", box, ")"
           | "ac(", box, ")"
           | "def(", expr, ")" ;

expr       = term, { "+", term } ;          // Phase 0: union only
term       = region-id | "(", expr, ")" ;
```

**Metadata**

```
meta           = "#", ( targeted-meta | current-meta ) ;

targeted-meta  = meta-target, ":", key, "=", json ;
current-meta   = key, "=", json ;

meta-target    = "$global" | region-id | region-id, ".*" ;
json           = RFC 8259 JSON literal ;
```

---

## Installation & Usage

### Rust Library

Add to your `Cargo.toml`:

```toml
[dependencies]
insign-core = "0.1.0"

# For FFI bindings (Kotlin/JVM integration)
insign-ffi = "0.1.0"

# For WASM bindings (Web/Node.js)
insign-wasm = "0.1.0"
```

**Basic usage:**

```rust
use insign_core::compile;

let input = vec![
    ([10, 64, 10], "@rc([0,0,0],[3,2,1])\n#doc.label=\"Patch A\"".to_string()),
];

let result = compile(&input)?;
let json = serde_json::to_string_pretty(&result)?;
println!("{}", json);
```

### CLI Tool

Install from crates.io:

```bash
cargo install insign-core --features=cli
```

Or download pre-built binaries from the [GitHub Releases](https://github.com/Schem-at/Insign/releases) page.

```bash
echo '{"pos": [0,0,0], "text": "@rc([0,0,0],[1,1,1])\\n#test=1"}' | insign-cli --pretty
```

### FFI (Kotlin/JVM)

Download the appropriate native library from [GitHub Releases](https://github.com/Schem-at/Insign/releases):

- Linux: `libinsign_ffi.so`
- macOS: `libinsign_ffi.dylib` 
- Windows: `insign_ffi.dll`

```kotlin
// Example Kotlin integration (requires JNA)
val library = Native.load("insign_ffi", InsignLibrary::class.java)
val json = "[{\"pos\":[0,0,0],\"text\":\"@rc([0,0,0],[1,1,1])\"}]"
val result = library.compile_json(json)
println(result)
```

### WASM (Web/Node.js)

**Node.js:**

```bash
npm install @schem-at/insign-wasm
```

```javascript
const { compile_json } = require('@schem-at/insign-wasm');

const input = JSON.stringify([
  { pos: [0,0,0], text: "@rc([0,0,0],[1,1,1])\n#test=1" }
]);

const result = compile_json(input);
console.log(JSON.parse(result));
```

**Browser:**

```html
<script type="module">
import init, { compile_json } from './pkg/insign.js';

async function run() {
  await init();
  
  const input = JSON.stringify([
    { pos: [0,0,0], text: "@rc([0,0,0],[1,1,1])\n#test=1" }
  ]);
  
  const result = compile_json(input);
  console.log(JSON.parse(result));
}

run();
</script>
```

---

## Development

### Building from Source

```bash
# Clone the repository
git clone https://github.com/Schem-at/Insign.git
cd Insign

# Build all packages
cargo build --all --release

# Run tests
cargo test --all

# Build FFI library
cargo build -p insign-ffi --release

# Build WASM package
cd crates/insign-wasm
wasm-pack build --release --target nodejs --out-dir pkg --out-name insign
```

### Parity Testing

To verify that FFI and WASM produce identical outputs:

```bash
./tools/scripts/parity-simple.sh
```

---

## License

MIT (see `LICENSE` in this repository).
