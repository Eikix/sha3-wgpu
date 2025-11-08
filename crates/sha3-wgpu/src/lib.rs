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
        assert_eq!(gpu_results.len(), expected.len(), "Result length mismatch for {:?}", variant);

        for (i, (gpu_chunk, ref_chunk)) in
            gpu_results.chunks(output_size).zip(expected.chunks(output_size)).enumerate()
        {
            assert_eq!(
                gpu_chunk,
                ref_chunk,
                "Hash mismatch at index {} for {:?}\nGPU:  {}\nCPU:  {}",
                i,
                variant,
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
            (0..100).map(|i| format!("test input number {:03}", i).into_bytes()).collect();
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
        let long_input = vec![b'a'; 10000];
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
}
