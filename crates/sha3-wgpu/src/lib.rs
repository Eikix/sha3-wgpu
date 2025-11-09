//! GPU-accelerated SHA-3 implementation using WGSL and wgpu

pub mod compute;
pub mod context;
pub mod error;

pub use compute::*;
pub use context::*;
pub use error::*;

#[cfg(test)]
mod tests {
    use super::*;
    use sha3::{Digest, Sha3_224, Sha3_256, Sha3_384, Sha3_512};
    use sha3_core::Sha3Variant;

    async fn test_variant_against_reference(
        variant: Sha3Variant,
        test_inputs: &[&[u8]],
    ) -> Result<(), GpuSha3Error> {
        // Create GPU hasher
        let context = GpuContext::new().await?;
        let gpu_hasher = GpuSha3Hasher::new(context, variant)?;

        // Hash with GPU
        let gpu_results = gpu_hasher.hash_batch(test_inputs).await?;

        // Compute reference hashes
        let output_size = variant.output_bytes();
        let mut expected = Vec::new();

        for input in test_inputs {
            let hash = match variant {
                Sha3Variant::Sha3_224 => {
                    let mut hasher = Sha3_224::new();
                    hasher.update(input);
                    hasher.finalize().to_vec()
                }
                Sha3Variant::Sha3_256 => {
                    let mut hasher = Sha3_256::new();
                    hasher.update(input);
                    hasher.finalize().to_vec()
                }
                Sha3Variant::Sha3_384 => {
                    let mut hasher = Sha3_384::new();
                    hasher.update(input);
                    hasher.finalize().to_vec()
                }
                Sha3Variant::Sha3_512 => {
                    let mut hasher = Sha3_512::new();
                    hasher.update(input);
                    hasher.finalize().to_vec()
                }
                _ => panic!("Unsupported variant for reference test"),
            };
            expected.extend_from_slice(&hash);
        }

        // Compare results
        assert_eq!(gpu_results.len(), expected.len(), "Result length mismatch for {variant:?}");

        for (i, (gpu_chunk, ref_chunk)) in
            gpu_results.chunks(output_size).zip(expected.chunks(output_size)).enumerate()
        {
            assert_eq!(
                gpu_chunk,
                ref_chunk,
                "Hash mismatch at index {i} for {variant:?}\nGPU:  {}\nCPU:  {}",
                hex::encode(gpu_chunk),
                hex::encode(ref_chunk)
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_sha3_256_empty() {
        let inputs = vec![b"".as_slice()];
        test_variant_against_reference(Sha3Variant::Sha3_256, &inputs).await.unwrap();
    }

    #[tokio::test]
    async fn test_sha3_256_single() {
        let inputs = vec![b"hello world".as_slice()];
        test_variant_against_reference(Sha3Variant::Sha3_256, &inputs).await.unwrap();
    }

    #[tokio::test]
    async fn test_sha3_256_batch() {
        let inputs = vec![
            b"hello".as_slice(),
            b"world".as_slice(),
            b"batch".as_slice(),
            b"tests".as_slice(),
        ];
        test_variant_against_reference(Sha3Variant::Sha3_256, &inputs).await.unwrap();
    }

    #[tokio::test]
    async fn test_sha3_256_large_batch() {
        // Ensure all inputs have the same length by using fixed-width formatting
        let data: Vec<Vec<u8>> =
            (0..100).map(|i| format!("test input number {i:03}").into_bytes()).collect();
        let inputs: Vec<&[u8]> = data.iter().map(|v| v.as_slice()).collect();

        test_variant_against_reference(Sha3Variant::Sha3_256, &inputs).await.unwrap();
    }

    #[tokio::test]
    async fn test_sha3_224_batch() {
        let inputs = vec![b"test1".as_slice(), b"test2".as_slice(), b"test3".as_slice()];
        test_variant_against_reference(Sha3Variant::Sha3_224, &inputs).await.unwrap();
    }

    #[tokio::test]
    async fn test_sha3_384_batch() {
        let inputs = vec![b"test1".as_slice(), b"test2".as_slice(), b"test3".as_slice()];
        test_variant_against_reference(Sha3Variant::Sha3_384, &inputs).await.unwrap();
    }

    #[tokio::test]
    async fn test_sha3_512_batch() {
        let inputs = vec![b"test1".as_slice(), b"test2".as_slice(), b"test3".as_slice()];
        test_variant_against_reference(Sha3Variant::Sha3_512, &inputs).await.unwrap();
    }

    #[tokio::test]
    async fn test_all_variants_same_input() {
        let inputs = vec![b"The quick brown fox jumps over the lazy dog".as_slice()];

        for variant in &[
            Sha3Variant::Sha3_224,
            Sha3Variant::Sha3_256,
            Sha3Variant::Sha3_384,
            Sha3Variant::Sha3_512,
        ] {
            test_variant_against_reference(*variant, &inputs).await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_long_input() {
        // Test with 8000 bytes (within the 8KB GPU buffer limit)
        let long_input = vec![b'a'; 8000];
        let inputs = vec![long_input.as_slice()];

        test_variant_against_reference(Sha3Variant::Sha3_256, &inputs).await.unwrap();
    }

    #[tokio::test]
    async fn test_varying_lengths_batch() {
        // Test with inputs of different lengths in same batch
        // This should work as long as we pad each correctly
        let input1 = b"short";
        let input2 = b"medium length input";
        let input3 = b"a very long input that spans many more bytes than the others";

        // Test each individually since batch requires same length
        test_variant_against_reference(Sha3Variant::Sha3_256, &[input1]).await.unwrap();
        test_variant_against_reference(Sha3Variant::Sha3_256, &[input2]).await.unwrap();
        test_variant_against_reference(Sha3Variant::Sha3_256, &[input3]).await.unwrap();
    }

    // SHAKE variant tests (from audit report)
    #[tokio::test]
    async fn test_shake128_default_output() {
        use sha3_core::BatchHashParams;

        let context = GpuContext::new().await.unwrap();
        let hasher = GpuSha3Hasher::new(context, Sha3Variant::Shake128).unwrap();
        let inputs = vec![b"test".as_slice()];
        let params = BatchHashParams::new(Sha3Variant::Shake128, 1, 4).with_output_length(32);
        let result = hasher.hash_batch_with_params(&inputs, &params).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 32);
    }

    #[tokio::test]
    async fn test_shake128_custom_output_length() {
        use sha3_core::BatchHashParams;

        let context = GpuContext::new().await.unwrap();
        let hasher = GpuSha3Hasher::new(context, Sha3Variant::Shake128).unwrap();
        let inputs = vec![b"test input for SHAKE128".as_slice()];

        // Test various output lengths
        for output_len in [16, 32, 64, 128] {
            let params = BatchHashParams::new(Sha3Variant::Shake128, 1, inputs[0].len())
                .with_output_length(output_len);
            let result = hasher.hash_batch_with_params(&inputs, &params).await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap().len(), output_len);
        }
    }

    #[tokio::test]
    async fn test_shake256_custom_output_length() {
        use sha3_core::BatchHashParams;

        let context = GpuContext::new().await.unwrap();
        let hasher = GpuSha3Hasher::new(context, Sha3Variant::Shake256).unwrap();
        let inputs = vec![b"test input for SHAKE256".as_slice()];

        let params =
            BatchHashParams::new(Sha3Variant::Shake256, 1, inputs[0].len()).with_output_length(64);
        let result = hasher.hash_batch_with_params(&inputs, &params).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 64);
    }

    // Error path tests (from audit report)
    #[tokio::test]
    async fn test_error_mismatched_input_lengths() {
        let context = GpuContext::new().await.unwrap();
        let hasher = GpuSha3Hasher::new(context, Sha3Variant::Sha3_256).unwrap();
        let inputs = vec![b"short".as_slice(), b"longer input".as_slice()];
        let result = hasher.hash_batch(&inputs).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GpuSha3Error::InvalidInputLength(_)));
    }

