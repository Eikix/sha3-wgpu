//! Error types for SHA-3 operations

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Sha3Error {
    #[error("Invalid input length: {0}")]
    InvalidInputLength(usize),

    #[error("GPU operation failed: {0}")]
    GpuError(String),

    #[error("WASM operation failed: {0}")]
    WasmError(String),
}
