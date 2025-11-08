// Demonstrating all SHA-3 variants
// Run with: node examples/node/all-variants.mjs

import { Sha3WasmHasher } from '../../pkg/sha3_wasm.js';

async function testVariant(variantName) {
    console.log(`\n=== ${variantName.toUpperCase()} ===`);

    const hasher = await new Sha3WasmHasher(variantName);
    const input = new TextEncoder().encode('The quick brown fox jumps over the lazy dog');
    const hash = await hasher.hashSingle(input);

    console.log(`Input:  "The quick brown fox jumps over the lazy dog"`);
    console.log(`Output: ${Buffer.from(hash).toString('hex')}`);
    console.log(`Length: ${hash.length} bytes (${hash.length * 8} bits)`);
}

async function testShakeVariant(variantName, outputLengths) {
    console.log(`\n=== ${variantName.toUpperCase()} (Variable Output) ===`);

    const hasher = await new Sha3WasmHasher(variantName);
    const input = new TextEncoder().encode('Hello SHAKE!');

    console.log(`Input: "Hello SHAKE!"`);

    for (const length of outputLengths) {
        const hashes = await hasher.hashBatchWithLength(
            [new TextEncoder().encode('Hello SHAKE!')],
            length
        );
        const hash = hashes[0];
        console.log(`${length} bytes: ${Buffer.from(hash).toString('hex')}`);
    }
}

async function main() {
    console.log('=== SHA-3 Family Demonstration ===');
    console.log('Testing all SHA-3 variants with GPU acceleration\n');

    // Standard SHA-3 variants (fixed output length)
    await testVariant('sha3-224');
    await testVariant('sha3-256');
    await testVariant('sha3-384');
    await testVariant('sha3-512');

    // SHAKE variants (extensible output functions)
    await testShakeVariant('shake128', [16, 32, 64]);
    await testShakeVariant('shake256', [32, 64, 128]);

    console.log('\n=== All Variants Tested Successfully ===');
}

main().catch(console.error);