    #[tokio::test]
    async fn test_error_input_too_large() {
        // Test that inputs exceeding the 8KB GPU buffer limit are rejected
        let context = GpuContext::new().await.unwrap();
        let hasher = GpuSha3Hasher::new(context, Sha3Variant::Sha3_256).unwrap();
        let oversized_input = vec![b'x'; 10000]; // Exceeds 8192 byte limit
        let inputs = vec![oversized_input.as_slice()];
        let result = hasher.hash_batch(&inputs).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GpuSha3Error::InvalidInputLength(_)));
    }

    #[tokio::test]
    async fn test_error_shake_without_output_length() {
        use sha3_core::BatchHashParams;

        let context = GpuContext::new().await.unwrap();
        let hasher = GpuSha3Hasher::new(context, Sha3Variant::Shake128).unwrap();
        let inputs = vec![b"test".as_slice()];

        // Create params without setting output_length for SHAKE variant
        let params = BatchHashParams::new(Sha3Variant::Shake128, 1, 4);
        let result = hasher.hash_batch_with_params(&inputs, &params).await;
        assert!(result.is_err());
    }

    // Rate boundary tests (from audit report)
    #[tokio::test]
    async fn test_sha3_224_at_rate_boundary() {
        // SHA3-224 rate is 144 bytes - test 143, 144, 145 byte inputs
        let context = GpuContext::new().await.unwrap();
        let hasher = GpuSha3Hasher::new(context, Sha3Variant::Sha3_224).unwrap();

        for size in [143, 144, 145] {
            let input = vec![0xAAu8; size];
            let inputs = vec![input.as_slice()];
            let result = hasher.hash_batch(&inputs).await;
            assert!(result.is_ok(), "Failed for size {size}");
        }
    }

    #[tokio::test]
    async fn test_sha3_256_at_rate_boundary() {
        // SHA3-256 rate is 136 bytes - test 135, 136, 137 byte inputs
        let context = GpuContext::new().await.unwrap();
        let hasher = GpuSha3Hasher::new(context, Sha3Variant::Sha3_256).unwrap();

        for size in [135, 136, 137] {
            let input = vec![0xBBu8; size];
            let inputs = vec![input.as_slice()];
            let result = hasher.hash_batch(&inputs).await;
            assert!(result.is_ok(), "Failed for size {size}");
        }
    }

    #[tokio::test]
    async fn test_sha3_384_at_rate_boundary() {
        // SHA3-384 rate is 104 bytes - test 103, 104, 105 byte inputs
        let context = GpuContext::new().await.unwrap();
        let hasher = GpuSha3Hasher::new(context, Sha3Variant::Sha3_384).unwrap();

        for size in [103, 104, 105] {
            let input = vec![0xCCu8; size];
            let inputs = vec![input.as_slice()];
            let result = hasher.hash_batch(&inputs).await;
            assert!(result.is_ok(), "Failed for size {size}");
        }
    }

    #[tokio::test]
    async fn test_sha3_512_at_rate_boundary() {
        // SHA3-512 rate is 72 bytes - test 71, 72, 73 byte inputs
        let context = GpuContext::new().await.unwrap();
        let hasher = GpuSha3Hasher::new(context, Sha3Variant::Sha3_512).unwrap();

        for size in [71, 72, 73] {
            let input = vec![0xDDu8; size];
            let inputs = vec![input.as_slice()];
            let result = hasher.hash_batch(&inputs).await;
            assert!(result.is_ok(), "Failed for size {size}");
        }
    }

    // Concurrent usage tests (from audit report)
    #[tokio::test]
    async fn test_concurrent_batch_hashing() {
        use std::sync::Arc;

        let context = GpuContext::new().await.unwrap();
        let hasher = Arc::new(GpuSha3Hasher::new(context, Sha3Variant::Sha3_256).unwrap());

        // Launch multiple concurrent hash_batch calls
        let mut handles = vec![];
        for i in 0..5 {
            let hasher_clone = Arc::clone(&hasher);
            let handle = tokio::spawn(async move {
                let input = format!("concurrent test {i}");
                let inputs = vec![input.as_bytes()];
                hasher_clone.hash_batch(&inputs).await
            });
            handles.push(handle);
        }

        // Wait for all to complete
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }
    }
}
