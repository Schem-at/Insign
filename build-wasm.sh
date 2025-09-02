#!/bin/bash

# build-wasm.sh - Production-ready build script for insign-wasm
# Supports bundlers, Node.js, CDN usage with universal initialization
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
CRATE_DIR="crates/insign-wasm"
CRATE_NAME="insign"
OUT_NAME="insign"
CDN_LOADER_FILENAME="insign-cdn-loader.js"
TARGETS=("nodejs" "web" "bundler" "no-modules")
CLEAN=${CLEAN:-false}
UNIVERSAL=${UNIVERSAL:-true}

echo -e "${BLUE}🚀 Building insign-wasm for all targets...${NC}"

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo -e "${RED}❌ wasm-pack is not installed!${NC}"
    echo "Install it with:"
    echo "  curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh"
    echo "  or visit: https://rustwasm.github.io/wasm-pack/installer/"
    exit 1
fi

# Check if we're in the right directory
if [[ ! -d "$CRATE_DIR" ]]; then
    echo -e "${RED}❌ Directory $CRATE_DIR not found!${NC}"
    echo "Make sure you're running this script from the project root."
    exit 1
fi

cd "$CRATE_DIR"

# Clean previous builds if requested
if [[ "$CLEAN" == "true" ]]; then
    echo -e "${YELLOW}🧹 Cleaning previous builds...${NC}"
    rm -rf pkg*
fi

# Function to create universal init wrapper
create_universal_wrapper() {
    local output_dir="$1"
    local target="$2"
    
    if [[ "$UNIVERSAL" != "true" ]] || [[ "$target" == "no-modules" ]]; then
        return 0
    fi
    
    echo -e "${BLUE}🔄 Creating universal init wrapper for $target...${NC}"
    
    # Backup original file
    mv "$output_dir/${OUT_NAME}.js" "$output_dir/${OUT_NAME}-original.js"
    
    # Create universal wrapper based on target
    if [[ "$target" == "web" ]]; then
        # Web target with CDN support
        cat << 'EOF' > "$output_dir/${OUT_NAME}.js"
// Universal WASM initializer for web environments
import init_wasm from './insign-original.js';

export default async function init(input) {
  // If input is provided, use it directly (manual override)
  if (input !== undefined) {
    return await init_wasm(input);
  }

  // Auto-detect environment and load appropriately
  try {
    // Try to load from same directory (default behavior)
    return await init_wasm();
  } catch (error) {
    console.warn('Default WASM loading failed, trying relative path:', error.message);
    // Fallback: try explicit WASM URL
    const wasmUrl = new URL('./insign_bg.wasm', import.meta.url);
    return await init_wasm(wasmUrl);
  }
}

// Re-export everything from the original module
export * from './insign-original.js';
EOF
    elif [[ "$target" == "nodejs" ]]; then
        # Node.js target with file system support
        cat << 'EOF' > "$output_dir/${OUT_NAME}.js"
// Universal WASM initializer for Node.js environments
const init_wasm = require('./insign-original.js');

async function init(input) {
  // If input is provided, use it directly (manual override)
  if (input !== undefined) {
    if (typeof init_wasm.default === 'function') {
      return await init_wasm.default(input);
    }
    return await init_wasm(input);
  }

  // Node.js: read the WASM file directly
  try {
    const fs = require('fs');
    const path = require('path');
    
    const wasmPath = path.join(__dirname, 'insign_bg.wasm');
    const wasmBytes = fs.readFileSync(wasmPath);

    if (typeof init_wasm.default === 'function') {
      return await init_wasm.default(wasmBytes);
    }
    return await init_wasm(wasmBytes);
  } catch (error) {
    console.warn('Failed to load WASM in Node.js, trying default init:', error.message);
    if (typeof init_wasm.default === 'function') {
      return await init_wasm.default();
    }
    return await init_wasm();
  }
}

// Export the init function
module.exports = init;
module.exports.default = init;

// Re-export everything from the original module
Object.assign(module.exports, init_wasm);
EOF
    elif [[ "$target" == "bundler" ]]; then
        # Bundler target 
        cat << 'EOF' > "$output_dir/${OUT_NAME}.js"
// Universal WASM initializer for bundlers
import init_wasm from './insign-original.js';

export default async function init(input) {
  // If input is provided, use it directly (manual override)
  if (input !== undefined) {
    return await init_wasm(input);
  }

  // Let bundler handle WASM loading
  return await init_wasm();
}

// Re-export everything from the original module
export * from './insign-original.js';
EOF
    fi
}

