# Motivation & context (why Rust + FFI + WASM)

* **Single source of truth.** Insign is a *deterministic compiler* from a sign-friendly DSL (list of `(pos, text)` tuples) to a **regions-root JSON** (AABBs + metadata). Implementing it **once in Rust** avoids drift between client/server tools.

* **Three targets, one API.**

    * **Rust crate (`insign-core`)**: the canonical implementation used for unit/golden tests and CLI/debug tooling.
    * **FFI cdylib (`insign-ffi`)**: exposes a tiny **C ABI** (JSON-in → JSON-out) so a **Kotlin Spigot plugin** can call Insign from inside the game server without re-implementing the parser/evaluator.
    * **WASM (`insign-wasm`)**: exports the *same* JSON-in/JSON-out function for **Schematio** (web/Node) so builds and annotations can be compiled client-side or in web workers with no native dependencies.

* **Why not re-implement in Kotlin?** The DSL has non-trivial lexing (statement boundaries across nested brackets/JSON), boolean region semantics, and strict determinism rules. Duplicating that logic invites divergence. Rust gives performance + safety; Kotlin and the browser consume a stable, simple **string ↔ string** interface.

* **Interface contract (consistent across all targets).**

    * Input: UTF-8 JSON array of `{ "pos": [x,y,z], "text": "..." }`.
    * Output: UTF-8 JSON — either the **regions-root** object on success, or a **structured error JSON** on failure.
    * This keeps bindings thin (no complex struct marshalling) and makes parity tests trivial.

* **Operational fit.**

    * **Spigot plugin (Kotlin):** loads platform-specific native lib and calls `compile_json` off the main thread; returns JSON to game logic.
    * **Schematio:** uses WASM in the browser/Node to compile signs/books uploaded from worlds; same outputs as server, enabling identical downstream tooling.

* **Quality & parity.** A shared golden-fixture suite drives all targets. A local parity harness runs the *same inputs* through FFI and WASM and diffs the canonicalized JSON to guarantee identical behavior.

* **Future-proof.** Keeping the FFI/WASM surface to **one function** (plus `free`/`version` for FFI) lets you evolve internal features (full booleans, re-boxing strategies) without changing downstream integrations.

---

# INSIGN – Local Build & Interface Parity Plan (Rust + FFI + WASM)

This plan gets Insign compiling to three targets and validates that the **JSON↔JSON** interface is consistent across them—**locally**, before any plugin or CI wiring.

---

## 0) Outcomes

* A Rust **core** crate builds and passes tests.
* A minimal **FFI cdylib** exports a single JSON interface (C ABI).
* A minimal **WASM** build (wasm-bindgen/wasm-pack) exports the same JSON interface.
* A small **local parity check** runs the same inputs through FFI and WASM and compares outputs byte-for-byte (or via `jq` canonicalization).

---

## 1) Workspace layout (no code, just structure)

```
insign/
├─ Cargo.toml                # [workspace] with three members
├─ crates/
│  ├─ insign-core/           # pure Rust library (no platform specifics)
│  ├─ insign-ffi/            # cdylib crate; exposes C ABI using core
│  └─ insign-wasm/           # wasm crate; exposes wasm-bindgen API using core
├─ fixtures/
│  ├─ inputs/
│  │  ├─ a_basic.json        # [(pos,text)] list
│  │  └─ b_wildcards.json
│  └─ expected/
│     ├─ a_basic.json        # regions-root JSON
│     └─ b_wildcards.json
└─ tools/
   ├─ parity/                # tiny harnesses (WASM + FFI) to run inputs & compare (language of your choice)
   └─ scripts/               # helper shell scripts to build & test locally
```

> Keep **all interface tests** driven by files in `fixtures/`. Both FFI and WASM consume exactly the same JSON inputs and must produce identical JSON outputs.

---

## 2) Interface contract (single function, JSON in/out)

**Input JSON (array of tuples):**

```jsonc
[
  { "pos": [10,64,10], "text": "@rc([0,0,0],[3,2,1])\n#doc.label=\"Patch A\"" },
  { "pos": [0,64,0],  "text": "@cpu.core=ac([100,70,-20],[104,72,-18])\n#cpu.core:logic.clock_hz=4" }
]
```

**Success output:** regions-root JSON (per README/spec).
**Error output (always JSON):**

```json
{
  "status": "error",
  "code": "ParseError",
  "message": "unknown region 'foo' in expression",
  "location": { "tuple_index": 1, "statement_index": 0 }
}
```

> **Rule:** both FFI and WASM **must** return a UTF-8 JSON string for **both** success and failure. No binary blobs. This keeps Kotlin and browsers simple.

---

## 3) Target definitions (what each crate must export)

### A) `insign-core` (pure Rust)

* Public `compile(units: Vec<( [i32;3], String )>) -> Result<DslMap, Error>`.
* Deterministic JSON via `serde_json::to_string` (and stable key ordering internally).

### B) `insign-ffi` (cdylib)

* Build type: `cdylib`.
* **C ABI** functions:

    * `insign_abi_version() -> uint32_t`
    * `insign_compile_json(in_ptr, in_len, out_ptr, out_len) -> int32_t`
    * `insign_free(ptr, len)`
* Contract:

    * On **success**: return 0; allocate `*out_ptr/*out_len` with a UTF-8 JSON success string.
    * On **error**: return non-zero; allocate `*out_ptr/*out_len` with a UTF-8 JSON **error** string.
    * Always free with `insign_free`.

### C) `insign-wasm` (wasm-bindgen / wasm-pack)

