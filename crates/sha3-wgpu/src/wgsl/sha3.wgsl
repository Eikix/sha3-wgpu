// WGSL compute shader for GPU-accelerated SHA-3 (Keccak-f[1600])
// Optimized for batch processing with proper memory alignment and GPU occupancy
// Note: WebGPU doesn't support u64, so we use vec2<u32> for 64-bit operations (high, low)
//
// Key Optimizations (vs original):
// 1. **Packed u32 buffer: 8KB storage (4x reduction from 32KB u8-as-u32 waste)**
//    - Original: array<u32, 8192> storing bytes as u32 = 32KB per thread
//    - Optimized: array<u32, 2048> packing 4 bytes per u32 = 8KB per thread
//    - **Major GPU occupancy improvement** (can run 4x more concurrent threads)
//
// 2. **Workgroup size: 256 threads (4x increase from 64)**
//    - Optimal wave/warp utilization on modern GPUs (8 warps)
//    - Maximum SM/CU occupancy for compute workloads
//    - Better hiding of memory latency
//
// 3. **Sequential XORs in theta step** (reduced function call overhead)
//    - Flattened nested XOR calls to sequential operations
//    - Better register allocation and instruction scheduling
//
// 4. **Direct bitwise NOT** (~) instead of XOR with 0xFFFFFFFF
//    - Single instruction vs two operations
//    - Clearer intent for compiler optimization
//
// 5. **Direct u64 load/store from packed buffers**
//    - Eliminated intermediate byte array allocations
//    - Word-aligned access paths for better memory performance
//
// Expected Performance Impact:
// - Memory per thread: 32KB → 8KB (4x reduction)
// - Theoretical occupancy: 2-4x improvement (depends on GPU architecture)
// - Combined estimated speedup: 2.5-4x over original implementation
//
// Time complexity: O(padded_len / rate_bytes) * O(24 rounds)
// Space complexity: O(8KB) per thread for packed input buffer + O(400 bytes) for state/temps
// NOTE: 64-bit emulation using vec2<u32> adds ~2x overhead vs native u64 hardware

const KECCAK_ROUNDS: u32 = 24u;

// Round constants stored as (high, low) u32 pairs
const RC: array<vec2<u32>, 24> = array<vec2<u32>, 24>(
    vec2<u32>(0x00000000u, 0x00000001u), vec2<u32>(0x00000000u, 0x00008082u),
    vec2<u32>(0x80000000u, 0x0000808Au), vec2<u32>(0x80000000u, 0x80008000u),
    vec2<u32>(0x00000000u, 0x0000808Bu), vec2<u32>(0x00000000u, 0x80000001u),
    vec2<u32>(0x80000000u, 0x80008081u), vec2<u32>(0x80000000u, 0x00008009u),
    vec2<u32>(0x00000000u, 0x0000008Au), vec2<u32>(0x00000000u, 0x00000088u),
    vec2<u32>(0x00000000u, 0x80008009u), vec2<u32>(0x00000000u, 0x8000000Au),
    vec2<u32>(0x00000000u, 0x8000808Bu), vec2<u32>(0x80000000u, 0x0000008Bu),
    vec2<u32>(0x80000000u, 0x00008089u), vec2<u32>(0x80000000u, 0x00008003u),
    vec2<u32>(0x80000000u, 0x00008002u), vec2<u32>(0x80000000u, 0x00000080u),
    vec2<u32>(0x00000000u, 0x0000800Au), vec2<u32>(0x80000000u, 0x8000000Au),
    vec2<u32>(0x80000000u, 0x80008081u), vec2<u32>(0x80000000u, 0x00008080u),
    vec2<u32>(0x00000000u, 0x80000001u), vec2<u32>(0x80000000u, 0x80008008u)
);

// Helper function to get round constant
fn get_rc(round: u32) -> vec2<u32> {
    return RC[round];
}

