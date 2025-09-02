#!/usr/bin/env node

/**
 * test-wasm-builds.js - Test all WASM builds
 * 
 * This script tests all WASM build targets to ensure they work correctly.
 * Run after building with ./build-wasm.sh
 */

const fs = require('fs');
const path = require('path');

// Colors for console output
const colors = {
    reset: '\x1b[0m',
    red: '\x1b[31m',
    green: '\x1b[32m',
    yellow: '\x1b[33m',
    blue: '\x1b[34m'
};

// Test data
const testInput = JSON.stringify([
    {
        pos: [0, 64, 0],
        text: "@building=rc([0,0,0],[10,5,10])\n#building:type=\"house\"\n#building:owner=\"player1\""
    },
    {
        pos: [100, 64, 100],
        text: "@spawn=ac([0,64,0],[5,70,5])\n#spawn:safe_zone=true\n#$global:world_name=\"TestWorld\""
    }
]);

const targets = ['nodejs', 'web', 'bundler'];

async function testTarget(target) {
    const pkgDir = path.join(__dirname, 'crates', 'insign-wasm', `pkg-${target}`);
    
    if (!fs.existsSync(pkgDir)) {
        console.log(`${colors.red}❌ ${target}: Package directory not found: ${pkgDir}${colors.reset}`);
        return false;
    }
    
    const packageJsonPath = path.join(pkgDir, 'package.json');
    const jsPath = path.join(pkgDir, 'insign.js');
    const wasmPath = path.join(pkgDir, 'insign_bg.wasm');
    const dtsPath = path.join(pkgDir, 'insign.d.ts');
    
    // Check all required files exist
    const requiredFiles = [
        { path: packageJsonPath, name: 'package.json' },
        { path: jsPath, name: 'insign.js' },
        { path: wasmPath, name: 'insign_bg.wasm' },
        { path: dtsPath, name: 'insign.d.ts' }
    ];
    
    for (const file of requiredFiles) {
        if (!fs.existsSync(file.path)) {
            console.log(`${colors.red}❌ ${target}: Missing ${file.name}${colors.reset}`);
            return false;
        }
    }
    
    // Check package.json contents
    try {
        const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
        console.log(`${colors.blue}📋 ${target}: ${packageJson.name}@${packageJson.version}${colors.reset}`);
        
        if (!packageJson.name || !packageJson.version) {
            console.log(`${colors.red}❌ ${target}: Invalid package.json${colors.reset}`);
            return false;
        }
    } catch (error) {
        console.log(`${colors.red}❌ ${target}: Failed to parse package.json: ${error.message}${colors.reset}`);
        return false;
    }
    
    // Test loading and functionality (Node.js target only)
    if (target === 'nodejs') {
        try {
            // Clear require cache to ensure fresh load
            delete require.cache[require.resolve(jsPath)];
            
            const { abi_version, compile_json } = require(jsPath);
            
            // Test ABI version
            const version = abi_version();
            if (typeof version !== 'number' || version < 1) {
                console.log(`${colors.red}❌ ${target}: Invalid ABI version: ${version}${colors.reset}`);
                return false;
            }
            
            // Test compilation
            const result = compile_json(testInput);
            const output = JSON.parse(result);
            
            // Verify output structure
            if (!output.building || !output.spawn || !output.$global) {
                console.log(`${colors.red}❌ ${target}: Invalid compilation output${colors.reset}`);
                console.log('Output:', JSON.stringify(output, null, 2));
                return false;
            }
            
            // Verify specific values
            if (output.building.metadata.type !== 'house') {
                console.log(`${colors.red}❌ ${target}: Incorrect building type${colors.reset}`);
                return false;
            }
            
            if (output.$global.metadata.world_name !== 'TestWorld') {
                console.log(`${colors.red}❌ ${target}: Incorrect global metadata${colors.reset}`);
                return false;
            }
            
            console.log(`${colors.green}✅ ${target}: Functional test passed (ABI v${version})${colors.reset}`);
            
        } catch (error) {
            console.log(`${colors.red}❌ ${target}: Functional test failed: ${error.message}${colors.reset}`);
            return false;
        }
    } else {
        // For non-Node.js targets, check structure based on universal wrapper presence
        try {
            const jsContent = fs.readFileSync(jsPath, 'utf8');
            const originalJsPath = path.join(pkgDir, 'insign-original.js');
            
            // Check if this is a universal wrapper build
            if (fs.existsSync(originalJsPath)) {
                console.log(`${colors.blue}🔄 ${target}: Universal wrapper detected${colors.reset}`);
                
                // For universal wrapper, check the original file or bg file for bundler
                const originalJsContent = fs.readFileSync(originalJsPath, 'utf8');
                let hasExports = originalJsContent.includes('abi_version') || originalJsContent.includes('compile_json');
                
                // For bundler target, exports are in insign_bg.js
                if (!hasExports && target === 'bundler') {
                    const bgJsPath = path.join(pkgDir, 'insign_bg.js');
                    if (fs.existsSync(bgJsPath)) {
                        const bgJsContent = fs.readFileSync(bgJsPath, 'utf8');
                        hasExports = bgJsContent.includes('abi_version') || bgJsContent.includes('compile_json');
                    }
                }
                
                if (!hasExports) {
                    console.log(`${colors.red}❌ ${target}: No exports found in original or background JavaScript files${colors.reset}`);
                    return false;
                }
                
                // Verify wrapper has init function
                if (!jsContent.includes('export default') && !jsContent.includes('function init')) {
                    console.log(`${colors.red}❌ ${target}: Wrapper missing init function${colors.reset}`);
                    return false;
                }
                
            } else {
                // For bundler target, the main exports are in a different file
                if (target === 'bundler') {
                    const bgJsPath = path.join(pkgDir, 'insign_bg.js');
                    if (fs.existsSync(bgJsPath)) {
                        const bgJsContent = fs.readFileSync(bgJsPath, 'utf8');
                        if (!bgJsContent.includes('abi_version') || !bgJsContent.includes('compile_json')) {
                            console.log(`${colors.red}❌ ${target}: Background JavaScript file missing expected exports${colors.reset}`);
                            return false;
                        }
                    } else {
                        console.log(`${colors.red}❌ ${target}: Missing insign_bg.js file${colors.reset}`);
                        return false;
                    }
                } else {
                    // Direct build without wrapper
                    if (!jsContent.includes('abi_version') || !jsContent.includes('compile_json')) {
                        console.log(`${colors.red}❌ ${target}: JavaScript file missing expected exports${colors.reset}`);
                        return false;
                    }
                }
            }
            
            console.log(`${colors.green}✅ ${target}: Structure check passed${colors.reset}`);
        } catch (error) {
            console.log(`${colors.red}❌ ${target}: Failed to read JavaScript file: ${error.message}${colors.reset}`);
            return false;
        }
    }
    
    // Check TypeScript definitions
    try {
        const dtsContent = fs.readFileSync(dtsPath, 'utf8');
        if (!dtsContent.includes('export function abi_version') || 
            !dtsContent.includes('export function compile_json')) {
            console.log(`${colors.yellow}⚠️  ${target}: TypeScript definitions incomplete${colors.reset}`);
        }
    } catch (error) {
        console.log(`${colors.yellow}⚠️  ${target}: Failed to read TypeScript definitions: ${error.message}${colors.reset}`);
    }
    
    return true;
}

async function main() {
    console.log(`${colors.blue}🧪 Testing WASM builds...${colors.reset}`);
    console.log('');
    
    let allPassed = true;
    
    for (const target of targets) {
        const passed = await testTarget(target);
        if (!passed) {
            allPassed = false;
        }
        console.log('');
    }
    
    if (allPassed) {
        console.log(`${colors.green}🎉 All WASM builds passed tests!${colors.reset}`);
        process.exit(0);
    } else {
        console.log(`${colors.red}💥 Some WASM builds failed tests!${colors.reset}`);
        console.log(`${colors.yellow}💡 Try rebuilding with: ./build-wasm.sh${colors.reset}`);
        process.exit(1);
    }
}

main().catch(error => {
    console.error(`${colors.red}💥 Test script failed: ${error.message}${colors.reset}`);
    process.exit(1);
});