* Build target: `wasm32-unknown-unknown`.
* Exported function (JS): `compile_json(input: string): string` (returns JSON string; throw only on catastrophic internal failure).
* Two `wasm-pack` targets for local testing:

    * `--target nodejs` for Node parity tests.
    * Optionally `--target web` later for browsers.

---

## 4) Local environment setup

```bash
# Rust toolchain & targets
rustup toolchain install stable
rustup default stable
rustup target add wasm32-unknown-unknown

# wasm-pack
curl -sSf https://rustwasm.github.io/wasm-pack/installer/init.sh | sh

# Helpers
brew install jq || sudo apt-get install -y jq   # any OS equivalent is fine
node -v || brew install node || sudo apt-get install -y nodejs # for WASM node tests
```

---

## 5) Build steps (local)

### A) Build core (debug + release)

```bash
cargo build -p insign-core
cargo build -p insign-core --release
cargo test  -p insign-core
```

### B) Build FFI (host triple)

```bash
cargo build -p insign-ffi --release
# artifacts:
# - Linux:   target/release/libinsign_ffi.so
# - macOS:   target/release/libinsign_ffi.dylib
# - Windows: target/release/insign_ffi.dll
```

### C) Build WASM (Node target)

```bash
cd crates/insign-wasm
wasm-pack build --release --target nodejs --out-dir pkg --out-name insign
cd ../../
# artifacts in crates/insign-wasm/pkg/ (JS glue + .wasm)
```

---

## 6) Parity test plan (no implementation code here, just the flow)

> Implement tiny runners in any language you prefer (Node + Python are quickest), placed under `tools/parity/`. They should be trivial: read JSON, call function, print JSON.

### Runner responsibilities

* **FFI runner**:

    1. Load the cdylib (`libinsign_ffi.*`) using your language’s FFI (e.g., Python `ctypes`, Node `ffi-napi`, or a tiny Kotlin/JNA smoke if you prefer).
    2. Read an input file from `fixtures/inputs/*.json`.
    3. Call `insign_compile_json` with the UTF-8 bytes; get back `out_ptr/out_len`; convert to string; call `insign_free`.
    4. Print the JSON to `stdout`.

* **WASM runner (Node)**:

    1. `require()`/`import` the built `pkg/` from `insign-wasm`.
    2. Read the same input file.
    3. Call `compile_json(inputString)`; capture returned string.
    4. Print the JSON to `stdout`.

### Parity script (shell orchestration)

A simple script in `tools/scripts/parity.sh` should:

1. **Build everything** (steps in §5).
2. For each fixture in `fixtures/inputs`:

    * Run FFI runner → capture `out_ffi.json`
    * Run WASM runner → capture `out_wasm.json`
    * Canonicalize both (e.g., `jq -S .`) and **diff**:

      ```bash
      jq -S . out_ffi.json  > ffi.canon.json
      jq -S . out_wasm.json > wasm.canon.json
      diff -u ffi.canon.json wasm.canon.json
      ```
    * If different, print both and **exit 1**.
3. If any file in `fixtures/expected/` exists for the input name, also compare:

   ```bash
   diff -u ffi.canon.json <(jq -S . fixtures/expected/<name>.json)
   ```

**Success criteria:** All fixture inputs produce **byte-identical canonical JSON** across FFI and WASM (and match `expected/` when present).

---

## 7) What to test first (fixtures to create)

Create small, readable inputs/expectations:

1. **a\_basic.json** — one anonymous `@rc` + two `#` metadata lines.
2. **b\_named\_multi.json** — `@region=rc(...) + rc(...)` and targeted `#region:key=...`.
3. **c\_wildcards\_global.json** — `#cpu.*:logic.clock_hz=4` and `#$global:io.bus_width=8`.
4. **d\_union\_expr.json** — `@core=a+b` (union only).
5. **e\_error\_conflict.json** — two tuples conflicting on the same `<region,key>` (expect error JSON).
6. **f\_multiline.json** — long `@...` across multiple lines + metadata.

---

## 8) Versioning & plumbing (keep both sides honest)

* The FFI returns `insign_abi_version()` (e.g., `1`) and the WASM module exports a `abiVersion(): number` (or hard-coded constant) so your parity script can assert **ABI parity** before running.
* Optionally include `"tooling":{"insign":"0.1.0"}` inside the success JSON for quick eyeballing.

---

## 9) Platform notes (local dev)

* On macOS/Linux: you’ll test FFI with `libinsign_ffi.dylib/.so`.
* On Windows: `insign_ffi.dll`.
* For a single-machine parity run, **stick to one OS/arch** (your workstation) first; cross-compile/CI later.

---

## 10) When everything passes

* You now have a **stable JSON contract** used identically by FFI and WASM.
* Kotlin (Spigot) can load the FFI and just pass strings through.
* Web tooling (or Node scripts) can load the WASM and do the same.

---

### Quick checklist

* [ ] Workspace with `insign-core`, `insign-ffi`, `insign-wasm` created
* [ ] `insign-core` compiles & unit tests green
* [ ] `insign-ffi` builds a cdylib on your machine
* [ ] `insign-wasm` builds with `wasm-pack --target nodejs`
* [ ] `fixtures/inputs/*` + `fixtures/expected/*` added
* [ ] `tools/parity/*` runners created (FFI + WASM), return JSON strings
* [ ] `tools/scripts/parity.sh` runs both and diffs canonical JSON
* [ ] Parity is green on your workstation

---

**Tip:** keep the interface tiny (one function). The moment you want to add more, extend the **input JSON** with a `"mode":"compile"` field and route internally—so your FFI/WASM surface stays fixed.
