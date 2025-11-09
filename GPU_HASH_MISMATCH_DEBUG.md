# GPU SHA-3 Hash Mismatch - Diagnostic Prompt

## Problem Summary

The GPU SHA-3 implementation is producing **incorrect hash outputs** that do not match the standard JavaScript SHA-3 library (js-sha3) or official SHA-3 test vectors.

## Evidence

### Console Output from Browser Testing:
```
Hash mismatch at index 0:
  cpu: "fc7b90c60bd458578860c218cc0e21726fd1eb34a85d00e88320bed886f85f4c"
  gpu: "2ceb9ee0bea7c5f30e98cbcda8b5e4ed00b77c7444bf5d021c9c15063bdbb147"
  input: "test input number 0\0\0\0..." (64 bytes total)
```

### Test Case Details:
- **Input**: 64-byte buffer with "test input number 0" (19 bytes) + 45 null bytes
- **Input Hex**: `7465737420696e707574206e756d6265722030` + 45×`00`
- **Expected SHA3-256** (verified with js-sha3): `fc7b90c60bd458578860c218cc0e21726fd1eb34a85d00e88320bed886f85f4c`
- **GPU Output**: `2ceb9ee0bea7c5f30e98cbcda8b5e4ed00b77c7444bf5d021c9c15063bdbb147` ❌

### Verified Correct Reference Hashes:
```javascript
// These are CORRECT (from js-sha3 matching NIST test vectors):
sha3_256('')    = 'a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a'
sha3_256('abc') = '3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532'
```

## Architecture Overview

```
Browser Input (Uint8Array)
    ↓
WASM Bindings (sha3-wasm/src/lib.rs)
    ↓
GPU Compute Module (crates/sha3-wgpu/src/compute.rs)
    ↓
WGSL Shader (crates/sha3-wgpu/src/wgsl/sha3.wgsl)
    ↓
GPU Execution → Output Buffer
    ↓
Back to JavaScript (Uint8Array)
```

## Likely Root Causes (Ranked by Probability)

### 1. **Endianness Issues in 64-bit Lane Representation** ⚠️ HIGH
- **Location**: `sha3.wgsl` - functions `bytes_to_u64()` and `u64_to_bytes()`
- **Issue**: Keccak operates on 64-bit lanes. The shader represents these as `vec2<u32>` with `(high, low)` components
- **Problem**: Byte ordering when converting between bytes and vec2<u32> may be reversed
- **Check**: Lines 110-127 (bytes_to_u64) and lines 129-147 (u64_to_bytes)

### 2. **Multi-Layer Byte Packing/Unpacking** ⚠️ HIGH
- **Location**: `compute.rs` (lines 167-172) and `sha3.wgsl` (lines 244-252, 291-298)
- **Issue**: Data is packed into u32 arrays by Rust, then unpacked byte-by-byte by shader, then repacked
- **Problem**: Each conversion layer could introduce byte-order bugs
- **Example**: Rust packs little-endian, shader assumes big-endian (or vice versa)

### 3. **Padding Implementation** ⚠️ MEDIUM
- **Location**: `sha3.wgsl` - function `apply_padding()` (lines 201-223)
- **Issue**: SHA-3 uses pad10*1 pattern: append `0x06`, pad with zeros, set last bit to `0x80`
- **Problem**: Intermediate bytes between `0x06` and `0x80` may not be properly zeroed
- **Check**: Lines 214-220 - verify padding bytes are explicitly cleared

### 4. **State Initialization** ⚠️ MEDIUM
- **Location**: `sha3.wgsl` - function `sha3_hash()` (lines 226-239)
- **Issue**: Keccak state must start as all zeros (25 lanes × 64 bits)
- **Problem**: State array may not be properly initialized before use
- **Check**: Line 237 - `var state: array<vec2<u32>, 25>;` - is this zero-initialized?

### 5. **Absorption Phase Lane Offset** ⚠️ LOW
- **Location**: `sha3.wgsl` - lines 259-273
- **Issue**: Input bytes are XORed into state lanes during absorption
- **Problem**: Offset calculation `offset + i * 8u + j` may be incorrect
- **Check**: Verify lane indexing matches Keccak specification

## Files to Investigate

| Priority | File | Lines | Purpose |
|----------|------|-------|---------|
| **CRITICAL** | `crates/sha3-wgpu/src/wgsl/sha3.wgsl` | 110-147 | Byte/u64 conversion functions |
| **CRITICAL** | `crates/sha3-wgpu/src/wgsl/sha3.wgsl` | 201-223 | Padding application |
| **CRITICAL** | `crates/sha3-wgpu/src/wgsl/sha3.wgsl` | 259-273 | Absorption phase |
| **CRITICAL** | `crates/sha3-wgpu/src/wgsl/sha3.wgsl` | 279-307 | Squeezing phase |
| **HIGH** | `crates/sha3-wgpu/src/compute.rs` | 167-172 | Input data flattening |
| **MEDIUM** | `crates/sha3-wgpu/src/compute.rs` | 244-252 | Input buffer extraction |

## Debugging Strategy

