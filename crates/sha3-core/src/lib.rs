//! Core SHA-3 types and utilities
//!
//! This crate provides the fundamental types used across the sha3-wgpu ecosystem,
//! including SHA-3 variants, batch hashing parameters, and error types.
//!
//! # Examples
//!
//! ```rust
//! use sha3_core::{Sha3Variant, BatchHashParams};
//!
//! // Create batch parameters for SHA3-256
//! let params = BatchHashParams::new(Sha3Variant::Sha3_256, 10, 64);
//!
//! // For SHAKE variants, specify output length
//! let shake_params = BatchHashParams::new(Sha3Variant::Shake128, 5, 32)
//!     .with_output_length(64);
//! ```

#![warn(missing_docs)]

pub mod error;
pub mod types;

pub use error::Sha3Error;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha3_variant_output_bits() {
        assert_eq!(Sha3Variant::Sha3_224.output_bits(), 224);
        assert_eq!(Sha3Variant::Sha3_256.output_bits(), 256);
        assert_eq!(Sha3Variant::Sha3_384.output_bits(), 384);
        assert_eq!(Sha3Variant::Sha3_512.output_bits(), 512);
        assert_eq!(Sha3Variant::Shake128.output_bits(), 0);
        assert_eq!(Sha3Variant::Shake256.output_bits(), 0);
    }

    #[test]
    fn test_sha3_variant_output_bytes() {
        assert_eq!(Sha3Variant::Sha3_224.output_bytes(), 28);
        assert_eq!(Sha3Variant::Sha3_256.output_bytes(), 32);
        assert_eq!(Sha3Variant::Sha3_384.output_bytes(), 48);
        assert_eq!(Sha3Variant::Sha3_512.output_bytes(), 64);
        assert_eq!(Sha3Variant::Shake128.output_bytes(), 0);
        assert_eq!(Sha3Variant::Shake256.output_bytes(), 0);
    }

    #[test]
    fn test_sha3_variant_rate_bytes() {
        assert_eq!(Sha3Variant::Sha3_224.rate_bytes(), 144);
        assert_eq!(Sha3Variant::Sha3_256.rate_bytes(), 136);
        assert_eq!(Sha3Variant::Sha3_384.rate_bytes(), 104);
        assert_eq!(Sha3Variant::Sha3_512.rate_bytes(), 72);
        assert_eq!(Sha3Variant::Shake128.rate_bytes(), 168);
        assert_eq!(Sha3Variant::Shake256.rate_bytes(), 136);
    }

    #[test]
    fn test_sha3_variant_capacity_bytes() {
        // rate + capacity should equal 200 bytes (1600 bits)
        assert_eq!(Sha3Variant::Sha3_224.capacity_bytes(), 200 - 144);
        assert_eq!(Sha3Variant::Sha3_256.capacity_bytes(), 200 - 136);
        assert_eq!(Sha3Variant::Sha3_384.capacity_bytes(), 200 - 104);
        assert_eq!(Sha3Variant::Sha3_512.capacity_bytes(), 200 - 72);
        assert_eq!(Sha3Variant::Shake128.capacity_bytes(), 200 - 168);
        assert_eq!(Sha3Variant::Shake256.capacity_bytes(), 200 - 136);
    }

    #[test]
    fn test_sha3_variant_domain_separator() {
        assert_eq!(Sha3Variant::Sha3_224.domain_separator(), 0x06);
        assert_eq!(Sha3Variant::Sha3_256.domain_separator(), 0x06);
        assert_eq!(Sha3Variant::Sha3_384.domain_separator(), 0x06);
        assert_eq!(Sha3Variant::Sha3_512.domain_separator(), 0x06);
        assert_eq!(Sha3Variant::Shake128.domain_separator(), 0x1F);
        assert_eq!(Sha3Variant::Shake256.domain_separator(), 0x1F);
    }

    #[test]
    fn test_batch_hash_params_new() {
        let params = BatchHashParams::new(Sha3Variant::Sha3_256, 10, 64);
        assert_eq!(params.variant, Sha3Variant::Sha3_256);
        assert_eq!(params.num_hashes, 10);
        assert_eq!(params.input_length, 64);
        assert_eq!(params.output_length, None);
    }

    #[test]
    fn test_batch_hash_params_with_output_length() {
        let params = BatchHashParams::new(Sha3Variant::Shake128, 5, 32)
            .with_output_length(64);
        assert_eq!(params.output_length, Some(64));
    }

    #[test]
    fn test_batch_hash_params_get_output_bytes_fixed_length() {
        let params = BatchHashParams::new(Sha3Variant::Sha3_256, 10, 64);
        assert_eq!(params.get_output_bytes().unwrap(), 32);
    }

    #[test]
    fn test_batch_hash_params_get_output_bytes_shake_with_length() {
        let params = BatchHashParams::new(Sha3Variant::Shake128, 5, 32)
            .with_output_length(64);
        assert_eq!(params.get_output_bytes().unwrap(), 64);
    }

    #[test]
    fn test_batch_hash_params_get_output_bytes_shake_without_length() {
        let params = BatchHashParams::new(Sha3Variant::Shake128, 5, 32);
        assert!(params.get_output_bytes().is_err());
    }

    #[test]
    fn test_batch_hash_params_get_output_bytes_custom_override() {
        // Even for fixed-length variants, custom output_length takes precedence
        let params = BatchHashParams::new(Sha3Variant::Sha3_256, 5, 32)
            .with_output_length(16);
        assert_eq!(params.get_output_bytes().unwrap(), 16);
    }
}
