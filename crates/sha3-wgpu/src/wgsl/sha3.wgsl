// WGSL compute shader for GPU-accelerated SHA-3 (Keccak-f[1600])
// Optimized for batch processing with proper memory alignment

// Keccak round constants for iota step
const KECCAK_ROUNDS: u32 = 24u;
const RC: array<u64, 24> = array<u64, 24>(
    0x0000000000000001u, 0x0000000000008082u, 0x800000000000808Au, 0x8000000080008000u,
    0x000000000000808Bu, 0x0000000080000001u, 0x8000000080008081u, 0x8000000000008009u,
    0x000000000000008Au, 0x0000000000000088u, 0x0000000080008009u, 0x000000008000000Au,
    0x000000008000808Bu, 0x800000000000008Bu, 0x8000000000008089u, 0x8000000000008003u,
    0x8000000000008002u, 0x8000000000000080u, 0x000000000000800Au, 0x800000008000000Au,
    0x8000000080008081u, 0x8000000000008080u, 0x0000000080000001u, 0x8000000080008008u
);

// Rotation offsets for rho step
const RHO_OFFSETS: array<u32, 25> = array<u32, 25>(
     0u,  1u, 62u, 28u, 27u,
    36u, 44u,  6u, 55u, 20u,
     3u, 10u, 43u, 25u, 39u,
    41u, 45u, 15u, 21u,  8u,
    18u,  2u, 61u, 56u, 14u
);

// Pi step permutation indices
const PI_INDICES: array<u32, 25> = array<u32, 25>(
     0u,  6u, 12u, 18u, 24u,
     3u,  9u, 15u, 21u,  2u,
     1u,  7u, 13u, 19u, 20u,
     4u,  5u, 11u, 17u, 23u,
     2u,  8u, 14u, 15u, 16u
);

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
fn bytes_to_u64(bytes: ptr<function, array<u8, 8>>) -> u64 {
    var result: u64 = 0u;
    for (var i = 0u; i < 8u; i = i + 1u) {
        result |= u64((*bytes)[i]) << (i * 8u);
    }
    return result;
}

// Helper: Convert u64 to byte array (little-endian)
fn u64_to_bytes(value: u64, bytes: ptr<function, array<u8, 8>>) {
    for (var i = 0u; i < 8u; i = i + 1u) {
        (*bytes)[i] = u8((value >> (i * 8u)) & 0xFFu);
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
        t = (*state)[1];
        for (var i = 0u; i < 24u; i = i + 1u) {
            let j = PI_INDICES[i];
            bc[0] = (*state)[j];
            (*state)[j] = rotl64(t, RHO_OFFSETS[j]);
            t = bc[0];
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
        (*state)[0] ^= RC[round];
    }
}

// SHA-3 padding (pad10*1)
fn apply_padding(
    input_data: ptr<function, array<u8, 2048>>,  // Max input size per hash
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
        state[i] = 0u;
    }

    // Load input data for this hash
    var input_buffer: array<u8, 2048>;  // Max 2KB per input
    let input_offset = hash_idx * params.input_length;

    for (var i = 0u; i < params.input_length; i = i + 1u) {
        // Load from u32 array (inputs are packed)
        let byte_idx = input_offset + i;
        let word_idx = byte_idx / 4u;
        let byte_in_word = byte_idx % 4u;
        input_buffer[i] = u8((inputs.data[word_idx] >> (byte_in_word * 8u)) & 0xFFu);
    }

    // Apply SHA-3 padding
    let padded_len = apply_padding(&input_buffer, params.input_length, params.rate_bytes);

    // Absorbing phase: XOR input into state and permute
    var offset = 0u;
    while (offset < padded_len) {
        // XOR rate bytes into state
        for (var i = 0u; i < params.rate_bytes / 8u; i = i + 1u) {
            var lane_bytes: array<u8, 8>;
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

        for (var i = 0u; i < to_extract / 8u; i = i + 1u) {
            var lane_bytes: array<u8, 8>;
            u64_to_bytes(state[i], &lane_bytes);

            for (var j = 0u; j < 8u; j = j + 1u) {
                let byte_pos = output_offset + extracted + i * 8u + j;
                let word_idx = byte_pos / 4u;
                let byte_in_word = byte_pos % 4u;

                // Write to output buffer (pack into u32 array)
                let old_value = outputs.hash[word_idx];
                let mask = ~(0xFFu << (byte_in_word * 8u));
                let new_byte = u32(lane_bytes[j]) << (byte_in_word * 8u);
                outputs.hash[word_idx] = (old_value & mask) | new_byte;
            }
        }

        extracted = extracted + to_extract;

        if (extracted < params.output_bytes) {
            keccak_f1600(&state);
        }
    }
}
