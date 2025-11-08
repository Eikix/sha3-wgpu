//! WASM tests using wasm-bindgen-test
//! Comprehensive test suite for sha3-wasm JavaScript/WASM bindings

use js_sys::{Array, Uint8Array};
use sha3_wasm::{sha3, sha3_batch, Sha3WasmHasher};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert a Rust byte slice to JavaScript Uint8Array
fn to_uint8_array(data: &[u8]) -> Uint8Array {
    Uint8Array::from(data)
}

/// Convert JavaScript Uint8Array to Rust Vec<u8>
fn from_uint8_array(arr: &Uint8Array) -> Vec<u8> {
    arr.to_vec()
}

/// Create a JavaScript Array of Uint8Array from Rust byte slices
fn to_js_array(inputs: &[&[u8]]) -> Array {
    let array = Array::new();
    for input in inputs {
        array.push(&to_uint8_array(input));
    }
    array
}

/// Convert bytes to hex string
fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// ============================================================================
// Type Conversion Tests
// ============================================================================

#[wasm_bindgen_test]
async fn test_type_conversion_uint8array_to_vec() {
    // Test that Uint8Array converts correctly to Rust Vec<u8>
    let test_data = b"hello world";
    let js_array = to_uint8_array(test_data);
    let rust_vec = from_uint8_array(&js_array);

    assert_eq!(rust_vec, test_data);
}

#[wasm_bindgen_test]
async fn test_type_conversion_vec_to_uint8array() {
    // Test that Rust Vec<u8> converts correctly back to Uint8Array
    let test_data = vec![1u8, 2, 3, 4, 5];
    let js_array = to_uint8_array(&test_data);
    let converted_back = from_uint8_array(&js_array);

    assert_eq!(converted_back, test_data);
}

#[wasm_bindgen_test]
async fn test_type_conversion_array_of_uint8arrays() {
    // Test that JavaScript Array of Uint8Array converts correctly
    let inputs = [b"hello".as_slice(), b"world".as_slice(), b"test".as_slice()];
    let js_array = to_js_array(&inputs);

    assert_eq!(js_array.length(), 3);

    // Verify each element
    for (i, expected) in inputs.iter().enumerate() {
        let elem = Uint8Array::from(js_array.get(i as u32));
        assert_eq!(from_uint8_array(&elem), *expected);
    }
}

#[wasm_bindgen_test]
async fn test_type_conversion_empty_uint8array() {
    // Test empty Uint8Array handling
    let empty: &[u8] = b"";
    let js_array = to_uint8_array(empty);
    let rust_vec = from_uint8_array(&js_array);

    assert_eq!(rust_vec.len(), 0);
}

#[wasm_bindgen_test]
async fn test_type_conversion_large_uint8array() {
    // Test large array conversion
    let large_data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
    let js_array = to_uint8_array(&large_data);
    let converted_back = from_uint8_array(&js_array);

    assert_eq!(converted_back, large_data);
}

// ============================================================================
// Sha3WasmHasher Construction Tests
// ============================================================================

#[wasm_bindgen_test]
async fn test_hasher_new_sha3_224() {
    let hasher = Sha3WasmHasher::new("sha3-224").await;
    assert!(hasher.is_ok());
    let hasher = hasher.unwrap();
    assert_eq!(hasher.get_variant(), "sha3-224");
    assert_eq!(hasher.get_output_size(), 28);
}

#[wasm_bindgen_test]
async fn test_hasher_new_sha3_256() {
    let hasher = Sha3WasmHasher::new("sha3-256").await;
    assert!(hasher.is_ok());
    let hasher = hasher.unwrap();
    assert_eq!(hasher.get_variant(), "sha3-256");
    assert_eq!(hasher.get_output_size(), 32);
}

#[wasm_bindgen_test]
async fn test_hasher_new_sha3_384() {
    let hasher = Sha3WasmHasher::new("sha3-384").await;
    assert!(hasher.is_ok());
    let hasher = hasher.unwrap();
    assert_eq!(hasher.get_variant(), "sha3-384");
    assert_eq!(hasher.get_output_size(), 48);
}

