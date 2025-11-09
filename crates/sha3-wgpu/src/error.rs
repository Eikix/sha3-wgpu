//! GPU-specific error types

use sha3_core::Sha3Error;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GpuSha3Error {
    #[error("Core SHA-3 error: {0}")]
    Core(#[from] Sha3Error),

    #[error("GPU adapter not found: {0}")]
    AdapterNotFound(String),

    #[error("Device creation failed: {0}")]
    DeviceCreation(String),

    #[error("Buffer mapping failed: {0}")]
    BufferMapping(String),

    #[error("Shader compilation failed: {0}")]
    ShaderCompilation(String),

    #[error("Invalid input length: {0}")]
    InvalidInputLength(usize),

    #[error("GPU operation failed: {0}")]
    GpuOperationFailed(String),
}