# Function to create CDN loader
create_cdn_loader() {
    local output_dir="$1"
    
    echo -e "${BLUE}🌐 Creating CDN loader...${NC}"
    
    cat << EOF > "$output_dir/$CDN_LOADER_FILENAME"
// CDN loader for insign-wasm
// Use this for loading from CDN in browsers via <script type="module">

// Import the real init function and all exports from the original module
import init, * as wasm from './${OUT_NAME}-original.js';

// The default export is an initializer function for CDN use
// It calls the real 'init' but provides the URL to the .wasm file
export default async function() {
  const wasmUrl = new URL('./${OUT_NAME}_bg.wasm', import.meta.url);
  await init(wasmUrl);
}

// Re-export all the named exports (abi_version, compile_json, etc.)
export * from './${OUT_NAME}-original.js';
EOF
}

# Function to configure package.json
configure_package_json() {
    local output_dir="$1"
    local target="$2"
    
    echo -e "${BLUE}📋 Configuring package.json for $target...${NC}"
    
    node -e "
        const fs = require('fs');
        const path = require('path');
        const pkgPath = '$output_dir/package.json';
        const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));
        
        // Define files to include in npm package
        const baseFiles = [
            '${OUT_NAME}.js',
            '${OUT_NAME}.d.ts', 
            '${OUT_NAME}_bg.wasm',
            '${OUT_NAME}_bg.wasm.d.ts',
            'README.md'
        ];
        
        if ('$UNIVERSAL' === 'true' && '$target' !== 'no-modules') {
            baseFiles.push('${OUT_NAME}-original.js');
        }
        
        if ('$target' === 'web') {
            baseFiles.push('${CDN_LOADER_FILENAME}');
        }
        
        pkg.files = [...new Set(baseFiles)];
        
        // Configure entry points based on target
        if ('$target' === 'nodejs') {
            pkg.main = './${OUT_NAME}.js';
            pkg.types = './${OUT_NAME}.d.ts';
            pkg.engines = { node: '>=12' };
        } else if ('$target' === 'web') {
            pkg.module = './${OUT_NAME}.js';
            pkg.main = './${OUT_NAME}.js';
            pkg.types = './${OUT_NAME}.d.ts';
            pkg.browser = './${OUT_NAME}.js';
            
            // Add exports field for modern resolution
            pkg.exports = {
                '.': {
                    'import': './${OUT_NAME}.js',
                    'types': './${OUT_NAME}.d.ts'
                },
                './cdn-loader': {
                    'import': './${CDN_LOADER_FILENAME}'
                },
                './package.json': './package.json'
            };
        } else if ('$target' === 'bundler') {
            pkg.module = './${OUT_NAME}.js';
            pkg.main = './${OUT_NAME}.js';
            pkg.types = './${OUT_NAME}.d.ts';
        }
        
        // Synchronize version from workspace Cargo.toml if available
        const workspaceCargoPath = '../../Cargo.toml';
        if (fs.existsSync(workspaceCargoPath)) {
            try {
                const cargoContent = fs.readFileSync(workspaceCargoPath, 'utf8');
                const versionMatch = cargoContent.match(/^version\s*=\s*\"([^\"]+)\"/m);
                if (versionMatch && versionMatch[1]) {
                    pkg.version = versionMatch[1];
                }
            } catch (e) {
                console.warn('Could not sync version from Cargo.toml:', e.message);
            }
        }
        
        fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2));
    "
}