#[wasm_bindgen_test]
async fn test_hasher_new_sha3_512() {
    let hasher = Sha3WasmHasher::new("sha3-512").await;
    assert!(hasher.is_ok());
    let hasher = hasher.unwrap();
    assert_eq!(hasher.get_variant(), "sha3-512");
    assert_eq!(hasher.get_output_size(), 64);
}

#[wasm_bindgen_test]
async fn test_hasher_new_shake128() {
    let hasher = Sha3WasmHasher::new("shake128").await;
    assert!(hasher.is_ok());
    let hasher = hasher.unwrap();
    assert_eq!(hasher.get_variant(), "shake128");
    // SHAKE variants have 0 as default output size (variable length)
    assert_eq!(hasher.get_output_size(), 0);
}

#[wasm_bindgen_test]
async fn test_hasher_new_shake256() {
    let hasher = Sha3WasmHasher::new("shake256").await;
    assert!(hasher.is_ok());
    let hasher = hasher.unwrap();
    assert_eq!(hasher.get_variant(), "shake256");
    assert_eq!(hasher.get_output_size(), 0);
}

#[wasm_bindgen_test]
async fn test_hasher_new_variant_with_underscore() {
    // Test alternative variant naming (with underscore)
    let hasher = Sha3WasmHasher::new("sha3_256").await;
    assert!(hasher.is_ok());
    let hasher = hasher.unwrap();
    assert_eq!(hasher.get_variant(), "sha3-256");
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[wasm_bindgen_test]
async fn test_hasher_new_invalid_variant() {
    let result = Sha3WasmHasher::new("invalid-variant").await;
    assert!(result.is_err());
}

#[wasm_bindgen_test]
async fn test_hasher_new_empty_variant() {
    let result = Sha3WasmHasher::new("").await;
    assert!(result.is_err());
}

#[wasm_bindgen_test]
async fn test_hasher_new_case_insensitive() {
    // Test that variant names are case-insensitive
    let hasher1 = Sha3WasmHasher::new("SHA3-256").await;
    assert!(hasher1.is_ok());

    let hasher2 = Sha3WasmHasher::new("Sha3-256").await;
    assert!(hasher2.is_ok());

    let hasher3 = Sha3WasmHasher::new("SHAKE128").await;
    assert!(hasher3.is_ok());
}

#[wasm_bindgen_test]
async fn test_hash_batch_empty_array() {
    let hasher = Sha3WasmHasher::new("sha3-256").await.unwrap();
    let empty_array = Array::new();
    let result = hasher.hash_batch(&empty_array).await;

    assert!(result.is_ok());
    let hashes = result.unwrap();
    assert_eq!(hashes.length(), 0);
}

#[wasm_bindgen_test]
async fn test_hash_batch_with_length_empty_array() {
    let hasher = Sha3WasmHasher::new("shake128").await.unwrap();
    let empty_array = Array::new();
    let result = hasher.hash_batch_with_length(&empty_array, 32).await;

    assert!(result.is_ok());
    let hashes = result.unwrap();
    assert_eq!(hashes.length(), 0);
}

// ============================================================================
// hashSingle Tests
// ============================================================================

#[wasm_bindgen_test]
async fn test_hash_single_empty_input() {
    let hasher = Sha3WasmHasher::new("sha3-256").await.unwrap();
    let input = to_uint8_array(b"");
    let result = hasher.hash_single(&input).await;

    assert!(result.is_ok());
    let hash = result.unwrap();
    assert_eq!(hash.length(), 32); // SHA3-256 outputs 32 bytes
}

#[wasm_bindgen_test]
async fn test_hash_single_small_input() {
    let hasher = Sha3WasmHasher::new("sha3-256").await.unwrap();
    let input = to_uint8_array(b"hello");
    let result = hasher.hash_single(&input).await;

    assert!(result.is_ok());
    let hash = result.unwrap();
    assert_eq!(hash.length(), 32);
}

#[wasm_bindgen_test]
async fn test_hash_single_large_input() {
    let hasher = Sha3WasmHasher::new("sha3-256").await.unwrap();
    let large_input: Vec<u8> = vec![b'a'; 10000];
    let input = to_uint8_array(&large_input);
    let result = hasher.hash_single(&input).await;

    assert!(result.is_ok());
    let hash = result.unwrap();
    assert_eq!(hash.length(), 32);
}

#[wasm_bindgen_test]
async fn test_hash_single_different_variants() {
    let test_input = b"test input";

    // SHA3-224
    let hasher = Sha3WasmHasher::new("sha3-224").await.unwrap();
    let result = hasher.hash_single(&to_uint8_array(test_input)).await.unwrap();
    assert_eq!(result.length(), 28);

    // SHA3-256
    let hasher = Sha3WasmHasher::new("sha3-256").await.unwrap();
    let result = hasher.hash_single(&to_uint8_array(test_input)).await.unwrap();
    assert_eq!(result.length(), 32);

    // SHA3-384
    let hasher = Sha3WasmHasher::new("sha3-384").await.unwrap();
    let result = hasher.hash_single(&to_uint8_array(test_input)).await.unwrap();
    assert_eq!(result.length(), 48);

    // SHA3-512
    let hasher = Sha3WasmHasher::new("sha3-512").await.unwrap();
    let result = hasher.hash_single(&to_uint8_array(test_input)).await.unwrap();
    assert_eq!(result.length(), 64);
}

// ============================================================================
// hashBatch Tests
// ============================================================================

#[wasm_bindgen_test]
async fn test_hash_batch_single_item() {
    let hasher = Sha3WasmHasher::new("sha3-256").await.unwrap();
    let inputs = to_js_array(&[b"hello"]);
    let result = hasher.hash_batch(&inputs).await;

    assert!(result.is_ok());
    let hashes = result.unwrap();
    assert_eq!(hashes.length(), 1);

    let hash = Uint8Array::from(hashes.get(0));
    assert_eq!(hash.length(), 32);
}

#[wasm_bindgen_test]
async fn test_hash_batch_multiple_items() {
    let hasher = Sha3WasmHasher::new("sha3-256").await.unwrap();
    let inputs = to_js_array(&[b"hello", b"world", b"batch"]);
    let result = hasher.hash_batch(&inputs).await;

    assert!(result.is_ok());
    let hashes = result.unwrap();
    assert_eq!(hashes.length(), 3);

    // Verify all hashes have correct length
    for i in 0..3 {
        let hash = Uint8Array::from(hashes.get(i));
        assert_eq!(hash.length(), 32);
    }
}

#[wasm_bindgen_test]
async fn test_hash_batch_with_empty_inputs() {
    let hasher = Sha3WasmHasher::new("sha3-256").await.unwrap();
    let inputs = to_js_array(&[b"", b"", b""]);
    let result = hasher.hash_batch(&inputs).await;

    assert!(result.is_ok());
    let hashes = result.unwrap();
    assert_eq!(hashes.length(), 3);
}

#[wasm_bindgen_test]
async fn test_hash_batch_consistency() {
    // Verify that hashing the same input multiple times produces the same hash
    let hasher = Sha3WasmHasher::new("sha3-256").await.unwrap();
    let inputs = to_js_array(&[b"test", b"test", b"test"]);
    let result = hasher.hash_batch(&inputs).await.unwrap();

    let hash0 = from_uint8_array(&Uint8Array::from(result.get(0)));
    let hash1 = from_uint8_array(&Uint8Array::from(result.get(1)));
    let hash2 = from_uint8_array(&Uint8Array::from(result.get(2)));

    assert_eq!(hash0, hash1);
    assert_eq!(hash1, hash2);
}

#[wasm_bindgen_test]
async fn test_hash_batch_vs_single() {
    // Verify that batch hashing produces same results as individual hashes
    let hasher = Sha3WasmHasher::new("sha3-256").await.unwrap();
    let test_inputs = [b"hello".as_slice(), b"world".as_slice()];

    // Hash individually
    let hash1_single = hasher.hash_single(&to_uint8_array(test_inputs[0])).await.unwrap();
    let hash2_single = hasher.hash_single(&to_uint8_array(test_inputs[1])).await.unwrap();

    // Hash as batch
    let inputs = to_js_array(&test_inputs);
    let batch_result = hasher.hash_batch(&inputs).await.unwrap();

    let hash1_batch = Uint8Array::from(batch_result.get(0));
    let hash2_batch = Uint8Array::from(batch_result.get(1));

    assert_eq!(from_uint8_array(&hash1_single), from_uint8_array(&hash1_batch));
    assert_eq!(from_uint8_array(&hash2_single), from_uint8_array(&hash2_batch));
}

#[wasm_bindgen_test]
async fn test_hash_batch_large_batch() {
    let hasher = Sha3WasmHasher::new("sha3-256").await.unwrap();

    // Create 50 inputs
    let inputs_vec: Vec<Vec<u8>> = (0..50)
        .map(|i| format!("test input number {:03}", i).into_bytes())
        .collect();
    let input_refs: Vec<&[u8]> = inputs_vec.iter().map(|v| v.as_slice()).collect();
    let inputs = to_js_array(&input_refs);

    let result = hasher.hash_batch(&inputs).await;
    assert!(result.is_ok());

    let hashes = result.unwrap();
    assert_eq!(hashes.length(), 50);

    // Verify all hashes have correct length
    for i in 0..50 {
        let hash = Uint8Array::from(hashes.get(i));
        assert_eq!(hash.length(), 32);
    }
}

// ============================================================================
// hashBatchWithLength Tests (SHAKE variants)
// ============================================================================

#[wasm_bindgen_test]
async fn test_hash_batch_with_length_shake128() {
    let hasher = Sha3WasmHasher::new("shake128").await.unwrap();
    let inputs = to_js_array(&[b"test1", b"test2", b"test3"]);

    // Request 32 bytes output
    let result = hasher.hash_batch_with_length(&inputs, 32).await;
    assert!(result.is_ok());

    let hashes = result.unwrap();
    assert_eq!(hashes.length(), 3);

    for i in 0..3 {
        let hash = Uint8Array::from(hashes.get(i));
        assert_eq!(hash.length(), 32);
    }
}

#[wasm_bindgen_test]
async fn test_hash_batch_with_length_shake256() {
    let hasher = Sha3WasmHasher::new("shake256").await.unwrap();
    let inputs = to_js_array(&[b"test1", b"test2"]);

    // Request 64 bytes output
    let result = hasher.hash_batch_with_length(&inputs, 64).await;
    assert!(result.is_ok());

    let hashes = result.unwrap();
    assert_eq!(hashes.length(), 2);

    for i in 0..2 {
        let hash = Uint8Array::from(hashes.get(i));
        assert_eq!(hash.length(), 64);
    }
}

#[wasm_bindgen_test]
async fn test_hash_batch_with_length_custom_sizes() {
    let hasher = Sha3WasmHasher::new("shake128").await.unwrap();
    let inputs = to_js_array(&[b"test"]);

    // Test various output sizes
    for output_size in [16, 32, 64, 128] {
        let result = hasher.hash_batch_with_length(&inputs, output_size).await.unwrap();
        let hash = Uint8Array::from(result.get(0));
        assert_eq!(hash.length() as usize, output_size);
    }
}

// ============================================================================
// Standalone Function Tests
// ============================================================================

#[wasm_bindgen_test]
async fn test_sha3_function() {
    let input = to_uint8_array(b"hello world");
    let result = sha3("sha3-256", &input).await;

    assert!(result.is_ok());
    let hash = result.unwrap();
    assert_eq!(hash.length(), 32);
}

#[wasm_bindgen_test]
async fn test_sha3_function_all_variants() {
    let input = to_uint8_array(b"test");

    let hash224 = sha3("sha3-224", &input).await.unwrap();
    assert_eq!(hash224.length(), 28);

    let hash256 = sha3("sha3-256", &input).await.unwrap();
    assert_eq!(hash256.length(), 32);

    let hash384 = sha3("sha3-384", &input).await.unwrap();
    assert_eq!(hash384.length(), 48);

    let hash512 = sha3("sha3-512", &input).await.unwrap();
    assert_eq!(hash512.length(), 64);
}

#[wasm_bindgen_test]
async fn test_sha3_function_invalid_variant() {
    let input = to_uint8_array(b"test");
    let result = sha3("invalid", &input).await;
    assert!(result.is_err());
}

#[wasm_bindgen_test]
async fn test_sha3_batch_function() {
    let inputs = to_js_array(&[b"hello", b"world"]);
    let result = sha3_batch("sha3-256", &inputs).await;

    assert!(result.is_ok());
    let hashes = result.unwrap();
    assert_eq!(hashes.length(), 2);

    for i in 0..2 {
        let hash = Uint8Array::from(hashes.get(i));
        assert_eq!(hash.length(), 32);
    }
}

#[wasm_bindgen_test]
async fn test_sha3_batch_function_empty_array() {
    let empty = Array::new();
    let result = sha3_batch("sha3-256", &empty).await;

    assert!(result.is_ok());
    let hashes = result.unwrap();
    assert_eq!(hashes.length(), 0);
}

#[wasm_bindgen_test]
async fn test_sha3_batch_function_invalid_variant() {
    let inputs = to_js_array(&[b"test"]);
    let result = sha3_batch("invalid", &inputs).await;
    assert!(result.is_err());
}

// ============================================================================
// Correctness Tests with Known Test Vectors
// ============================================================================

#[wasm_bindgen_test]
async fn test_sha3_256_empty_correctness() {
    // Known SHA3-256 hash of empty string
    // echo -n "" | openssl dgst -sha3-256
    let hasher = Sha3WasmHasher::new("sha3-256").await.unwrap();
    let input = to_uint8_array(b"");
    let result = hasher.hash_single(&input).await.unwrap();
    let hash_hex = to_hex(&from_uint8_array(&result));

    assert_eq!(
        hash_hex,
        "a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a"
    );
}

#[wasm_bindgen_test]
async fn test_sha3_256_abc_correctness() {
    // Known SHA3-256 hash of "abc"
    let hasher = Sha3WasmHasher::new("sha3-256").await.unwrap();
    let input = to_uint8_array(b"abc");
    let result = hasher.hash_single(&input).await.unwrap();
    let hash_hex = to_hex(&from_uint8_array(&result));

    assert_eq!(
        hash_hex,
        "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532"
    );
}

#[wasm_bindgen_test]
async fn test_sha3_224_abc_correctness() {
    // Known SHA3-224 hash of "abc"
    let hasher = Sha3WasmHasher::new("sha3-224").await.unwrap();
    let input = to_uint8_array(b"abc");
    let result = hasher.hash_single(&input).await.unwrap();
    let hash_hex = to_hex(&from_uint8_array(&result));

    assert_eq!(
        hash_hex,
        "e642824c3f8cf24ad09234ee7d3c766fc9a3a5168d0c94ad73b46fdf"
    );
}

#[wasm_bindgen_test]
async fn test_sha3_384_abc_correctness() {
    // Known SHA3-384 hash of "abc"
    let hasher = Sha3WasmHasher::new("sha3-384").await.unwrap();
    let input = to_uint8_array(b"abc");
    let result = hasher.hash_single(&input).await.unwrap();
    let hash_hex = to_hex(&from_uint8_array(&result));

    assert_eq!(
        hash_hex,
        "ec01498288516fc926459f58e2c6ad8df9b473cb0fc08c2596da7cf0e49be4b298d88cea927ac7f539f1edf228376d25"
    );
}

#[wasm_bindgen_test]
async fn test_sha3_512_abc_correctness() {
    // Known SHA3-512 hash of "abc"
    let hasher = Sha3WasmHasher::new("sha3-512").await.unwrap();
    let input = to_uint8_array(b"abc");
    let result = hasher.hash_single(&input).await.unwrap();
    let hash_hex = to_hex(&from_uint8_array(&result));

    assert_eq!(
        hash_hex,
        "b751850b1a57168a5693cd924b6b096e08f621827444f70d884f5d0240d2712e10e116e9192af3c91a7ec57647e3934057340b4cf408d5a56592f8274eec53f0"
    );
}

#[wasm_bindgen_test]
async fn test_sha3_256_long_message_correctness() {
    // Known SHA3-256 hash of "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
    let hasher = Sha3WasmHasher::new("sha3-256").await.unwrap();
    let input = to_uint8_array(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq");
    let result = hasher.hash_single(&input).await.unwrap();
    let hash_hex = to_hex(&from_uint8_array(&result));

    assert_eq!(
        hash_hex,
        "41c0dba2a9d6240849100376a8235e2c82e1b9998a999e21db32dd97496d3376"
    );
}

#[wasm_bindgen_test]
async fn test_batch_correctness_multiple_known_vectors() {
    // Test batch hashing with multiple known vectors
    let hasher = Sha3WasmHasher::new("sha3-256").await.unwrap();
    let inputs = to_js_array(&[b"", b"abc"]);
    let result = hasher.hash_batch(&inputs).await.unwrap();

    let hash0_hex = to_hex(&from_uint8_array(&Uint8Array::from(result.get(0))));
    let hash1_hex = to_hex(&from_uint8_array(&Uint8Array::from(result.get(1))));

    assert_eq!(
        hash0_hex,
        "a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a"
    );
    assert_eq!(
        hash1_hex,
        "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532"
    );
}

#[wasm_bindgen_test]
async fn test_standalone_sha3_function_correctness() {
    // Test standalone function with known vector
    let input = to_uint8_array(b"abc");
    let result = sha3("sha3-256", &input).await.unwrap();
    let hash_hex = to_hex(&from_uint8_array(&result));

    assert_eq!(
        hash_hex,
        "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532"
    );
}

#[wasm_bindgen_test]
async fn test_standalone_sha3_batch_correctness() {
    // Test standalone batch function with known vectors
    let inputs = to_js_array(&[b"", b"abc"]);
    let result = sha3_batch("sha3-256", &inputs).await.unwrap();

    let hash0_hex = to_hex(&from_uint8_array(&Uint8Array::from(result.get(0))));
    let hash1_hex = to_hex(&from_uint8_array(&Uint8Array::from(result.get(1))));

    assert_eq!(
        hash0_hex,
        "a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a"
    );
    assert_eq!(
        hash1_hex,
        "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532"
    );
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[wasm_bindgen_test]
async fn test_edge_case_all_zeros() {
    let hasher = Sha3WasmHasher::new("sha3-256").await.unwrap();
    let input = to_uint8_array(&vec![0u8; 100]);
    let result = hasher.hash_single(&input).await;

    assert!(result.is_ok());
    let hash = result.unwrap();
    assert_eq!(hash.length(), 32);
}

#[wasm_bindgen_test]
async fn test_edge_case_all_ones() {
    let hasher = Sha3WasmHasher::new("sha3-256").await.unwrap();
    let input = to_uint8_array(&vec![0xffu8; 100]);
    let result = hasher.hash_single(&input).await;

    assert!(result.is_ok());
    let hash = result.unwrap();
    assert_eq!(hash.length(), 32);
}

#[wasm_bindgen_test]
async fn test_edge_case_single_byte() {
    let hasher = Sha3WasmHasher::new("sha3-256").await.unwrap();
    let input = to_uint8_array(&[0x42]);
    let result = hasher.hash_single(&input).await;

    assert!(result.is_ok());
    let hash = result.unwrap();
    assert_eq!(hash.length(), 32);
}

#[wasm_bindgen_test]
async fn test_edge_case_boundary_136_bytes() {
    // SHA3-256 has a rate of 136 bytes (1088 bits)
    // Test at exactly the boundary
    let hasher = Sha3WasmHasher::new("sha3-256").await.unwrap();
    let input = to_uint8_array(&vec![b'a'; 136]);
    let result = hasher.hash_single(&input).await;

    assert!(result.is_ok());
    let hash = result.unwrap();
    assert_eq!(hash.length(), 32);
}

#[wasm_bindgen_test]
async fn test_edge_case_just_over_boundary() {
    // Test just over the rate boundary
    let hasher = Sha3WasmHasher::new("sha3-256").await.unwrap();
    let input = to_uint8_array(&vec![b'a'; 137]);
    let result = hasher.hash_single(&input).await;

    assert!(result.is_ok());
    let hash = result.unwrap();
    assert_eq!(hash.length(), 32);
}

#[wasm_bindgen_test]
async fn test_edge_case_very_large_input() {
    // Test with 1MB input
    let hasher = Sha3WasmHasher::new("sha3-256").await.unwrap();
    let input = to_uint8_array(&vec![b'x'; 1024 * 1024]);
    let result = hasher.hash_single(&input).await;

    assert!(result.is_ok());
    let hash = result.unwrap();
    assert_eq!(hash.length(), 32);
}

#[wasm_bindgen_test]
async fn test_edge_case_different_output_sizes() {
    // Verify all variants produce correct output sizes
    let test_input = to_uint8_array(b"test");

    let variants_and_sizes = [
        ("sha3-224", 28),
        ("sha3-256", 32),
        ("sha3-384", 48),
        ("sha3-512", 64),
    ];

    for (variant, expected_size) in variants_and_sizes.iter() {
        let hash = sha3(variant, &test_input).await.unwrap();
        assert_eq!(
            hash.length() as usize,
            *expected_size,
            "Variant {} should produce {} bytes",
            variant,
            expected_size
        );
    }
}

#[wasm_bindgen_test]
async fn test_edge_case_batch_with_varying_same_length() {
    // All inputs must be same length for batch processing
    let hasher = Sha3WasmHasher::new("sha3-256").await.unwrap();
    let inputs = to_js_array(&[b"aaaa", b"bbbb", b"cccc"]);
    let result = hasher.hash_batch(&inputs).await;

    assert!(result.is_ok());
    let hashes = result.unwrap();
    assert_eq!(hashes.length(), 3);
}

#[wasm_bindgen_test]
async fn test_reuse_hasher_multiple_times() {
    // Verify hasher can be reused multiple times
    let hasher = Sha3WasmHasher::new("sha3-256").await.unwrap();

    let input1 = to_uint8_array(b"first");
    let hash1 = hasher.hash_single(&input1).await.unwrap();

    let input2 = to_uint8_array(b"second");
    let hash2 = hasher.hash_single(&input2).await.unwrap();

    // Hashes should be different
    assert_ne!(
        from_uint8_array(&hash1),
        from_uint8_array(&hash2)
    );

    // Hashing same input again should produce same hash
    let hash1_again = hasher.hash_single(&input1).await.unwrap();
    assert_eq!(
        from_uint8_array(&hash1),
        from_uint8_array(&hash1_again)
    );
}

#[wasm_bindgen_test]
async fn test_different_hashers_independent() {
    // Verify different hasher instances are independent
    let hasher1 = Sha3WasmHasher::new("sha3-256").await.unwrap();
    let hasher2 = Sha3WasmHasher::new("sha3-512").await.unwrap();

    let input = to_uint8_array(b"test");

    let hash1 = hasher1.hash_single(&input).await.unwrap();
    let hash2 = hasher2.hash_single(&input).await.unwrap();

    assert_eq!(hash1.length(), 32);
    assert_eq!(hash2.length(), 64);
    assert_ne!(
        from_uint8_array(&hash1),
        from_uint8_array(&hash2)[..32]
    );
}
