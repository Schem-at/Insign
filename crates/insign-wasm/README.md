# insign-wasm

[![npm](https://img.shields.io/npm/v/insign-wasm.svg)](https://www.npmjs.com/package/insign-wasm)
[![Crates.io](https://img.shields.io/crates/v/insign-wasm.svg)](https://crates.io/crates/insign-wasm)
[![Documentation](https://docs.rs/insign-wasm/badge.svg)](https://docs.rs/insign-wasm)
[![CI](https://github.com/Schem-at/Insign/workflows/CI/badge.svg)](https://github.com/Schem-at/Insign/actions)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](../../LICENSE)

**WebAssembly bindings for [Insign](../../README.md) — a tiny DSL for Minecraft build regions & metadata.**

Compile compact sign annotations into deterministic region + metadata structures for Minecraft schematics and build tools. Perfect for web applications, Node.js servers, and build pipelines.

## ✨ Quick Start

```bash
# Install the main package
npm install insign-wasm
```

```javascript
const { compile_json } = require('insign-wasm');

// Define regions on a Minecraft sign
const signData = [{
  pos: [100, 64, 200],  // Sign position in world
  text: `
    @castle=rc([0,0,0],[50,20,50])
    #castle:builder="Steve"
    #castle:style="medieval"
  `
}];

// Compile to structured data
const result = compile_json(JSON.stringify(signData));
const regions = JSON.parse(result);

console.log(regions.castle);
// Output: {
//   "bounding_boxes": [[[100,64,200], [150,84,250]]],
//   "metadata": { "builder": "Steve", "style": "medieval" }
// }
```

### 🎮 Try the Interactive Example

After installation, run the comprehensive example:

```bash
# If installed via npm
node node_modules/insign-wasm/example.js

# If building from source
cd crates/insign-wasm
./example.js
```

This example demonstrates:
- ✅ Basic region definitions
- ✅ Multiple regions with global metadata
- ✅ Boolean operations (unions)
- ✅ Error handling
- ✅ Performance testing

## 📦 Installation

### Node.js / npm

```bash
# Main package (Node.js optimized)
npm install insign-wasm

# Target-specific packages for optimal performance:
npm install insign-wasm-web      # Browser ES modules
npm install insign-wasm-bundler  # Webpack/Vite/Rollup
```

### Browser (via CDN)

```html
<!-- Via unpkg CDN -->
<script src="https://unpkg.com/insign-wasm/insign.js"></script>

<!-- Or jsDelivr CDN -->
<script src="https://cdn.jsdelivr.net/npm/insign-wasm/insign.js"></script>

<!-- CDN loader for reliable loading -->
<script type="module">
  import init from 'https://unpkg.com/insign-wasm-web/insign-cdn-loader.js';
  await init();
</script>

<!-- Target-specific versions -->
<script src="https://unpkg.com/insign-wasm-web/insign.js"></script>
```

### Direct Download

Download pre-built WASM packages for all targets from the [GitHub Releases](https://github.com/Schem-at/Insign/releases) page.

### Package Variants

| Package | Best For | Size | Features |
|---------|----------|------|-----------|
| `insign-wasm` | Node.js, servers | ~100KB | Universal init, auto-detection |
| `insign-wasm-web` | Browsers, no bundler | ~95KB | Universal init + CDN loader |
| `insign-wasm-bundler` | Webpack, Vite, etc. | ~98KB | Universal init, bundler optimized |

## 🚀 Usage

### TypeScript Support

Full TypeScript definitions are included:

```typescript
import { abi_version, compile_json } from 'insign-wasm';

interface SignInput {
  pos: [number, number, number];
  text: string;
}

const data: SignInput[] = [{
  pos: [0, 64, 0],
  text: "@spawn=rc([0,0,0],[10,3,10])\n#spawn:safe=true"
}];

const result: string = compile_json(JSON.stringify(data));
const output = JSON.parse(result);
```

### Node.js

```javascript
const { abi_version, compile_json } = require('insign-wasm');

// Check ABI version
console.log('WASM ABI Version:', abi_version());

// Define input data
const input = JSON.stringify([
  {
    pos: [10, 64, 10],
    text: "@rc([0,0,0],[3,2,1])\n#doc.label=\"Patch A\""
  },
  {
    pos: [0, 64, 0], 
    text: "@cpu.core=ac([100,70,-20],[104,72,-18])\n#cpu.core:logic.clock_hz=4\n#cpu.*:power.budget=\"low\"\n#$global:io.bus_width=8"
  }
]);

// Compile and get results
const result = compile_json(input);
const output = JSON.parse(result);

console.log(JSON.stringify(output, null, 2));
```

### Browser (ES Modules)

```html
<!DOCTYPE html>
<html>
<head>
    <title>Insign WASM Example</title>
</head>
<body>
    <script type="module">
        import init, { abi_version, compile_json } from './pkg/insign.js';

        async function run() {
            // Initialize WASM module (universal - auto-detects environment)
            await init();
            
            console.log('WASM ABI Version:', abi_version());

            const input = JSON.stringify([
                {
                    pos: [0, 0, 0],
                    text: "@rc([0,0,0],[5,3,5])\n#structure.name=\"Main Hall\"\n#structure.type=\"building\""
                }
            ]);

            const result = compile_json(input);
            const output = JSON.parse(result);
            
            console.log('Compilation result:', output);
            
            // Display results in the page
            document.body.innerHTML = `
                <h2>Insign WASM Compilation Result</h2>
                <pre>${JSON.stringify(output, null, 2)}</pre>
            `;
        }

        run().catch(console.error);
    </script>
</body>
</html>
```

### Browser (CDN with Reliable Loading)

```html
<!DOCTYPE html>
<html>
<head>
    <title>Insign WASM CDN Example</title>
</head>
<body>
    <script type="module">
        import init, { compile_json } from 'https://unpkg.com/insign-wasm-web/insign-cdn-loader.js';

        async function run() {
            // CDN loader handles WASM path resolution automatically
            await init();
            
            const input = JSON.stringify([
                {
                    pos: [50, 64, -20],
                    text: "@castle=rc([0,0,0],[30,15,25])\n#castle:style=\"gothic\"\n#castle:built=1247"
                }
            ]);

            const result = compile_json(input);
            const output = JSON.parse(result);
            
            document.body.innerHTML = `
                <h2>CDN-Loaded Castle Metadata</h2>
                <pre>${JSON.stringify(output, null, 2)}</pre>
            `;
        }

        run().catch(console.error);
    </script>
</body>
</html>
```

### Browser (Script Tag)

```html
<!DOCTYPE html>
<html>
<head>
    <title>Insign WASM Example (Script)</title>
</head>
<body>
    <script src="./pkg/insign.js"></script>
    <script>
        wasm_bindgen('./pkg/insign_bg.wasm').then(() => {
            console.log('WASM ABI Version:', wasm_bindgen.abi_version());

            const input = JSON.stringify([
                {
                    pos: [100, 64, 200],
                    text: "@redstone.circuit=rc([-5,-1,-5],[5,2,5])\n#redstone.circuit:power.input=\"lever\"\n#redstone.circuit:power.output=\"piston\""
                }
            ]);

            const result = wasm_bindgen.compile_json(input);
            const output = JSON.parse(result);
            
            document.body.innerHTML = `
                <h2>Redstone Circuit Metadata</h2>
                <pre>${JSON.stringify(output, null, 2)}</pre>
            `;
        });
    </script>
</body>
</html>
```

## API Reference

### Functions

#### `abi_version(): number`

Returns the ABI (Application Binary Interface) version of the WASM module. This can be used to verify compatibility between different versions of the library.

#### `compile_json(input: string): string`

Compiles Insign DSL input and returns the result as a JSON string.

**Parameters:**
- `input` - A JSON string containing an array of input objects

**Input Format:**
```typescript
interface CompileInput {
  pos: [number, number, number];  // [x, y, z] world coordinates
  text: string;                   // Raw Insign DSL text
}
```

**Returns:**
- On success: JSON string with compiled region data
- On error: JSON string with error information

**Success Output Format:**
```json
{
  "$global": { "metadata": { ... } },
  "region.name": {
    "bounding_boxes": [[[x1,y1,z1], [x2,y2,z2]], ...],
    "metadata": { ... }
  },
  "__anon:0:0": {
    "bounding_boxes": [[[x1,y1,z1], [x2,y2,z2]]],
    "metadata": { ... }
  }
}
```

**Error Output Format:**
```json
{
  "status": "error",
  "code": "ErrorType",
  "message": "Detailed error message"
}
```

**Error Types:**
- `JSONParseError` - Invalid input JSON format
- `CompilationError` - DSL syntax or semantic errors
- `SerializationError` - Failed to serialize output to JSON

## Examples

### Basic Region Definition

```javascript
const input = JSON.stringify([{
  pos: [0, 64, 0],
  text: "@building=rc([0,0,0],[10,5,10])\n#building:type=\"house\"\n#building:owner=\"player1\""
}]);

const result = compile_json(input);
console.log(result);
// Output: {"building": {"bounding_boxes": [[[0,64,0],[10,69,10]]], "metadata": {"type": "house", "owner": "player1"}}}
```

### Multiple Regions with Global Metadata

```javascript
const input = JSON.stringify([
  {
    pos: [0, 64, 0],
    text: "@spawn=ac([0,64,0],[5,70,5])\n#spawn:safe_zone=true"
  },
  {
    pos: [100, 64, 100],
    text: "@shop=rc([0,0,0],[8,4,8])\n#shop:npc=\"trader\"\n#$global:world_name=\"MyWorld\""
  }
]);

const result = compile_json(input);
const output = JSON.parse(result);
console.log(output.$global.metadata.world_name); // "MyWorld"
console.log(output.spawn.metadata.safe_zone);    // true
console.log(output.shop.metadata.npc);           // "trader"
```

### Boolean Operations (Union)

```javascript
const input = JSON.stringify([{
  pos: [0, 64, 0],
  text: "@room.main=rc([0,0,0],[10,3,10])\n@room.annex=rc([10,0,5],[15,3,10])\n@room.combined=room.main+room.annex\n#room.combined:purpose=\"living space\""
}]);

const result = compile_json(input);
// The combined region will include both the main room and annex areas
```

### Error Handling

```javascript
// Invalid JSON input
let result = compile_json("invalid json");
let error = JSON.parse(result);
console.log(error.code);    // "JSONParseError"
console.log(error.message); // Details about the JSON parsing error

// Invalid DSL syntax
const badDSL = JSON.stringify([{
  pos: [0, 0, 0],
  text: "@invalid syntax here"
}]);
result = compile_json(badDSL);
error = JSON.parse(result);
console.log(error.code);    // "CompilationError"
console.log(error.message); // Details about the DSL error
```

## 🔧 Framework Integration

### React

```jsx
import React, { useState, useEffect } from 'react';
import { compile_json } from 'insign-wasm';

function SchematicAnnotator() {
  const [regions, setRegions] = useState(null);
  const [signText, setSignText] = useState(`
    @house=rc([0,0,0],[10,8,12])
    #house:style="cottage"
    #house:rooms=3
  `);

  const compileRegions = () => {
    try {
      const input = [{ pos: [0, 64, 0], text: signText }];
      const result = compile_json(JSON.stringify(input));
      const output = JSON.parse(result);
      
      if (output.status === 'error') {
        console.error('Compilation error:', output.message);
      } else {
        setRegions(output);
      }
    } catch (error) {
      console.error('Failed to compile:', error);
    }
  };

  return (
    <div>
      <textarea 
        value={signText} 
        onChange={(e) => setSignText(e.target.value)}
        rows={5}
      />
      <button onClick={compileRegions}>Compile Regions</button>
      {regions && (
        <pre>{JSON.stringify(regions, null, 2)}</pre>
      )}
    </div>
  );
}
```

### Vue.js

```vue
<template>
  <div>
    <textarea v-model="signText" rows="5" />
    <button @click="compileRegions">Compile Regions</button>
    <pre v-if="regions">{{ JSON.stringify(regions, null, 2) }}</pre>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import { compile_json } from 'insign-wasm'

const signText = ref(`
  @tower=rc([0,0,0],[5,25,5])
  #tower:height=25
  #tower:material="stone"
`)
const regions = ref(null)

function compileRegions() {
  try {
    const input = [{ pos: [0, 64, 0], text: signText.value }]
    const result = compile_json(JSON.stringify(input))
    const output = JSON.parse(result)
    
    if (output.status !== 'error') {
      regions.value = output
    }
  } catch (error) {
    console.error('Compilation failed:', error)
  }
}
</script>
```

### Express.js API

```javascript
const express = require('express');
const { compile_json } = require('insign-wasm');

const app = express();
app.use(express.json());

app.post('/api/compile-schematic', (req, res) => {
  try {
    const { signs } = req.body;
    
    // Validate input
    if (!Array.isArray(signs)) {
      return res.status(400).json({ error: 'Signs must be an array' });
    }
    
    // Compile regions
    const result = compile_json(JSON.stringify(signs));
    const output = JSON.parse(result);
    
    if (output.status === 'error') {
      return res.status(400).json({ error: output.message });
    }
    
    res.json({ regions: output });
    
  } catch (error) {
    res.status(500).json({ error: 'Internal compilation error' });
  }
});

app.listen(3000);
```

### Webpack Configuration

```javascript
// webpack.config.js
module.exports = {
  // ... other config
  experiments: {
    asyncWebAssembly: true,
  },
  module: {
    rules: [
      {
        test: /\.wasm$/,
        type: 'webassembly/async',
      },
    ],
  },
};
```

## Building from Source

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
- [Node.js](https://nodejs.org/) (for testing)

### Quick Build (All Targets)

```bash
# Clone the repository
git clone https://github.com/Schem-at/Insign.git
cd Insign

# Build all WASM targets at once
./build-wasm.sh

# Test all builds
./test-wasm-builds.js
```

### Manual Build Commands

```bash
cd crates/insign-wasm

# Build for Node.js
wasm-pack build --release --target nodejs --out-dir pkg-nodejs --out-name insign

# Build for bundlers (webpack, etc.)
wasm-pack build --release --target bundler --out-dir pkg-bundler --out-name insign

# Build for web (no bundler)
wasm-pack build --release --target web --out-dir pkg-web --out-name insign

# Build for no-modules (script tag)
wasm-pack build --release --target no-modules --out-dir pkg-no-modules --out-name insign
```

### Clean Rebuild

```bash
# Clean and rebuild all targets
CLEAN=true ./build-wasm.sh
```

### Testing

```bash
# Run Rust tests
cargo test

# Build and test all WASM targets
./build-wasm.sh && ./test-wasm-builds.js

# Run WASM-specific tests (requires wasm-pack-test)
wasm-pack test --node
```

## Performance Notes

- The WASM module is designed to handle moderate-sized inputs efficiently
- For very large datasets (hundreds of regions), consider processing in batches
- The compiled WASM binary is approximately ~100KB (may vary with optimization)
- Cold start initialization takes ~1-5ms in most JavaScript environments

## 🔧 Troubleshooting

### Common Issues

#### WASM Module Not Loading

```javascript
// Ensure proper initialization in browsers
import init from 'insign-wasm';

// Always call init() first
async function setup() {
  await init();
  // Now safe to use compile_json
}
```

#### Bundler Issues

```bash
# For Vite users
npm install insign-wasm-bundler

# Add to vite.config.js
export default {
  optimizeDeps: {
    exclude: ['insign-wasm-bundler']
  }
}
```

#### Large Input Performance

```javascript
// Process large datasets in chunks
function processLargeSchematic(signs) {
  const chunkSize = 100;
  const results = [];
  
  for (let i = 0; i < signs.length; i += chunkSize) {
    const chunk = signs.slice(i, i + chunkSize);
    const result = compile_json(JSON.stringify(chunk));
    results.push(JSON.parse(result));
  }
  
  return mergeResults(results);
}
```

### Debug Mode

```javascript
// Enable verbose error messages
process.env.RUST_LOG = 'debug';
const { compile_json } = require('insign-wasm');
```

## 🌍 Browser Compatibility

| Browser | Version | Support Level |
|---------|---------|---------------|
| **Chrome** | 57+ | ✅ Full support |
| **Firefox** | 52+ | ✅ Full support |
| **Safari** | 11+ | ✅ Full support |
| **Edge** | 16+ | ✅ Full support |
| **Node.js** | 12+ | ✅ Recommended |
| **Node.js** | 8-11 | ⚠️ Limited (use `--experimental-modules`) |

## 🔗 Related Projects

- **[insign-core](../insign-core/README.md)** - Core Rust library with full DSL implementation
- **[insign-ffi](../insign-ffi/README.md)** - FFI bindings for JVM/Kotlin integration  
- **[Insign DSL](../../README.md)** - Complete language specification and examples

## 🤝 Contributing

Contributions are welcome! Please see our [Contributing Guide](../../CONTRIBUTING.md) for details.

### Development Setup

```bash
git clone https://github.com/Schem-at/Insign.git
cd Insign
cargo build --all
./build-wasm.sh
./test-wasm-builds.js
```

## 📄 License

MIT License - see [LICENSE](../../LICENSE) for details.

---

<div align="center">
  <p>
    <strong>Built with ❤️ for the Minecraft community</strong>
  </p>
  <p>
    <a href="https://github.com/Schem-at/Insign">🏠 Home</a> •
    <a href="https://github.com/Schem-at/Insign/issues">🐛 Issues</a> •
    <a href="https://github.com/Schem-at/Insign/discussions">💬 Discussions</a> •
    <a href="https://www.npmjs.com/package/insign-wasm">📦 npm</a>
  </p>
</div>
