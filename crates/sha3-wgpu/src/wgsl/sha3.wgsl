// WGSL compute shader for GPU-accelerated SHA-3 (Keccak-f[1600])
// Optimized for batch processing with proper memory alignment
// Note: WebGPU doesn't support u64, so we use vec2<u32> for 64-bit operations (high, low)

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

// Helper: Convert byte array to u64 (little-endian)
// Note: Using u32 instead of u8 since WGSL doesn't support u8
fn bytes_to_u64(bytes: ptr<function, array<u32, 8>>) -> vec2<u32> {
    var low: u32 = 0u;
    var high: u32 = 0u;

    // Low 32 bits: bytes 0-3
    low |= ((*bytes)[0] & 0xFFu) << 0u;
    low |= ((*bytes)[1] & 0xFFu) << 8u;
    low |= ((*bytes)[2] & 0xFFu) << 16u;
    low |= ((*bytes)[3] & 0xFFu) << 24u;

    // High 32 bits: bytes 4-7
    high |= ((*bytes)[4] & 0xFFu) << 0u;
    high |= ((*bytes)[5] & 0xFFu) << 8u;
    high |= ((*bytes)[6] & 0xFFu) << 16u;
    high |= ((*bytes)[7] & 0xFFu) << 24u;

    return vec2<u32>(high, low);
}

// Helper: Convert u64 to byte array (little-endian)
// Note: Using u32 instead of u8 since WGSL doesn't support u8
fn u64_to_bytes(value: vec2<u32>, bytes: ptr<function, array<u32, 8>>) {
    let high = value.x;
    let low = value.y;

    // Low 32 bits: bytes 0-3
    (*bytes)[0] = (low >> 0u) & 0xFFu;
    (*bytes)[1] = (low >> 8u) & 0xFFu;
    (*bytes)[2] = (low >> 16u) & 0xFFu;
    (*bytes)[3] = (low >> 24u) & 0xFFu;

    // High 32 bits: bytes 4-7
    (*bytes)[4] = (high >> 0u) & 0xFFu;
    (*bytes)[5] = (high >> 8u) & 0xFFu;
    (*bytes)[6] = (high >> 16u) & 0xFFu;
    (*bytes)[7] = (high >> 24u) & 0xFFu;
}

// Keccak-f[1600] permutation
// State is represented as 25 vec2<u32> values (5x5 array of 64-bit lanes)
fn keccak_f1600(state: ptr<function, array<vec2<u32>, 25>>) {
    var bc: array<vec2<u32>, 5>;  // Temporary array for theta step
    var t: vec2<u32>;

    for (var round = 0u; round < KECCAK_ROUNDS; round = round + 1u) {
        // θ (theta) step: XOR each column and rotate
        for (var i = 0u; i < 5u; i = i + 1u) {
            bc[i] = xor_u64(xor_u64(xor_u64(xor_u64((*state)[i], (*state)[i + 5u]), (*state)[i + 10u]), (*state)[i + 15u]), (*state)[i + 20u]);
        }

        for (var i = 0u; i < 5u; i = i + 1u) {
            t = xor_u64(bc[(i + 4u) % 5u], rotl_u64(bc[(i + 1u) % 5u], 1u));
            for (var j = 0u; j < 25u; j = j + 5u) {
                (*state)[j + i] = xor_u64((*state)[j + i], t);
            }
        }

        // ρ (rho) and π (pi) steps: Rotate and permute
        // Standard Keccak: for each lane at (x,y), rotate by rho_offset(x,y) and place at pi(x,y)
        // State is stored as state[x + 5*y] for position (x,y)
        // Pi maps (x,y) -> (y, 2x+3y mod 5)
        // We need to rotate by the offset of the ORIGINAL position, not the destination
        var temp_state: array<vec2<u32>, 25>;
        for (var i = 0u; i < 25u; i = i + 1u) {
            temp_state[i] = (*state)[i];
        }

        for (var i = 0u; i < 25u; i = i + 1u) {
            let j = get_pi_index(i);  // Destination position after pi permutation
            (*state)[j] = rotl_u64(temp_state[i], get_rho_offset(i));  // Rotate by original position's offset
        }

        // χ (chi) step: Non-linear mixing
        for (var j = 0u; j < 25u; j = j + 5u) {
            for (var i = 0u; i < 5u; i = i + 1u) {
                bc[i] = (*state)[j + i];
            }
            for (var i = 0u; i < 5u; i = i + 1u) {
                // NOT operation: ~x = XOR with all 1s
                let not_b1 = xor_u64(bc[(i + 1u) % 5u], vec2<u32>(0xFFFFFFFFu, 0xFFFFFFFFu));
                (*state)[j + i] = xor_u64((*state)[j + i], xor_u64(not_b1, bc[(i + 2u) % 5u]));
            }
        }

        // ι (iota) step: Add round constant
        (*state)[0] = xor_u64((*state)[0], get_rc(round));
    }
}