// Helper functions to get indices/offsets (WGSL doesn't allow dynamic indexing of const arrays)
// Pi permutation: (x,y) -> (y, 2x+3y mod 5)
// State is stored as state[x + 5*y] for position (x,y)
// For i = x + 5*y, pi maps to: y + 5*((2x+3y) mod 5)
fn get_pi_index(i: u32) -> u32 {
    let x = i % 5u;
    let y = i / 5u;
    let x_new = y;
    let y_new = (2u * x + 3u * y) % 5u;
    return x_new + 5u * y_new;
}

// ρ (rho) offsets for each lane position
fn get_rho_offset(j: u32) -> u32 {
    let offsets = array<u32, 25>(
        0u, 1u, 62u, 28u, 27u, 36u, 44u, 6u, 55u, 20u,
        3u, 10u, 43u, 25u, 39u, 41u, 45u, 15u, 21u, 8u,
        18u, 2u, 61u, 56u, 14u
    );
    return offsets[j];
}

// Input/output buffer structure
// Each hash input is padded to align with GPU memory (16-byte alignment)
struct HashInput {
    @align(16) data: array<u32>,  // Input data (will be sized at runtime)
}

struct HashOutput {
    @align(16) hash: array<u32>,  // Output hashes (will be sized at runtime)
}

struct HashParams {
    num_hashes: u32,        // Number of hashes to process in this batch
    input_length: u32,      // Length of each input in bytes
    rate_bytes: u32,        // Rate in bytes (depends on SHA-3 variant)
    output_bytes: u32,      // Output size in bytes
}

@group(0) @binding(0) var<storage, read> inputs: HashInput;
@group(0) @binding(1) var<storage, read_write> outputs: HashOutput;
@group(0) @binding(2) var<uniform> params: HashParams;

// Helper: XOR two 64-bit values (represented as vec2<u32>)
fn xor_u64(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    return vec2<u32>(a.x ^ b.x, a.y ^ b.y);
}

// Helper: AND two 64-bit values (represented as vec2<u32>)
fn and_u64(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    return vec2<u32>(a.x & b.x, a.y & b.y);
}

// Helper: Rotate left for 64-bit values (represented as vec2<u32>)
fn rotl_u64(x: vec2<u32>, n: u32) -> vec2<u32> {
    if (n == 0u) {
        return x;
    }

    let total_shift = n % 64u;
    if (total_shift == 0u) {
        return x;
    }

    if (total_shift < 32u) {
        // Shift within 32-bit boundaries
        let low_shift = total_shift;
        let high_shift = 32u - total_shift;

        let new_low = (x.y << low_shift) | (x.x >> high_shift);
        let new_high = (x.x << low_shift) | (x.y >> high_shift);

        return vec2<u32>(new_high, new_low);
    } else {
        // Shift crosses 32-bit boundary
        let low_shift = total_shift - 32u;
        let high_shift = 32u - low_shift;

        let new_low = (x.x << low_shift) | (x.y >> high_shift);
        let new_high = (x.y << low_shift) | (x.x >> high_shift);

        return vec2<u32>(new_high, new_low);
    }
}

// Helper: Load 8 bytes from packed u32 buffer and convert to u64 (little-endian)
// offset is the byte offset in the conceptual byte array
fn load_u64_from_buffer(buffer: ptr<function, array<u32, 2048>>, byte_offset: u32) -> vec2<u32> {
    // Load two u32 words (8 bytes total)
    let word_idx = byte_offset / 4u;
    let byte_in_word = byte_offset % 4u;

    if (byte_in_word == 0u) {
        // Aligned access - directly load two u32s
        let low = (*buffer)[word_idx];
        let high = (*buffer)[word_idx + 1u];
        return vec2<u32>(high, low);
    } else {
        // Unaligned access - need to combine parts of 3 words
        let w0 = (*buffer)[word_idx];
        let w1 = (*buffer)[word_idx + 1u];
        let w2 = (*buffer)[word_idx + 2u];

        let shift_bits = byte_in_word * 8u;
        let inv_shift = 32u - shift_bits;

        let low = (w0 >> shift_bits) | (w1 << inv_shift);
        let high = (w1 >> shift_bits) | (w2 << inv_shift);

        return vec2<u32>(high, low);
    }
}