# Build for each target
for target in "${TARGETS[@]}"; do
    echo -e "${BLUE}📦 Building for target: ${target}${NC}"
    
    output_dir="pkg-${target}"
    
    # Map target names to wasm-pack target names
    case "$target" in
        "nodejs")
            wasm_target="nodejs"
            ;;
        "web")
            wasm_target="web"
            ;;
        "bundler")
            wasm_target="bundler"
            ;;
        "no-modules")
            wasm_target="no-modules"
            ;;
        *)
            echo -e "${RED}❌ Unknown target: $target${NC}"
            exit 1
            ;;
    esac
    
    # Build the WASM package
    if wasm-pack build --release --target "$wasm_target" --out-dir "$output_dir" --out-name "$OUT_NAME"; then
        echo -e "${GREEN}✅ Successfully built $target target${NC}"
        
        # Create universal wrapper if enabled
        create_universal_wrapper "$output_dir" "$target"
        
        # Create CDN loader for web target
        if [[ "$target" == "web" ]]; then
            create_cdn_loader "$output_dir"
        fi
        
        # Configure package.json
        configure_package_json "$output_dir" "$target"
        
        # Copy README if it exists
        if [[ ! -f "$output_dir/README.md" ]] && [[ -f "README.md" ]]; then
            cp README.md "$output_dir/README.md"
        fi
        
        # Show package contents
        echo -e "${BLUE}📄 Generated files:${NC}"
        ls -la "$output_dir" | head -10
        
        # Show package.json version info
        if [[ -f "$output_dir/package.json" ]]; then
            version=$(grep '"version"' "$output_dir/package.json" | cut -d'"' -f4)
            name=$(grep '"name"' "$output_dir/package.json" | cut -d'"' -f4)
            echo -e "${BLUE}📋 Package: ${name}@${version}${NC}"
        fi
        
        echo ""
    else
        echo -e "${RED}❌ Failed to build $target target${NC}"
        exit 1
    fi
done

echo -e "${GREEN}🎉 All WASM builds completed successfully!${NC}"

echo ""
echo "===================================================================="
echo -e " ${GREEN}✅ BUILD COMPLETE${NC}"
echo "===================================================================="
echo ""
echo " This package now supports multiple use cases:"
echo ""
echo " 1) BUNDLERS & NODE.JS (Universal - Auto-detects environment):"
echo "    ---------------------------------------------------------"
echo "    import init, { compile_json } from 'insign-wasm';"
echo "    await init(); // Works in both Node.js and browsers automatically"
echo "    const result = compile_json(data);"
echo ""
echo " 2) CDN (in a browser <script type=\"module\">):"
echo "    ----------------------------------------------"
echo "    import init, { compile_json } from 'https://cdn.jsdelivr.net/npm/insign-wasm@latest/insign-cdn-loader.js';"
echo "    await init();"
echo "    const result = compile_json(data);"
echo ""
echo " 3) MANUAL WASM loading (advanced usage):"
echo "    ---------------------------------------"
echo "    import init, { compile_json } from 'insign-wasm';"
echo "    const wasmBytes = /* your WASM bytes */;"
echo "    await init(wasmBytes);"
echo "    const result = compile_json(data);"
echo ""
echo "===================================================================="
echo ""
echo -e "${BLUE}🔧 Development Options:${NC}"
echo "  Clean rebuild: CLEAN=true ./build-wasm.sh"
echo "  Skip universal wrappers: UNIVERSAL=false ./build-wasm.sh"
echo "  Test builds: ./test-wasm-builds.js"
echo ""
echo -e "${BLUE}📚 Package Locations:${NC}"
for target in "${TARGETS[@]}"; do
    echo "  ${target}: ./pkg-${target}/"
done
echo ""
