# Agent Task: Fix GPU SHA-3 Hash Mismatch

## Problem Statement

The GPU-based SHA-3 implementation produces **incorrect hash outputs** that don't match the standard CPU SHA-3 library (js-sha3).

## Evidence

**Test Case**: 64-byte input with "test input number 0" + null bytes
- **Expected Hash (CPU)**: `fc7b90c60bd458578860c218cc0e21726fd1eb34a85d00e88320bed886f85f4c`
- **Actual Hash (GPU)**: `2ceb9ee0bea7c5f30e98cbcda8b5e4ed00b77c7444bf5d021c9c15063bdbb147` ❌

**Verified Test Vectors** (CPU implementation is correct):
```
SHA3-256('')    = a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a ✓
SHA3-256('abc') = 3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532 ✓
```

## Root Cause Analysis

The GPU implementation is in the WGSL shader at `crates/sha3-wgpu/src/wgsl/sha3.wgsl`.

**Most Likely Issue**: Endianness bug in 64-bit lane representation.

Keccak-f[1600] operates on 64-bit lanes, but WGSL doesn't have native 64-bit types. The implementation uses `vec2<u32>` to represent each 64-bit value as `(high_u32, low_u32)`.

### Critical Functions to Investigate:

1. **`bytes_to_u64()`** (lines 110-127)
   - Converts 8 bytes → `vec2<u32>`
   - May have byte order reversed within each u32
   - May have high/low components swapped

2. **`u64_to_bytes()`** (lines 129-147)
   - Converts `vec2<u32>` → 8 bytes
   - Must reverse the exact logic of `bytes_to_u64()`

3. **Input/Output packing** (lines 244-252, 279-307)
   - How bytes are packed into u32 arrays
   - How u32 arrays are unpacked to bytes

## Investigation Steps

### Step 1: Examine Byte-to-U64 Conversion
Look at `bytes_to_u64()` in `sha3.wgsl`:
- How are bytes combined into each u32?
- Is it little-endian or big-endian within each u32?
- Is `vec2<u32>(high, low)` or `vec2<u32>(low, high)` correct?
- Compare with Keccak specification's byte ordering

### Step 2: Check Against Reference
Compare with a working SHA-3 implementation:
- Look at `crates/sha3-wgpu/src/lib.rs` tests that use the `sha3` crate
- The tests may already be failing or passing incorrectly

### Step 3: Test Minimal Cases
Run these test cases through the GPU implementation:
```rust
// Empty input
let result = hasher.hash_single(&[]).await?;
// Should produce: a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a

// "abc"
let result = hasher.hash_single(b"abc").await?;
// Should produce: 3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532
```

## Expected Fix

The fix is likely a **simple endianness correction** in one or both of:

1. **Swap high/low in `bytes_to_u64()`**:
   ```wgsl
   // Current:
   return vec2<u32>(high, low);

   // Try:
   return vec2<u32>(low, high);
   ```

2. **Reverse byte order within each u32**:
   ```wgsl
   // Current (little-endian):
   let low = bytes[0] | (bytes[1] << 8u) | (bytes[2] << 16u) | (bytes[3] << 24u);

   // Try (big-endian):
   let low = (bytes[0] << 24u) | (bytes[1] << 16u) | (bytes[2] << 8u) | bytes[3];
   ```

3. **Ensure `u64_to_bytes()` is the exact inverse** of whatever `bytes_to_u64()` does

## Verification Process

### 1. Run Rust Tests
```bash
cd /home/user/sha3-wgpu
cargo test --lib sha3_wgpu -- --nocapture
```
All tests should pass with correct hashes.

### 2. Build WASM
```bash
npm run build
```
Should build successfully.

### 3. Test in Browser
```bash
cd examples/browser
npm run dev
```
- Navigate to "Performance Comparison" tab
- Click "Run Benchmark"
- **Success Criteria**: All results show "✓ PASS" in green
- Console should show: "✓ All GPU hashes match CPU SHA-3 hashes - Implementation is correct!"

### 4. Quick Manual Verification
```bash
cd examples/browser
node -e "
const { sha3_256 } = require('js-sha3');
console.log('Empty:', sha3_256(''));
console.log('abc:', sha3_256('abc'));
"
```
Compare these against GPU output.

## Success Criteria

✅ GPU hash for empty string matches: `a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a`
✅ GPU hash for "abc" matches: `3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532`
✅ All `cargo test` tests pass
✅ Browser demo shows "✓ PASS" for all batch sizes
✅ No console errors about hash mismatches

## Deliverables

1. Fix the endianness issue in `crates/sha3-wgpu/src/wgsl/sha3.wgsl`
2. Verify all tests pass
3. Verify browser demo shows all hashes matching
4. Commit with message explaining the root cause and fix
5. Push to branch: `claude/gpu-cpu-hash-compare-011CUw6537qniMPJQ4fUWNy4`

## Additional Context

- The CPU comparison UI was just added to `examples/browser/src/components/PerformanceDemo.tsx`
- It compares GPU SHA-3 against `js-sha3` library
- The comparison revealed that GPU hashes are consistently wrong
- This is a critical correctness bug that must be fixed before the library can be used
- The Keccak-f[1600] permutation rounds may be correct, but byte-to-lane conversion is likely wrong

## Files to Modify

**Primary**: `crates/sha3-wgpu/src/wgsl/sha3.wgsl` (lines 110-147)
**Secondary** (if needed): `crates/sha3-wgpu/src/compute.rs` (input/output packing)

## Key Insight

The GPU implementation uses `vec2<u32>` to represent 64-bit values. The ordering of bytes within this representation must exactly match how Keccak expects 64-bit lanes to be laid out in memory. A single byte-order mistake will cause completely incorrect hash outputs.

---

**Start by examining `bytes_to_u64()` and `u64_to_bytes()` in sha3.wgsl - the bug is almost certainly there.**
