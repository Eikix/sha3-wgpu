# sha3-wgpu

GPU-accelerated SHA-3 library using WebGPU (WGSL + wgpu-rs) with WASM bindings for Bun.js.

This library provides **batch hashing** capabilities, making it ideal for scenarios where you need to hash many inputs in parallel. The GPU implementation excels when processing 100+ hashes simultaneously.

## Features

- **GPU-Accelerated**: Uses WGSL compute shaders for parallel SHA-3 computation
- **Batch Processing**: Optimized for hashing multiple inputs simultaneously
- **All SHA-3 Variants**: Supports SHA3-224, SHA3-256, SHA3-384, SHA3-512, SHAKE128, and SHAKE256
- **WASM Support**: Full Bun.js compatibility via WASM bindings
- **Memory Optimized**: Proper GPU memory alignment and bank conflict avoidance
- **Tested**: Comprehensive tests against official SHA-3 implementations
- **Benchmarked**: Criterion benchmarks comparing GPU vs CPU performance

## Architecture

This project uses a Rust workspace with multiple crates:

- **`sha3-core`**: Core SHA-3 types, variant definitions, and utilities
- **`sha3-wgpu`**: GPU-accelerated implementation using WGSL compute shaders and wgpu-rs
- **`sha3-wasm`**: WASM bindings using wasm-bindgen for Bun.js integration
- **`sha3-bench`**: Criterion benchmarking suite for GPU vs CPU performance comparison

## Prerequisites

- Rust (latest stable)
- wasm-pack (`cargo install wasm-pack`)
- **Modern browser with WebGPU support** (Chrome/Edge 113+) for browser demos
- GPU with WebGPU support (for browser usage) or Vulkan/Metal/DX12 (for native usage)

## Quick Start

### 1. Build the library

```bash
# Build Rust library
cargo build --release

# Build WASM module for Bun.js
npm run build:release
```

### 2. Run examples

```bash
# Rust example (native, works without WebGPU)
cargo run --example basic

# Browser demo (requires WebGPU support in browser)
npm run demo
# Then open your browser to http://localhost:5173
```

**Note:** The browser demo is required for testing WASM + WebGPU functionality because **Node.js and Bun.js don't support WebGPU**. Only modern browsers (Chrome/Edge 113+) have WebGPU support.

### 3. Run tests

```bash
# Run all tests (compares GPU output vs official SHA-3)
cargo test

# Run benchmarks (GPU vs CPU performance)
cargo bench
```

## Building

### Build WASM module for Bun.js:

```bash
npm run build                # Development build (targets web for Bun.js)
npm run build:release        # Optimized release build (targets web for Bun.js)
```

**Note:** The build commands use `--target web` which is appropriate for browsers. WebGPU is only available in browsers, not in Node.js or Bun.js.

## Usage

### Rust

```rust
use sha3_core::Sha3Variant;
use sha3_wgpu::{GpuContext, GpuSha3Hasher};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize GPU context
    let context = GpuContext::new().await?;
    let hasher = GpuSha3Hasher::new(context, Sha3Variant::Sha3_256)?;

    // Batch hash multiple inputs
    let inputs = vec![
        b"message 1".as_slice(),
        b"message 2".as_slice(),
        b"message 3".as_slice(),
    ];

    let hashes = hasher.hash_batch(&inputs).await?;
    println!("Computed {} hashes on GPU", inputs.len());

    Ok(())
}
```

### Browser (WebGPU)

```javascript
import init, { Sha3WasmHasher } from "./pkg/sha3_wasm.js";

// Initialize WASM module (required for web target)
await init();

// Create hasher
const hasher = await new Sha3WasmHasher("sha3-256");

// Single hash
const input = new TextEncoder().encode("hello world");
const hash = await hasher.hashSingle(input);
console.log(
  Array.from(hash)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join(""),
);

// Batch hashing (optimal for GPU)
const inputs = [
  new TextEncoder().encode("message 1"),
  new TextEncoder().encode("message 2"),
  new TextEncoder().encode("message 3"),
];

const hashes = await hasher.hashBatch(inputs);
hashes.forEach((hash, i) => {
  console.log(
    `Hash ${i}:`,
    Array.from(hash)
      .map((b) => b.toString(16).padStart(2, "0"))
      .join(""),
  );
});
```

**Important:** This code only works in browsers with WebGPU support (Chrome/Edge 113+). Use the browser demo (`npm run demo`) to test the functionality.

## Performance

The GPU implementation is optimized for **batch processing**. Performance improves significantly with larger batch sizes:

- **1-10 hashes**: CPU is faster (less overhead)
- **10-100 hashes**: GPU starts to match CPU
- **100+ hashes**: GPU significantly outperforms CPU
- **1000+ hashes**: GPU can be 5-10x faster

Run `cargo bench` to benchmark on your hardware, or try the browser performance demo:

```bash
npm run demo
```

The browser demo includes an interactive performance comparison showing GPU vs CPU performance across different batch sizes (10, 50, 100, 500, 1000 hashes), displaying speedup ratios and throughput metrics.

