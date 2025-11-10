# SHA3-256 GPU Optimization Guide

This guide documents key GPU optimizations for SHA3-256 hashing based on CUDA/OpenCL best practices, adapted for WebGPU/WGSL implementation.

## Optimization Checklist

| Issue | Current Status | Fix | Expected Impact |
|-------|---------------|------|----------------|
| Global memory thrashing | Round constants in constant memory ✅<br>State in registers ❌ | Use shared memory for round constants & state | Medium |
| Poor occupancy | 256 threads/block ✅ | Align to warp boundaries | Low |
| No ILP | Rounds in loop ❌ | Unroll rounds, improve instruction scheduling | High |
| No coalescing | AoS (Array of Structures) ❌ | Store messages in SoA (Structure of Arrays) | High |
| Launch overhead | Basic persistent buffers ✅ | Use persistent kernels or large batches | Medium |

## 1. Global Memory Thrashing → Shared Memory

### Problem
- Round constants and intermediate state stored in global memory
- Frequent accesses cause memory thrashing
- Poor cache locality

### Solution: Use Shared Memory

#### WGSL Implementation:
```wgsl
// Add shared memory declarations
var<workgroup> shared_rc: array<vec2<u32>, 24>;
var<workgroup> shared_state: array<array<vec2<u32>, 25>, 256>; // One state per thread

@compute @workgroup_size(256, 1, 1)
fn main(@builtin(local_invocation_id) local_id: vec3<u32>,
        @builtin(global_invocation_id) global_id: vec3<u32>) {

    // Load round constants to shared memory once per workgroup
    if (local_id.x == 0u) {
        for (var i = 0u; i < 24u; i = i + 1u) {
            shared_rc[i] = RC[i];
        }
    }
    workgroupBarrier();

    // Use shared memory for state operations
    let thread_state = &shared_state[local_id.x];

    // ... rest of computation using shared_state
}
```

#### Benefits:
- Reduced global memory accesses
- Better cache locality
- Inter-thread data sharing opportunities

## 2. Poor Occupancy → Warp-Aligned Execution

### Problem
- Thread blocks not aligned to warp boundaries
- Inefficient SIMD utilization
- Memory access patterns not optimized for GPU warps

### Solution: Align to Warp Boundaries

#### WGSL Implementation:
```wgsl
const WARP_SIZE = 32u;

@compute @workgroup_size(256, 1, 1) // 256 = 8 warps
fn main(@builtin(global_invocation_id) global_id: vec3<u32>,
        @builtin(local_invocation_id) local_id: vec3<u32>) {

    let warp_id = local_id.x / WARP_SIZE;
    let lane_id = local_id.x % WARP_SIZE;

    // Ensure memory accesses are warp-coalesced
    let warp_aligned_offset = (global_id.x / WARP_SIZE) * WARP_SIZE;
    let thread_offset = lane_id;

    // Load data with coalesced access pattern
    // All threads in warp access consecutive memory locations
}
```

#### Benefits:
- Perfect SIMD utilization
- Coalesced memory accesses
- Maximum hardware efficiency

## 3. No ILP → Unroll Rounds

### Problem
- Keccak rounds executed in a loop
- Limits instruction-level parallelism (ILP)
- Loop overhead reduces performance
- No `__restrict__` equivalent in WGSL

### Solution: Manual Round Unrolling

#### WGSL Implementation:
```wgsl
// Replace this:
for (var round = 0u; round < KECCAK_ROUNDS; round = round + 1u) {
    keccak_round(&state, round);
}

// With this:
keccak_round_0(&state);
keccak_round_1(&state);
keccak_round_2(&state);
// ... manually unroll all 24 rounds
keccak_round_23(&state);

// Each round function with inlined operations
fn keccak_round_0(state: ptr<function, array<vec2<u32>, 25>>) {
    // Theta step - fully unrolled
    let bc0 = xor_u64(xor_u64(xor_u64(xor_u64((*state)[0], (*state)[5]), (*state)[10]), (*state)[15]), (*state)[20]);
    let bc1 = xor_u64(xor_u64(xor_u64(xor_u64((*state)[1], (*state)[6]), (*state)[11]), (*state)[16]), (*state)[21]);
    // ... continue for all 5 bc values

    // Apply theta diffusion
    let t0 = xor_u64(bc4, rotl_u64(bc1, 1u));
    (*state)[0] = xor_u64((*state)[0], t0);
    (*state)[5] = xor_u64((*state)[5], t0);
    // ... continue for all state positions

    // Rho + Pi + Chi steps fully unrolled for round 0
    // ... (full implementation for each round)
}
```

