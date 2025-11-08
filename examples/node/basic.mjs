// Basic SHA-3 GPU example for Node.js/Bun
// Run with: node examples/node/basic.mjs

import { Sha3WasmHasher } from '../../pkg/sha3_wasm.js';

async function main() {
    console.log('=== GPU-Accelerated SHA-3 Basic Example ===\n');

    // Create a hasher for SHA3-256
    console.log('Initializing GPU SHA-3 hasher...');
    const hasher = await new Sha3WasmHasher('sha3-256');
    console.log(`Hasher created: ${hasher.getVariant()}`);
    console.log(`Output size: ${hasher.getOutputSize()} bytes\n`);

    // Single hash example
    console.log('--- Single Hash Example ---');
    const input = new TextEncoder().encode('Hello, GPU-accelerated SHA-3!');
    const hash = await hasher.hashSingle(input);
    console.log(`Input: "Hello, GPU-accelerated SHA-3!"`);
    console.log(`Hash:  ${Buffer.from(hash).toString('hex')}\n`);

    // Batch hashing example
    console.log('--- Batch Hashing Example ---');
    const inputs = [
        new TextEncoder().encode('message 1'),
        new TextEncoder().encode('message 2'),
        new TextEncoder().encode('message 3'),
        new TextEncoder().encode('message 4'),
        new TextEncoder().encode('message 5'),
    ];

    console.log(`Hashing ${inputs.length} messages in one GPU batch...`);
    const hashes = await hasher.hashBatch(inputs);

    hashes.forEach((hash, i) => {
        const inputStr = new TextDecoder().decode(inputs[i]);
        const hashHex = Buffer.from(hash).toString('hex');
        console.log(`${i + 1}. "${inputStr}" => ${hashHex}`);
    });

    console.log('\n=== Example Complete ===');
}

main().catch(console.error);
