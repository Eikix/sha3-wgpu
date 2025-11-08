# Bun.js Examples

This directory contains examples demonstrating how to use the GPU-accelerated SHA-3 library in Bun.js.

## Prerequisites

1. Build the WASM module:
   ```bash
   npm run build
   ```

2. Install Bun.js (required for WASM usage)

## Examples

### 1. Basic Usage (`basic.mjs`)

Demonstrates single and batch hashing:

```bash
bun examples/node/basic.mjs
```

Shows:
- Creating a GPU hasher
- Hashing a single input
- Batch hashing multiple inputs

### 2. Batch Performance (`batch-performance.mjs`)

Compares GPU vs CPU performance across different batch sizes:

```bash
bun examples/node/batch-performance.mjs
```

Shows:
- Performance comparison GPU vs CPU (Bun.js crypto)
- Speedup measurements
- Throughput calculations

**Note:** GPU performance improves significantly with larger batch sizes (100+ inputs).

### 3. All Variants (`all-variants.mjs`)

Demonstrates all SHA-3 variants:

```bash
bun examples/node/all-variants.mjs
```

Shows:
- SHA3-224, SHA3-256, SHA3-384, SHA3-512
- SHAKE128 and SHAKE256 with variable output lengths

## API Reference

### Creating a Hasher

```javascript
import init, { Sha3WasmHasher } from '../../pkg/sha3_wasm.js';

// Initialize WASM module (required for web target)
await init();

const hasher = await new Sha3WasmHasher('sha3-256');
```

Supported variants: `'sha3-224'`, `'sha3-256'`, `'sha3-384'`, `'sha3-512'`, `'shake128'`, `'shake256'`

### Single Hash

```javascript
const input = new TextEncoder().encode('hello world');
const hash = await hasher.hashSingle(input);
console.log(Buffer.from(hash).toString('hex'));
```

### Batch Hashing

```javascript
const inputs = [
  new TextEncoder().encode('message 1'),
  new TextEncoder().encode('message 2'),
  new TextEncoder().encode('message 3')
];

const hashes = await hasher.hashBatch(inputs);
hashes.forEach((hash, i) => {
  console.log(`Hash ${i}: ${Buffer.from(hash).toString('hex')}`);
});
```

### Variable Output Length (SHAKE only)

```javascript
const hasher = await new Sha3WasmHasher('shake256');
const inputs = [new TextEncoder().encode('hello')];
const hashes = await hasher.hashBatchWithLength(inputs, 64); // 64 bytes output
```

## Performance Tips

1. **Batch Size**: GPU performance improves with larger batches (100-1000+ inputs)
2. **Input Length**: Keep inputs the same length for optimal GPU utilization
3. **Reuse Hasher**: Create the hasher once and reuse it for multiple batches
4. **Warmup**: First GPU call may be slower due to initialization

## Troubleshooting

- **GPU not found**: Ensure WebGPU is available (requires recent Chrome/Edge or native GPU support)
- **WASM errors**: Rebuild the WASM module with `npm run build`
- **Performance issues**: Try larger batch sizes (GPU excels at parallel processing)