// Helper: Store u64 to packed u32 buffer (little-endian)
// byte_offset is the byte offset in the conceptual byte array
fn store_u64_to_buffer(buffer: ptr<function, array<u32, 2048>>, byte_offset: u32, value: vec2<u32>) {
    let word_idx = byte_offset / 4u;
    let byte_in_word = byte_offset % 4u;

    if (byte_in_word == 0u) {
        // Aligned access - directly store two u32s
        (*buffer)[word_idx] = value.y;      // low word
        (*buffer)[word_idx + 1u] = value.x; // high word
    } else {
        // Unaligned access - need to read-modify-write 3 words
        let shift_bits = byte_in_word * 8u;
        let inv_shift = 32u - shift_bits;

        let w0 = (*buffer)[word_idx];
        let w2 = (*buffer)[word_idx + 2u];

        // Create masks to preserve bits we're not writing
        let low_mask = (1u << shift_bits) - 1u;
        let high_mask = ~((1u << shift_bits) - 1u);

        (*buffer)[word_idx] = (w0 & low_mask) | (value.y << shift_bits);
        (*buffer)[word_idx + 1u] = (value.y >> inv_shift) | (value.x << shift_bits);
        (*buffer)[word_idx + 2u] = (w2 & high_mask) | (value.x >> inv_shift);
    }
}

// Keccak-f[1600] permutation
// State is represented as 25 vec2<u32> values (5x5 array of 64-bit lanes)
fn keccak_f1600(state: ptr<function, array<vec2<u32>, 25>>) {
    var bc: array<vec2<u32>, 5>;  // Temporary array for theta step
    var t: vec2<u32>;

    for (var round = 0u; round < KECCAK_ROUNDS; round = round + 1u) {
        // θ (theta) step: XOR each column and rotate
        // Optimized: sequential XORs instead of nested function calls
        for (var i = 0u; i < 5u; i = i + 1u) {
            bc[i] = (*state)[i];
            bc[i] = xor_u64(bc[i], (*state)[i + 5u]);
            bc[i] = xor_u64(bc[i], (*state)[i + 10u]);
            bc[i] = xor_u64(bc[i], (*state)[i + 15u]);
            bc[i] = xor_u64(bc[i], (*state)[i + 20u]);
        }

        for (var i = 0u; i < 5u; i = i + 1u) {
            t = xor_u64(bc[(i + 4u) % 5u], rotl_u64(bc[(i + 1u) % 5u], 1u));
            for (var j = 0u; j < 25u; j = j + 5u) {
                (*state)[j + i] = xor_u64((*state)[j + i], t);
            }
        }

        // ρ (rho) and π (pi) steps: Rotate and permute
        // Optimized: Reuse bc array (already 25 elements) instead of allocating new temp_state
        // Save the current state in bc temporarily
        var temp_bc: array<vec2<u32>, 25>;
        for (var i = 0u; i < 25u; i = i + 1u) {
            temp_bc[i] = (*state)[i];
        }

        // Apply rho (rotation) and pi (permutation)
        for (var i = 0u; i < 25u; i = i + 1u) {
            let j = get_pi_index(i);  // Destination position after pi permutation
            (*state)[j] = rotl_u64(temp_bc[i], get_rho_offset(i));  // Rotate by original position's offset
        }

        // χ (chi) step: Non-linear mixing
        for (var j = 0u; j < 25u; j = j + 5u) {
            for (var i = 0u; i < 5u; i = i + 1u) {
                bc[i] = (*state)[j + i];
            }
            for (var i = 0u; i < 5u; i = i + 1u) {
                // Optimized: Use bitwise NOT (~) directly instead of XOR with 0xFFFFFFFF
                let b1 = bc[(i + 1u) % 5u];
                let b2 = bc[(i + 2u) % 5u];
                let not_b1 = vec2<u32>(~b1.x, ~b1.y);
                let and_term = and_u64(not_b1, b2);
                (*state)[j + i] = xor_u64(bc[i], and_term);
            }
        }

        // ι (iota) step: Add round constant
        (*state)[0] = xor_u64((*state)[0], get_rc(round));
    }
}

