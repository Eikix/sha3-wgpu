// WGSL compute shader for GPU-accelerated SHA-3 (Keccak-f[1600])
// Optimized for batch processing with proper memory alignment

// Keccak round constants for iota step
// Note: WGSL doesn't support large u64 literals directly, so we store high/low u32 parts separately
// and combine them at runtime
const KECCAK_ROUNDS: u32 = 24u;

// Round constants stored as high and low u32 parts
const RC_HIGH: array<u32, 24> = array<u32, 24>(
    0x00000000u, 0x00000000u, 0x80000000u, 0x80000000u,
    0x00000000u, 0x00000000u, 0x80000000u, 0x80000000u,
    0x00000000u, 0x00000000u, 0x00000000u, 0x00000000u,
    0x00000000u, 0x80000000u, 0x80000000u, 0x80000000u,
    0x80000000u, 0x80000000u, 0x00000000u, 0x80000000u,
    0x80000000u, 0x80000000u, 0x00000000u, 0x80000000u
);

const RC_LOW: array<u32, 24> = array<u32, 24>(
    0x00000001u, 0x00008082u, 0x0000808Au, 0x80008000u,
    0x0000808Bu, 0x80000001u, 0x80008081u, 0x00008009u,
    0x0000008Au, 0x00000088u, 0x80008009u, 0x8000000Au,
    0x8000808Bu, 0x0000008Bu, 0x00008089u, 0x00008003u,
    0x00008002u, 0x00000080u, 0x0000800Au, 0x8000000Au,
    0x80008081u, 0x00008080u, 0x80000001u, 0x80008008u
);

