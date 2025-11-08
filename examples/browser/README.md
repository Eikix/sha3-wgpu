# SHA-3 WebGPU Browser Demo

A minimal Vite + React application demonstrating GPU-accelerated SHA-3 hashing in the browser using WebGPU.

## Why Browser-Only?

**WebGPU is only supported in modern browsers** (Chrome/Edge 113+, Firefox with flag). Node.js and Bun.js do not support WebGPU, which is why we need a browser-based environment to test the WASM WebGPU functions.

## Prerequisites

1. **Browser with WebGPU support:**
   - Chrome/Edge 113+ (recommended)
   - Firefox Nightly with `dom.webgpu.enabled` flag
   - Check support at [webgpu.io](https://webgpu.io)

2. **Build the WASM module** (from repository root):
   ```bash
   npm run build
   ```

3. **Install dependencies** (in this directory):
   ```bash
   npm install
   ```

## Running the Demo

From the repository root:
```bash
npm run demo
```

Or from this directory:
```bash
npm run dev
```

Then open your browser to the URL shown (typically `http://localhost:5173`).

## Features

The demo includes three interactive sections:

### 1. Basic Usage
- Single hash computation
- Batch hashing multiple inputs
- View SHA-3 hashes in real-time

### 2. Performance Comparison
- Benchmark GPU vs CPU performance
- Test different batch sizes (10, 50, 100, 500, 1000)
- See speedup metrics and throughput

### 3. All SHA-3 Variants
- Test all SHA-3 variants:
  - SHA3-224 (28 bytes)
  - SHA3-256 (32 bytes)
  - SHA3-384 (48 bytes)
  - SHA3-512 (64 bytes)
  - SHAKE128 (variable)
  - SHAKE256 (variable)

## How It Works

1. **WASM Module**: The Rust code is compiled to WebAssembly using `wasm-pack`
2. **WebGPU Integration**: The WASM module uses WebGPU APIs to run compute shaders on the GPU
3. **React UI**: Provides an interactive interface to test the hashing functions
4. **Vite**: Bundles the application with proper WASM support

## Troubleshooting

### WebGPU Not Supported
If you see "WebGPU is not supported":
- Update to Chrome/Edge 113 or later
- Enable WebGPU in Firefox Nightly: `about:config` → `dom.webgpu.enabled` → `true`
- Check [caniuse.com/webgpu](https://caniuse.com/webgpu) for browser compatibility

### WASM Module Not Found
If you get import errors:
1. Ensure you've built the WASM module: `npm run build` (from root)
2. The `pkg` directory should exist in the repository root

### Performance Issues
- First hash may be slower due to GPU initialization
- GPU performance improves with larger batch sizes (100+ hashes)
- Try the Performance tab to see optimal batch sizes

## Technical Stack

- **Vite**: Fast build tool with native ESM support
- **React 18**: UI framework
- **TypeScript**: Type-safe development
- **vite-plugin-wasm**: WASM module support
- **vite-plugin-top-level-await**: Async WASM initialization

## Building for Production

```bash
npm run build
```

The built files will be in the `dist` directory. You can preview the production build with:

```bash
npm run preview
```

## Learn More

- [WebGPU Specification](https://gpuweb.github.io/gpuweb/)
- [SHA-3 Standard (FIPS 202)](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.202.pdf)
- [wgpu-rs Documentation](https://docs.rs/wgpu/latest/wgpu/)
- [wasm-bindgen Guide](https://rustwasm.github.io/wasm-bindgen/)