// SHA-3 padding (pad10*1)
// Note: Using u32 instead of u8 since WGSL doesn't support u8
fn apply_padding(
    input_data: ptr<function, array<u32, 16384>>,  // Max input size per hash (16KB)
    input_len: u32,
    rate_bytes: u32
) -> u32 {
    // SHA-3 uses domain separation byte 0x06
    (*input_data)[input_len] = 0x06u;

    // Calculate padded length (must be multiple of rate)
    var padded_len = input_len + 1u;
    let rate_blocks = (padded_len + rate_bytes - 1u) / rate_bytes;
    padded_len = rate_blocks * rate_bytes;

    // Clear padding bytes
    for (var i = input_len + 1u; i < padded_len; i = i + 1u) {
        (*input_data)[i] = 0u;
    }

    // Set final bit (pad10*1 pattern)
    (*input_data)[padded_len - 1u] |= 0x80u;

    return padded_len;
}

// Main compute shader - processes one hash per workgroup
@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let hash_idx = global_id.x;

    // Bounds check
    if (hash_idx >= params.num_hashes) {
        return;
    }

    // Initialize state (25 vec2<u32> values = 200 bytes)
    var state: array<vec2<u32>, 25>;
    for (var i = 0u; i < 25u; i = i + 1u) {
        state[i] = vec2<u32>(0u, 0u);
    }

    // Load input data for this hash
    // Note: Using u32 instead of u8 since WGSL doesn't support u8
    var input_buffer: array<u32, 16384>;  // Max 16KB per input
    let input_offset = hash_idx * params.input_length;

    for (var i = 0u; i < params.input_length; i = i + 1u) {
        // Load from u32 array (inputs are packed)
        let byte_idx = input_offset + i;
        let word_idx = byte_idx / 4u;
        let byte_in_word = byte_idx % 4u;
        input_buffer[i] = u32((inputs.data[word_idx] >> (byte_in_word * 8u)) & 0xFFu);
    }

    // Apply SHA-3 padding
    let padded_len = apply_padding(&input_buffer, params.input_length, params.rate_bytes);

    // Absorbing phase: XOR input into state and permute
    var offset = 0u;
    while (offset < padded_len) {
        // XOR rate bytes into state
        for (var i = 0u; i < params.rate_bytes / 8u; i = i + 1u) {
            var lane_bytes: array<u32, 8>;
            for (var j = 0u; j < 8u; j = j + 1u) {
                lane_bytes[j] = input_buffer[offset + i * 8u + j];
            }
            state[i] = xor_u64(state[i], bytes_to_u64(&lane_bytes));
        }

        // Apply Keccak-f permutation
        keccak_f1600(&state);

        offset = offset + params.rate_bytes;
    }

    // Squeezing phase: Extract output
    let output_offset = hash_idx * params.output_bytes;
    var extracted = 0u;

    while (extracted < params.output_bytes) {
        let to_extract = min(params.output_bytes - extracted, params.rate_bytes);
        let lanes_to_extract = (to_extract + 7u) / 8u;  // Round up to include partial lanes

        for (var i = 0u; i < lanes_to_extract; i = i + 1u) {
            var lane_bytes: array<u32, 8>;
            u64_to_bytes(state[i], &lane_bytes);

            // Extract bytes from this lane, but don't exceed to_extract
            let bytes_in_this_lane = min(8u, to_extract - i * 8u);
            for (var j = 0u; j < bytes_in_this_lane; j = j + 1u) {
                let byte_pos = output_offset + extracted + i * 8u + j;
                let word_idx = byte_pos / 4u;
                let byte_in_word = byte_pos % 4u;

                // Write to output buffer (pack into u32 array)
                let old_value = outputs.hash[word_idx];
                let mask = ~(0xFFu << (byte_in_word * 8u));
                let new_byte = (lane_bytes[j] & 0xFFu) << (byte_in_word * 8u);
                outputs.hash[word_idx] = (old_value & mask) | new_byte;
            }
        }

        extracted = extracted + to_extract;

        if (extracted < params.output_bytes) {
            keccak_f1600(&state);
        }
    }
}
