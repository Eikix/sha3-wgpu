//! Core SHA-3 types and utilities

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
    }
}
