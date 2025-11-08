# sha3-wgpu

GPU-accelerated SHA-3 library using WebGPU (WGSL + wgpu-rs) with WASM bindings for Node.js and Bun.js.

## Architecture

This project uses a Rust workspace with multiple crates:

- **`sha3-core`**: Core SHA-3 types and utilities
- **`sha3-wgpu`**: GPU-accelerated implementation using WGSL compute shaders and wgpu-rs
- **`sha3-wasm`**: WASM bindings using wasm-bindgen for Node.js/Bun.js integration
- **`sha3-bench`**: Benchmarking suite for performance comparison

## Prerequisites

- Rust (latest stable)
- wasm-pack (`cargo install wasm-pack`)
- Node.js 18+ or Bun.js

## Building

### Build WASM module for Node.js/Bun:

```bash
npm run build
# or
npm run build:release  # for optimized release build
```

**Note:** If you need WASM threading support (for `wasm-bindgen-rayon`), use:
```bash
npm run build:threads  # Requires atomics support
```

This requires enabling atomics and bulk-memory features, which may need additional setup.

### Build for web browsers:

```bash
npm run build:web
```

### Build for bundlers (webpack, vite, etc.):

```bash
npm run build:bundler
```

## Development

### Run tests:

```bash
cargo test
npm run test:wasm  # WASM-specific tests
```

### Run benchmarks:

```bash
cargo bench
```

## Project Structure

```
.
├── Cargo.toml              # Workspace configuration
├── package.json            # Node.js package configuration
├── tsconfig.json           # TypeScript configuration
├── wasm-pack.toml          # wasm-pack configuration
├── .cargo/
│   └── config.toml         # Cargo build configuration
├── crates/
│   ├── sha3-core/          # Core types and utilities
│   ├── sha3-wgpu/          # GPU implementation
│   │   └── src/
│   │       └── wgsl/       # WGSL compute shaders
│   ├── sha3-wasm/          # WASM bindings
│   └── sha3-bench/         # Benchmarking suite
└── pkg/                    # Generated WASM package (gitignored)
```

## License

MIT OR Apache-2.0

