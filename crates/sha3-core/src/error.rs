//! Error types for SHA-3 operations

use thiserror::Error;

/// Errors that can occur during SHA-3 operations
#[derive(Debug, Error)]
pub enum Sha3Error {
    /// Invalid input length provided
    #[error("Invalid input length: {0}")]
    InvalidInputLength(usize),

    /// GPU operation failed with the given error message
    #[error("GPU operation failed: {0}")]
    GpuError(String),

    /// WASM operation failed with the given error message
    #[error("WASM operation failed: {0}")]
    WasmError(String),
}