### Step 1: Add Logging/Debugging
Add intermediate state logging to compare GPU vs reference implementation at each stage:
1. After padding
2. After each absorption round
3. After Keccak-f[1600] permutation
4. During squeezing

### Step 2: Test with Minimal Input
Start with the simplest test case:
- **Empty string**: Should produce `a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a`
- If this fails, the issue is in core Keccak or padding

### Step 3: Test "abc"
- **Input**: "abc" (3 bytes)
- **Expected**: `3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532`
- If empty works but this fails, issue is in input handling

### Step 4: Compare Against Reference Implementation
Use the Rust `sha3` crate in the existing tests (`crates/sha3-wgpu/src/lib.rs`) to verify:
```rust
#[cfg(test)]
mod tests {
    use sha3::{Digest, Sha3_256};

    #[test]
    fn test_known_vectors() {
        // Test empty
        let hash = Sha3_256::digest(b"");
        println!("Reference empty: {:x}", hash);

        // Test abc
        let hash = Sha3_256::digest(b"abc");
        println!("Reference abc: {:x}", hash);
    }
}
```

### Step 5: Verify Byte Order in State
Print the first few state lanes after absorption to compare with reference:
- Keccak state is 5×5 lanes (25 total) of 64-bit values
- For "abc", after first absorption, specific state pattern should match reference

## Expected Fix Locations

Based on analysis, the fix is most likely in **ONE** of these functions:

1. **`bytes_to_u64()` in sha3.wgsl** - Reverse byte order:
   ```wgsl
   // Current (possibly wrong):
   let low = bytes[0] | (bytes[1] << 8u) | (bytes[2] << 16u) | (bytes[3] << 24u);
   let high = bytes[4] | (bytes[5] << 8u) | (bytes[6] << 16u) | (bytes[7] << 24u);
   return vec2<u32>(high, low);

   // May need to be:
   return vec2<u32>(low, high); // Swap high/low
   // OR reverse byte order within each u32
   ```

2. **`u64_to_bytes()` in sha3.wgsl** - Reverse extraction order:
   ```wgsl
   // Ensure bytes are extracted in correct endian order
   ```

3. **Input buffer packing in compute.rs** - Change endianness:
   ```rust
   // Verify u32 packing matches shader expectations
   ```

## Verification Commands

### Run Rust Tests:
```bash
cd /home/user/sha3-wgpu
cargo test --lib sha3_wgpu -- --nocapture
```

### Build and Test in Browser:
```bash
npm run build
cd examples/browser
npm run dev
# Navigate to Performance Comparison tab
# Click "Run Benchmark"
# Check console for hash matches
```

### Quick Node Test:
```bash
cd examples/browser
node -e "
const { sha3_256 } = require('js-sha3');
console.log('Empty:', sha3_256(''));
console.log('abc:', sha3_256('abc'));
"
```

## Success Criteria

The fix is correct when:
1. ✅ GPU hash matches `js-sha3` for empty string
2. ✅ GPU hash matches `js-sha3` for "abc"
3. ✅ GPU hash matches `js-sha3` for "test input number 0" (64 bytes)
4. ✅ All Rust tests pass (`cargo test`)
5. ✅ Browser PerformanceDemo shows "✓ PASS" for all batch sizes
6. ✅ Console shows "All GPU hashes match CPU SHA-3 hashes"

## Additional Context

- The CPU implementation (js-sha3) is **verified correct** against NIST test vectors
- Recent commits show "fix" messages, suggesting known issues
- The verification UI was just added, which revealed this mismatch
- All comparisons should use **SHA3-256** variant (not SHAKE or other variants)

---

## PROMPT FOR AGENT:

**Task**: Fix the GPU SHA-3 implementation that is producing incorrect hash outputs.

**Current Issue**: GPU hashes don't match CPU SHA-3 hashes from the `js-sha3` library. For example, input "test input number 0" (64 bytes) produces:
- Expected (CPU): `fc7b90c60bd458578860c218cc0e21726fd1eb34a85d00e88320bed886f85f4c`
- Actual (GPU): `2ceb9ee0bea7c5f30e98cbcda8b5e4ed00b77c7444bf5d021c9c15063bdbb147`

**Investigation Steps**:
1. Examine `crates/sha3-wgpu/src/wgsl/sha3.wgsl` focusing on:
   - `bytes_to_u64()` (lines 110-127) - check endianness
   - `u64_to_bytes()` (lines 129-147) - check byte extraction order
   - `apply_padding()` (lines 201-223) - verify pad10*1 pattern
   - Absorption (lines 259-273) and squeezing (lines 279-307) phases

2. Test with known vectors:
   - Empty string → `a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a`
   - "abc" → `3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532`

3. Run `cargo test` to see if existing tests catch the issue

4. Most likely fix: Reverse byte order in `bytes_to_u64()` or swap `vec2<u32>(high, low)` to `vec2<u32>(low, high)`

5. Verify fix by running browser demo Performance Comparison - all benchmarks should show "✓ PASS"

**Files to Modify**: Primarily `crates/sha3-wgpu/src/wgsl/sha3.wgsl`

**Success**: All hashes match between GPU and CPU implementations in the browser demo.
