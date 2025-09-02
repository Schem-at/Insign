#!/usr/bin/env node

/**
 * insign-wasm Example
 * 
 * This example demonstrates how to use insign-wasm to compile
 * Minecraft schematic region annotations from sign text.
 * 
 * Run with: node example.js
 */

const { abi_version, compile_json } = require('./pkg/insign.js');

console.log('🏰 Insign WASM Example');
console.log('=====================');
console.log(`📋 WASM ABI Version: ${abi_version()}`);
console.log('');

// Example 1: Simple building with metadata
console.log('📝 Example 1: Simple Building');
console.log('------------------------------');

const building = [{
  pos: [100, 64, 200],
  text: `
    @house=rc([0,0,0],[15,8,12])
    #house:builder="Steve"
    #house:style="cottage"
    #house:rooms=4
    #house:completed=true
  `
}];

try {
  const result1 = compile_json(JSON.stringify(building));
  const output1 = JSON.parse(result1);
  
  if (output1.status === 'error') {
    console.error('❌ Error:', output1.message);
  } else {
    console.log('✅ Compiled successfully:');
    console.log(JSON.stringify(output1, null, 2));
  }
} catch (error) {
  console.error('💥 Exception:', error.message);
}

console.log('\n');

// Example 2: Multiple regions with global metadata
console.log('🏘️  Example 2: Village Layout');
console.log('------------------------------');

const village = [
  {
    pos: [0, 64, 0],
    text: `
      @spawn=ac([0,64,0],[3,67,3])
      #spawn:safe_zone=true
      #spawn:respawn_point=true
    `
  },
  {
    pos: [50, 64, 0],
    text: `
      @blacksmith=rc([0,0,0],[8,6,8])
      #blacksmith:profession="armorer"
      #blacksmith:tier=2
    `
  },
  {
    pos: [0, 64, 50],
    text: `
      @market=rc([0,0,0],[12,4,16])
      #market:stalls=6
      #market:open_hours="9-18"
      #$global:village_name="Oakwood"
      #$global:population=45
    `
  }
];

try {
  const result2 = compile_json(JSON.stringify(village));
  const output2 = JSON.parse(result2);
  
  if (output2.status === 'error') {
    console.error('❌ Error:', output2.message);
  } else {
    console.log('✅ Compiled village layout:');
    console.log('');
    console.log('🌐 Global metadata:');
    console.log(JSON.stringify(output2.$global, null, 2));
    console.log('');
    console.log('🏠 Buildings:');
    Object.entries(output2).forEach(([name, data]) => {
      if (name !== '$global') {
        console.log(`  ${name}:`, JSON.stringify(data.metadata, null, 4));
      }
    });
  }
} catch (error) {
  console.error('💥 Exception:', error.message);
}

console.log('\n');

// Example 3: Boolean operations (union)
console.log('🔧 Example 3: Boolean Operations');
console.log('---------------------------------');

const compound = [{
  pos: [0, 64, 0],
  text: `
    @main_hall=rc([0,0,0],[20,8,30])
    @side_wing=rc([20,0,10],[35,8,25])
    @tower=rc([35,0,0],[40,25,5])
    @complex=main_hall+side_wing+tower
    #complex:purpose="royal palace"
    #complex:construction_year=1247
  `
}];

try {
  const result3 = compile_json(JSON.stringify(compound));
  const output3 = JSON.parse(result3);
  
  if (output3.status === 'error') {
    console.error('❌ Error:', output3.message);
  } else {
    console.log('✅ Complex structure compiled:');
    console.log('');
    console.log('🏰 Complex bounding boxes:');
    if (output3.complex && output3.complex.bounding_boxes) {
      output3.complex.bounding_boxes.forEach((box, i) => {
        console.log(`  Box ${i + 1}: [${box[0].join(',')}] to [${box[1].join(',')}]`);
      });
      console.log('📋 Metadata:', JSON.stringify(output3.complex.metadata, null, 2));
    }
  }
} catch (error) {
  console.error('💥 Exception:', error.message);
}

console.log('\n');

// Example 4: Error handling
console.log('⚠️  Example 4: Error Handling');
console.log('------------------------------');

const invalidInput = [{
  pos: [0, 0, 0],
  text: '@invalid_syntax_here_missing_equals'
}];

try {
  const result4 = compile_json(JSON.stringify(invalidInput));
  const output4 = JSON.parse(result4);
  
  if (output4.status === 'error') {
    console.log('✅ Error properly caught:');
    console.log(`   Code: ${output4.code}`);
    console.log(`   Message: ${output4.message}`);
  } else {
    console.log('❌ Expected an error but got success');
  }
} catch (error) {
  console.error('💥 Exception:', error.message);
}

console.log('\n');

// Example 5: Performance test
console.log('⚡ Example 5: Performance Test');
console.log('------------------------------');

const largeDataset = Array.from({ length: 100 }, (_, i) => ({
  pos: [i * 10, 64, i * 10],
  text: `
    @building_${i}=rc([0,0,0],[5,3,5])
    #building_${i}:id=${i}
    #building_${i}:type="auto_generated"
  `
}));

console.log(`📊 Testing with ${largeDataset.length} regions...`);

const startTime = Date.now();
try {
  const resultLarge = compile_json(JSON.stringify(largeDataset));
  const outputLarge = JSON.parse(resultLarge);
  const endTime = Date.now();
  
  if (outputLarge.status === 'error') {
    console.error('❌ Error:', outputLarge.message);
  } else {
    const regionCount = Object.keys(outputLarge).length;
    console.log(`✅ Successfully compiled ${regionCount} regions`);
    console.log(`⏱️  Time taken: ${endTime - startTime}ms`);
    console.log(`📈 Performance: ~${Math.round(largeDataset.length / (endTime - startTime) * 1000)} regions/second`);
  }
} catch (error) {
  console.error('💥 Exception:', error.message);
}

console.log('\n');
console.log('🎉 All examples completed!');
console.log('');
console.log('💡 Try modifying the examples above to experiment with different');
console.log('   region definitions, metadata, and boolean operations.');
console.log('');
console.log('📚 For more information, see:');
console.log('   • README.md - Full API documentation');
console.log('   • ../../README.md - Complete DSL specification');
console.log('   • https://github.com/Schem-at/Insign - Project homepage');
