//! Core types for SHA-3 operations

/// SHA-3 variant
///
/// Represents the different SHA-3 hash function variants, including
/// fixed-length SHA3 and variable-length SHAKE variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sha3Variant {
    /// SHA3-224: 224-bit output (28 bytes)
    Sha3_224,
    /// SHA3-256: 256-bit output (32 bytes)
    Sha3_256,
    /// SHA3-384: 384-bit output (48 bytes)
    Sha3_384,
    /// SHA3-512: 512-bit output (64 bytes)
    Sha3_512,
    /// SHAKE128: Variable-length output, 128-bit security
    Shake128,
    /// SHAKE256: Variable-length output, 256-bit security
    Shake256,
}

impl Sha3Variant {
    /// Returns the output size in bits (0 for variable-length variants)
    pub fn output_bits(&self) -> usize {
        match self {
            Sha3Variant::Sha3_224 => 224,
            Sha3Variant::Sha3_256 => 256,
            Sha3Variant::Sha3_384 => 384,
            Sha3Variant::Sha3_512 => 512,
            Sha3Variant::Shake128 => 0, // Variable length
            Sha3Variant::Shake256 => 0, // Variable length
        }
    }

    /// Returns the output size in bytes (0 for variable-length variants)
    pub fn output_bytes(&self) -> usize {
        self.output_bits() / 8
    }

    /// Returns the rate (r) in bytes for this SHA-3 variant
    /// The rate is the number of bytes absorbed/squeezed per permutation
    pub fn rate_bytes(&self) -> usize {
        match self {
            Sha3Variant::Sha3_224 => 144, // 1152 bits = 144 bytes
            Sha3Variant::Sha3_256 => 136, // 1088 bits = 136 bytes
            Sha3Variant::Sha3_384 => 104, // 832 bits = 104 bytes
            Sha3Variant::Sha3_512 => 72,  // 576 bits = 72 bytes
            Sha3Variant::Shake128 => 168, // 1344 bits = 168 bytes
            Sha3Variant::Shake256 => 136, // 1088 bits = 136 bytes
        }
    }

    /// Returns the capacity (c) in bytes for this SHA-3 variant
    /// The capacity is the security parameter (rate + capacity = 1600 bits)
    pub fn capacity_bytes(&self) -> usize {
        200 - self.rate_bytes() // Total state is 1600 bits = 200 bytes
    }

    /// Returns the domain separation byte for this variant
    pub fn domain_separator(&self) -> u8 {
        match self {
            Sha3Variant::Sha3_224
            | Sha3Variant::Sha3_256
            | Sha3Variant::Sha3_384
            | Sha3Variant::Sha3_512 => 0x06, // SHA-3
            Sha3Variant::Shake128 | Sha3Variant::Shake256 => 0x1F, // SHAKE
        }
    }
}

/// Parameters for a batch hashing operation
#[derive(Debug, Clone)]
pub struct BatchHashParams {
    /// The SHA-3 variant to use
    pub variant: Sha3Variant,
    /// Number of hashes in this batch
    pub num_hashes: usize,
    /// Length of each input in bytes (all inputs must be same length for batching)
    pub input_length: usize,
    /// Output length in bytes (for SHAKE variants, otherwise ignored)
    pub output_length: Option<usize>,
}

impl BatchHashParams {
    /// Creates new batch parameters
    pub fn new(variant: Sha3Variant, num_hashes: usize, input_length: usize) -> Self {
        Self { variant, num_hashes, input_length, output_length: None }
    }

    /// Sets custom output length (for SHAKE variants)
    pub fn with_output_length(mut self, length: usize) -> Self {
        self.output_length = Some(length);
        self
    }

    /// Returns the output length in bytes for this batch
    ///
    /// # Errors
    ///
    /// Returns an error if this is a SHAKE variant without a specified output length
    pub fn get_output_bytes(&self) -> Result<usize, crate::error::Sha3Error> {
        match self.output_length {
            Some(len) => Ok(len),
            None => {
                let default_len = self.variant.output_bytes();
                if default_len > 0 {
                    Ok(default_len)
                } else {
                    Err(crate::error::Sha3Error::InvalidInputLength(0))
                }
            }
        }
    }
}