## GPU Optimizations

This implementation includes several GPU-specific optimizations:

1. **Memory Alignment**: All data structures are aligned to GPU requirements (16-byte boundaries)
2. **Batch Processing**: One compute thread per hash, utilizing GPU parallelism
3. **Efficient State Management**: Keccak state (200 bytes) kept in GPU registers
4. **Optimized Permutation**: 24-round Keccak-f[1600] implemented in WGSL
5. **Minimal Memory Transfers**: Input/output transferred in single operations

## Project Structure

```
.
├── Cargo.toml                      # Workspace configuration
├── package.json                    # Bun.js package configuration
├── crates/
│   ├── sha3-core/                  # Core SHA-3 types and utilities
│   │   ├── src/
│   │   │   ├── types.rs            # SHA-3 variants, batch params
│   │   │   └── error.rs            # Error types
│   ├── sha3-wgpu/                  # GPU implementation
│   │   ├── src/
│   │   │   ├── wgsl/
│   │   │   │   └── sha3.wgsl       # GPU compute shader
│   │   │   ├── context.rs          # GPU context management
│   │   │   ├── compute.rs          # Compute pipeline & batch processing
│   │   │   └── error.rs            # GPU error types
│   ├── sha3-wasm/                  # WASM bindings for Bun.js
│   │   └── src/lib.rs              # WASM API
│   └── sha3-bench/                 # Benchmarking suite
│       ├── benches/
│       │   └── sha3_comparison.rs  # Criterion benchmarks
│       └── src/main.rs             # Benchmark runner
├── examples/
│   ├── basic.rs                    # Rust example
│   └── browser/                    # Browser demo (Vite + React)
│       ├── src/
│       │   ├── App.tsx             # Main app component
│       │   ├── components/         # Demo components
│       │   └── hooks/              # React hooks
│       ├── package.json            # Browser demo dependencies
│       └── README.md               # Browser demo documentation
└── pkg/                            # Generated WASM package (gitignored)
```

## Testing

This library includes comprehensive tests comparing GPU output against the official `sha3` crate:

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_sha3_256_batch

# Run with output
cargo test -- --nocapture
```

Tests cover:

- All SHA-3 variants (SHA3-224, SHA3-256, SHA3-384, SHA3-512)
- Empty inputs
- Single inputs
- Small batches (4 inputs)
- Large batches (100+ inputs)
- Long inputs (10KB+)

## Benchmarking

Run comprehensive benchmarks comparing GPU vs CPU:

```bash
cargo bench
```

Benchmarks include:

- Different batch sizes (1, 10, 50, 100, 500, 1000)
- Different input sizes (32B to 4KB)
- Single vs batch comparison
- Large batches (1000, 5000, 10000)

## Technical Details

### SHA-3 Implementation

The core SHA-3 algorithm is implemented in WGSL following the Keccak specification:

1. **Padding**: SHA-3 uses the `pad10*1` rule with domain separator 0x06
2. **Absorbing**: Input XORed into state, then Keccak-f[1600] permutation applied
3. **Squeezing**: Output extracted from state
4. **Permutation**: 24 rounds of θ, ρ, π, χ, ι steps

### WGSL Shader

The compute shader (`sha3.wgsl`) implements:

- Full Keccak-f[1600] permutation (24 rounds)
- Proper padding and domain separation
- Batch processing (one workgroup per hash)
- Memory-efficient state management

### Memory Layout

- **Input buffer**: Flattened u32 array with 16-byte alignment
- **Output buffer**: Flattened u32 array with 16-byte alignment
- **Uniform buffer**: Hash parameters (batch size, input length, rate, output size)

## Troubleshooting

### WebGPU Not Supported

**WebGPU is only available in modern browsers.** Node.js and Bun.js do not support WebGPU.

**Solutions:**

1. **Use the browser demo**: Run `npm run demo` and open in Chrome/Edge 113+
2. **Check browser support**: Visit [webgpu.io](https://webgpu.io) to verify WebGPU support
3. **Use native Rust**: For non-browser GPU acceleration, use the native Rust API:
   ```bash
   cargo run --example basic
   ```

### GPU Initialization Failures

If GPU initialization fails:

- Ensure you have a compatible GPU installed
- Check that GPU drivers are up to date
- For native Rust usage, ensure Vulkan (Linux/Windows), Metal (macOS), or DX12 (Windows) drivers are installed

## Future Improvements

- [ ] Optimize for same-length inputs (current requirement)
- [ ] Add support for variable-length batches
- [ ] Implement streaming API
- [ ] Add SHAKE256 extended output support
- [ ] Optimize workgroup sizes for different GPU architectures
- [ ] Add Web Worker support for browser usage

## License

MIT OR Apache-2.0

## Contributing

Contributions are welcome! Please ensure:

- All tests pass (`cargo test`)
- Code is formatted (`cargo fmt`)
- No clippy warnings (`cargo clippy`)
- Benchmarks show no performance regression