#### Benefits:
- Eliminates loop overhead
- Maximum ILP exploitation
- Better register allocation
- Improved instruction scheduling

## 4. No Coalescing → Structure of Arrays (SoA)

### Problem
- Input data stored as Array of Structures (AoS)
- Each hash input is contiguous in memory
- Threads access non-consecutive memory locations
- Poor memory coalescing

### Solution: Structure of Arrays (SoA)

#### Memory Layout Change:
```rust
// Current AoS layout (Array of Structures):
// [hash0_byte0, hash0_byte1, ..., hash0_byte63,
//  hash1_byte0, hash1_byte1, ..., hash1_byte63,
//  ...]

// New SoA layout (Structure of Arrays):
// [hash0_byte0, hash1_byte0, hash2_byte0, ...,
//  hash0_byte1, hash1_byte1, hash2_byte1, ...,
//  ...]
```

#### WGSL Implementation:
```wgsl
// SoA input buffer structure
struct SoAInput {
    @align(16) byte_arrays: array<array<u32, MAX_HASHES>, 64>, // 64 arrays of MAX_HASHES u32s
}

// Coalesced loading
fn load_soa_input(hash_idx: u32, byte_offset: u32, byte_in_word: u32) -> u32 {
    let array_idx = byte_offset / 4u;
    // All threads in warp access consecutive elements in the same array
    return inputs.byte_arrays[array_idx][hash_idx];
}
```

#### Benefits:
- Perfect memory coalescing
- All threads in a warp access consecutive memory locations
- Maximum memory bandwidth utilization
- Reduced memory transactions

## 5. Launch Overhead → Persistent Kernels

### Problem
- Multiple kernel launches for SHA3 phases
- GPU launch overhead accumulates
- CPU-GPU synchronization between phases

### Solution: Kernel Fusion & Persistence

#### Single Kernel Approach:
```wgsl
@compute @workgroup_size(256, 1, 1)
fn sha3_batch_kernel(...) {
    // Phase 1: Load and pad inputs
    // Phase 2: Absorbing phase (multiple rounds)
    // Phase 3: Squeezing phase
    // All in one kernel invocation
}
```

#### Persistent Kernel Approach:
```wgsl
// Keep kernel running and process multiple batches
var<workgroup> persistent_buffer: array<u32, PERSISTENT_SIZE>;

@compute @workgroup_size(256, 1, 1)
fn persistent_sha3_kernel(@builtin(num_workgroups) num_workgroups: vec3<u32>) {
    // Process batches until no more work
    loop {
        // Load batch parameters from persistent buffer
        // Process batch
        // Check for more work or exit
    }
}
```

#### Benefits:
- Reduced launch overhead
- Better CPU-GPU utilization
- Improved throughput for large batch processing

## Implementation Priority

### Phase 1 (High Impact, Low Risk):
1. **Unroll rounds** - Immediate performance gain, straightforward
2. **SoA memory layout** - Better memory bandwidth, requires data reorganization

### Phase 2 (Medium Impact, Medium Risk):
3. **Shared memory** - Cache locality improvements, inter-thread optimizations
4. **Persistent kernels** - Reduced overhead, complex implementation

### Phase 3 (Low Impact, High Risk):
5. **Warp alignment** - Already mostly implemented, marginal gains

## Performance Expectations

| Optimization | Expected Speedup | Implementation Effort |
|--------------|------------------|----------------------|
| Round unrolling | 1.5-2.0x | Medium |
| SoA layout | 1.3-1.8x | High |
| Shared memory | 1.1-1.4x | Medium |
| Persistent kernels | 1.2-1.6x | High |
| Combined | 3.0-5.0x | Very High |

## Measurement Methodology

- Use Criterion.rs with `Throughput::Elements` for hash/second reporting
- Test batch sizes: 1K, 10K, 100K, 1M hashes
- Input size: 64 bytes fixed
- Measure both latency and throughput
- Compare against optimized CPU SHA3 implementations

## Testing Strategy

1. **Unit tests**: Verify correctness after each optimization
2. **Performance regression tests**: Ensure no slowdown
3. **Memory usage validation**: Check for memory leaks/coherency
4. **Cross-GPU testing**: Validate on different GPU architectures

## WGSL Limitations & Workarounds

- No `__restrict__` equivalent: Use explicit variable separation
- No dynamic shared memory: Pre-allocate maximum sizes
- No function pointers: Manual code duplication for round functions
- Limited loop unrolling: Compiler may not unroll deeply nested loops

This guide provides the roadmap for optimizing SHA3-256 GPU performance while maintaining correctness and portability across WebGPU implementations.