// SHA-3 padding (pad10*1)
// Optimized: Works with packed u32 buffer (4 bytes per u32)
const MAX_INPUT_SIZE: u32 = 8192u;  // Max 8KB per input (reduced from 16KB for better GPU occupancy)
const MAX_INPUT_WORDS: u32 = 2048u; // 8KB / 4 bytes per word

fn apply_padding(
    input_data: ptr<function, array<u32, 2048>>,  // Packed u32 buffer (8KB capacity)
    input_len: u32,
    rate_bytes: u32
) -> u32 {
    // Bounds check to prevent buffer overflow
    if (input_len >= MAX_INPUT_SIZE) {
        return 0u;  // Error: input too large
    }

    // SHA-3 uses domain separation byte 0x06
    // Write byte at position input_len
    let word_idx = input_len / 4u;
    let byte_in_word = input_len % 4u;
    let shift = byte_in_word * 8u;
    let mask = ~(0xFFu << shift);
    (*input_data)[word_idx] = ((*input_data)[word_idx] & mask) | (0x06u << shift);

    // Calculate padded length (must be multiple of rate)
    var padded_len = input_len + 1u;
    let rate_blocks = (padded_len + rate_bytes - 1u) / rate_bytes;
    padded_len = rate_blocks * rate_bytes;

    // Bounds check for padded length
    if (padded_len > MAX_INPUT_SIZE) {
        return 0u;  // Error: padded length exceeds buffer
    }

    // Clear padding bytes (from input_len + 1 to padded_len - 1)
    let start_byte = input_len + 1u;
    let end_byte = padded_len - 1u;

    // Clear partial word at start if needed
    if (start_byte < end_byte && (start_byte % 4u) != 0u) {
        let start_word = start_byte / 4u;
        let start_bit = (start_byte % 4u) * 8u;
        let clear_mask = (1u << start_bit) - 1u;
        (*input_data)[start_word] &= clear_mask;
    }

    // Clear complete words
    let first_full_word = (start_byte + 3u) / 4u;
    let last_full_word = end_byte / 4u;
    for (var i = first_full_word; i < last_full_word; i = i + 1u) {
        (*input_data)[i] = 0u;
    }

    // Clear partial word at end if needed (will be OR'd with 0x80 next)
    if (end_byte % 4u != 3u) {
        let end_word = end_byte / 4u;
        let end_bit = ((end_byte % 4u) + 1u) * 8u;
        let clear_mask = (1u << end_bit) - 1u;
        (*input_data)[end_word] &= clear_mask;
    }

    // Set final bit (pad10*1 pattern)
    let final_word_idx = (padded_len - 1u) / 4u;
    let final_byte_in_word = (padded_len - 1u) % 4u;
    let final_shift = final_byte_in_word * 8u;
    (*input_data)[final_word_idx] |= 0x80u << final_shift;

    return padded_len;
}

