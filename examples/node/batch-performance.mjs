// Batch performance demonstration for Bun.js
// Shows the performance benefits of GPU batch processing
// Run with: bun examples/node/batch-performance.mjs

import init, { Sha3WasmHasher } from '../../pkg/sha3_wasm.js';
import crypto from 'crypto';

// Helper to measure execution time
async function measureTime(fn, label) {
    const start = performance.now();
    await fn();
    const end = performance.now();
    const duration = end - start;
    console.log(`${label}: ${duration.toFixed(2)}ms`);
    return duration;
}

async function benchmarkCPU(inputs) {
    const hashes = [];
    for (const input of inputs) {
        const hash = crypto.createHash('sha3-256').update(input).digest();
        hashes.push(hash);
    }
    return hashes;
}

async function benchmarkGPU(hasher, inputs) {
    // Convert to Uint8Arrays
    const uint8Inputs = inputs.map(buf => new Uint8Array(buf));
    const hashes = await hasher.hashBatch(uint8Inputs);
    return hashes;
}

async function main() {
    console.log('=== GPU vs CPU Batch Performance Comparison ===\n');

    // Initialize WASM module
    console.log('Initializing WASM module...');
    await init();

    // Initialize GPU hasher
    console.log('Initializing GPU hasher...');
    const hasher = await new Sha3WasmHasher('sha3-256');
    console.log('GPU ready!\n');

    // Test different batch sizes
    const batchSizes = [10, 50, 100, 500, 1000];

    for (const batchSize of batchSizes) {
        console.log(`\n--- Batch Size: ${batchSize} ---`);

        // Generate test data (64 bytes per input)
        const inputs = [];
        for (let i = 0; i < batchSize; i++) {
            const data = Buffer.alloc(64);
            data.write(`test input number ${i}`);
            inputs.push(data);
        }

        // Warmup
        await benchmarkGPU(hasher, inputs);

        // CPU benchmark
        const cpuMs = await measureTime(
            async () => await benchmarkCPU(inputs),
            `CPU (Bun.js crypto)`
        );

        // GPU benchmark
        const gpuMs = await measureTime(
            async () => await benchmarkGPU(hasher, inputs),
            `GPU (WGPU + WASM)   `
        );

        // Calculate speedup
        const speedup = (cpuMs / gpuMs).toFixed(2);
        const throughput = (batchSize / (gpuMs / 1000)).toFixed(0);

        console.log(`Speedup: ${speedup}x`);
        console.log(`GPU Throughput: ${throughput} hashes/sec`);
    }

    console.log('\n=== Benchmark Complete ===');
    console.log('Note: GPU performance improves with larger batch sizes!');
}

main().catch(console.error);