// Helper function to get round constant by combining high and low parts
// Note: WGSL doesn't allow dynamic indexing of const arrays, so we use if-else chain
fn get_rc(round: u32) -> u64 {
    // Round constants stored as (high, low) u32 pairs
    var high: u32;
    var low: u32;
    
    if (round == 0u) { high = 0x00000000u; low = 0x00000001u; }
    else if (round == 1u) { high = 0x00000000u; low = 0x00008082u; }
    else if (round == 2u) { high = 0x80000000u; low = 0x0000808Au; }
    else if (round == 3u) { high = 0x80000000u; low = 0x80008000u; }
    else if (round == 4u) { high = 0x00000000u; low = 0x0000808Bu; }
    else if (round == 5u) { high = 0x00000000u; low = 0x80000001u; }
    else if (round == 6u) { high = 0x80000000u; low = 0x80008081u; }
    else if (round == 7u) { high = 0x80000000u; low = 0x00008009u; }
    else if (round == 8u) { high = 0x00000000u; low = 0x0000008Au; }
    else if (round == 9u) { high = 0x00000000u; low = 0x00000088u; }
    else if (round == 10u) { high = 0x00000000u; low = 0x80008009u; }
    else if (round == 11u) { high = 0x00000000u; low = 0x8000000Au; }
    else if (round == 12u) { high = 0x00000000u; low = 0x8000808Bu; }
    else if (round == 13u) { high = 0x80000000u; low = 0x0000008Bu; }
    else if (round == 14u) { high = 0x80000000u; low = 0x00008089u; }
    else if (round == 15u) { high = 0x80000000u; low = 0x00008003u; }
    else if (round == 16u) { high = 0x80000000u; low = 0x00008002u; }
    else if (round == 17u) { high = 0x80000000u; low = 0x00000080u; }
    else if (round == 18u) { high = 0x00000000u; low = 0x0000800Au; }
    else if (round == 19u) { high = 0x80000000u; low = 0x8000000Au; }
    else if (round == 20u) { high = 0x80000000u; low = 0x80008081u; }
    else if (round == 21u) { high = 0x80000000u; low = 0x00008080u; }
    else if (round == 22u) { high = 0x00000000u; low = 0x80000001u; }
    else { high = 0x80000000u; low = 0x80008008u; } // round == 23u
    
    return (u64(high) << 32u) | u64(low);
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

fn get_rho_offset(j: u32) -> u32 {
    if (j == 0u) { return 0u; }
    else if (j == 1u) { return 1u; }
    else if (j == 2u) { return 62u; }
    else if (j == 3u) { return 28u; }
    else if (j == 4u) { return 27u; }
    else if (j == 5u) { return 36u; }
    else if (j == 6u) { return 44u; }
    else if (j == 7u) { return 6u; }
    else if (j == 8u) { return 55u; }
    else if (j == 9u) { return 20u; }
    else if (j == 10u) { return 3u; }
    else if (j == 11u) { return 10u; }
    else if (j == 12u) { return 43u; }
    else if (j == 13u) { return 25u; }
    else if (j == 14u) { return 39u; }
    else if (j == 15u) { return 41u; }
    else if (j == 16u) { return 45u; }
    else if (j == 17u) { return 15u; }
    else if (j == 18u) { return 21u; }
    else if (j == 19u) { return 8u; }
    else if (j == 20u) { return 18u; }
    else if (j == 21u) { return 2u; }
    else if (j == 22u) { return 61u; }
    else if (j == 23u) { return 56u; }
    else { return 14u; } // j == 24u
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

// Helper: Rotate left for 64-bit values (WGSL doesn't have native u64 rotl)
fn rotl64(x: u64, n: u32) -> u64 {
    return (x << n) | (x >> (64u - n));
}

// Helper: Convert byte array to u64 (little-endian)
// Note: Using u32 instead of u8 since WGSL doesn't support u8
fn bytes_to_u64(bytes: ptr<function, array<u32, 8>>) -> u64 {
    var result: u64 = u64(0u);
    for (var i = 0u; i < 8u; i = i + 1u) {
        result |= u64((*bytes)[i] & 0xFFu) << (i * 8u);
    }
    return result;
}

// Helper: Convert u64 to byte array (little-endian)
// Note: Using u32 instead of u8 since WGSL doesn't support u8
fn u64_to_bytes(value: u64, bytes: ptr<function, array<u32, 8>>) {
    for (var i = 0u; i < 8u; i = i + 1u) {
        (*bytes)[i] = u32((value >> (i * 8u)) & u64(0xFFu));
    }
}

// Keccak-f[1600] permutation
// State is represented as 25 u64 values (5x5 array of 64-bit lanes)
fn keccak_f1600(state: ptr<function, array<u64, 25>>) {
    var bc: array<u64, 5>;  // Temporary array for theta step
    var t: u64;

    for (var round = 0u; round < KECCAK_ROUNDS; round = round + 1u) {
        // θ (theta) step: XOR each column and rotate
        for (var i = 0u; i < 5u; i = i + 1u) {
            bc[i] = (*state)[i] ^ (*state)[i + 5u] ^ (*state)[i + 10u] ^ (*state)[i + 15u] ^ (*state)[i + 20u];
        }

        for (var i = 0u; i < 5u; i = i + 1u) {
            t = bc[(i + 4u) % 5u] ^ rotl64(bc[(i + 1u) % 5u], 1u);
            for (var j = 0u; j < 25u; j = j + 5u) {
                (*state)[j + i] ^= t;
            }
        }

        // ρ (rho) and π (pi) steps: Rotate and permute
        // Standard Keccak: for each lane at (x,y), rotate by rho_offset(x,y) and place at pi(x,y)
        // State is stored as state[x + 5*y] for position (x,y)
        // Pi maps (x,y) -> (y, 2x+3y mod 5)
        // We need to rotate by the offset of the ORIGINAL position, not the destination
        var temp_state: array<u64, 25>;
        for (var i = 0u; i < 25u; i = i + 1u) {
            temp_state[i] = (*state)[i];
        }
        
        for (var i = 0u; i < 25u; i = i + 1u) {
            let j = get_pi_index(i);  // Destination position after pi permutation
            (*state)[j] = rotl64(temp_state[i], get_rho_offset(i));  // Rotate by original position's offset
        }

        // χ (chi) step: Non-linear mixing
        for (var j = 0u; j < 25u; j = j + 5u) {
            for (var i = 0u; i < 5u; i = i + 1u) {
                bc[i] = (*state)[j + i];
            }
            for (var i = 0u; i < 5u; i = i + 1u) {
                (*state)[j + i] ^= (~bc[(i + 1u) % 5u]) & bc[(i + 2u) % 5u];
            }
        }

        // ι (iota) step: Add round constant
        (*state)[0] ^= get_rc(round);
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

    // Initialize state (25 u64 values = 200 bytes)
    var state: array<u64, 25>;
    for (var i = 0u; i < 25u; i = i + 1u) {
        state[i] = u64(0u);
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
            state[i] ^= bytes_to_u64(&lane_bytes);
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
