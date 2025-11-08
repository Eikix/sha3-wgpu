//! Core types for SHA-3 operations

/// SHA-3 variant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sha3Variant {
    Sha3_224,
    Sha3_256,
    Sha3_384,
    Sha3_512,
    Shake128,
    Shake256,
}

impl Sha3Variant {
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
    
    pub fn output_bytes(&self) -> usize {
        self.output_bits() / 8
    }
}