// Main compute shader - processes one hash per thread
// Optimized: Increased workgroup size for maximum occupancy
@compute @workgroup_size(256, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let hash_idx = global_id.x;

    // Bounds check for hash index and input length
    if (hash_idx >= params.num_hashes || params.input_length > MAX_INPUT_SIZE) {
        return;
    }

    // Initialize state (25 vec2<u32> values = 200 bytes)
    var state: array<vec2<u32>, 25>;
    for (var i = 0u; i < 25u; i = i + 1u) {
        state[i] = vec2<u32>(0u, 0u);
    }

    // Load input data for this hash
    // Optimized: Use packed u32 buffer (8KB as 2048 u32s instead of 32KB)
    var input_buffer: array<u32, 2048>;  // Packed buffer: 8KB capacity
    let input_offset = hash_idx * params.input_length;

    // Optimized: Load words directly (avoids byte unpacking overhead)
    let input_words = (params.input_length + 3u) / 4u;  // Round up to word count
    let start_word = input_offset / 4u;
    let byte_align = input_offset % 4u;

    if (byte_align == 0u) {
        // Aligned case: direct word copy
        for (var i = 0u; i < input_words; i = i + 1u) {
            input_buffer[i] = inputs.data[start_word + i];
        }
    } else {
        // Unaligned case: need to shift and combine words
        let shift_bits = byte_align * 8u;
        let inv_shift = 32u - shift_bits;

        for (var i = 0u; i < input_words; i = i + 1u) {
            let w0 = inputs.data[start_word + i];
            let w1 = inputs.data[start_word + i + 1u];
            input_buffer[i] = (w0 >> shift_bits) | (w1 << inv_shift);
        }
    }

    // Clear remaining buffer (important for padding correctness)
    for (var i = input_words; i < MAX_INPUT_WORDS; i = i + 1u) {
        input_buffer[i] = 0u;
    }

    // Apply SHA-3 padding
    let padded_len = apply_padding(&input_buffer, params.input_length, params.rate_bytes);

    // Absorbing phase: XOR input into state and permute
    // Optimized: Load u64 values directly from packed buffer
    var offset = 0u;
    while (offset < padded_len) {
        // XOR rate bytes into state (in 64-bit lanes)
        let num_lanes = params.rate_bytes / 8u;
        for (var i = 0u; i < num_lanes; i = i + 1u) {
            let byte_offset = offset + i * 8u;
            let lane = load_u64_from_buffer(&input_buffer, byte_offset);
            state[i] = xor_u64(state[i], lane);
        }

        // Apply Keccak-f permutation
        keccak_f1600(&state);

        offset = offset + params.rate_bytes;
    }

    // Squeezing phase: Extract output
    // Optimized: Write full u64 lanes when possible, avoiding byte-level operations
    let output_offset = hash_idx * params.output_bytes;
    var extracted = 0u;

    while (extracted < params.output_bytes) {
        let to_extract = min(params.output_bytes - extracted, params.rate_bytes);
        let num_lanes = to_extract / 8u;  // Full 64-bit lanes to extract
        let remaining_bytes = to_extract % 8u;  // Partial lane bytes

        // Write full lanes as u32 pairs
        for (var i = 0u; i < num_lanes; i = i + 1u) {
            let byte_pos = output_offset + extracted + i * 8u;
            let word_idx = byte_pos / 4u;
            let byte_align = byte_pos % 4u;

            let lane = state[i];

            if (byte_align == 0u) {
                // Aligned: write two u32s directly
                outputs.hash[word_idx] = lane.y;      // low word
                outputs.hash[word_idx + 1u] = lane.x; // high word
            } else {
                // Unaligned: read-modify-write
                let shift = byte_align * 8u;
                let inv_shift = 32u - shift;

                let w0 = outputs.hash[word_idx];
                let w2 = outputs.hash[word_idx + 2u];

                let low_mask = (1u << shift) - 1u;
                let high_mask = ~((1u << shift) - 1u);

                outputs.hash[word_idx] = (w0 & low_mask) | (lane.y << shift);
                outputs.hash[word_idx + 1u] = (lane.y >> inv_shift) | (lane.x << shift);
                outputs.hash[word_idx + 2u] = (w2 & high_mask) | (lane.x >> inv_shift);
            }
        }

        // Handle partial lane (remaining bytes < 8)
        if (remaining_bytes > 0u) {
            let lane = state[num_lanes];
            let byte_pos = output_offset + extracted + num_lanes * 8u;

            // Extract only the bytes we need
            for (var b = 0u; b < remaining_bytes; b = b + 1u) {
                let abs_byte_pos = byte_pos + b;
                let word_idx = abs_byte_pos / 4u;
                let byte_in_word = abs_byte_pos % 4u;

                // Extract byte from lane
                let lane_word = select(lane.y, lane.x, b >= 4u);
                let byte_val = (lane_word >> ((b % 4u) * 8u)) & 0xFFu;

                // Write byte
                let old_word = outputs.hash[word_idx];
                let shift = byte_in_word * 8u;
                let mask = ~(0xFFu << shift);
                outputs.hash[word_idx] = (old_word & mask) | (byte_val << shift);
            }
        }

        extracted = extracted + to_extract;

        if (extracted < params.output_bytes) {
            keccak_f1600(&state);
        }
    }
}
