//! WASM bindings for SHA-3 GPU acceleration
//! Provides Node.js and Bun.js compatible API for batch SHA-3 hashing

use js_sys::{Array, Uint8Array};
use sha3_core::{BatchHashParams, Sha3Variant};
use sha3_wgpu::{GpuContext, GpuSha3Hasher};
use wasm_bindgen::prelude::*;

/// Parse SHA-3 variant string to enum
fn parse_variant(variant: &str) -> Result<Sha3Variant, JsValue> {
    match variant.to_lowercase().as_str() {
        "sha3-224" | "sha3_224" => Ok(Sha3Variant::Sha3_224),
        "sha3-256" | "sha3_256" => Ok(Sha3Variant::Sha3_256),
        "sha3-384" | "sha3_384" => Ok(Sha3Variant::Sha3_384),
        "sha3-512" | "sha3_512" => Ok(Sha3Variant::Sha3_512),
        "shake128" => Ok(Sha3Variant::Shake128),
        "shake256" => Ok(Sha3Variant::Shake256),
        _ => Err(JsValue::from_str(&format!(
            "Invalid SHA-3 variant: {}. Valid options: sha3-224, sha3-256, sha3-384, sha3-512, shake128, shake256",
            variant
        ))),
    }
}

/// GPU-accelerated SHA-3 hasher for WASM
#[wasm_bindgen]
pub struct Sha3WasmHasher {
    hasher: GpuSha3Hasher,
    variant: Sha3Variant,
}

#[wasm_bindgen]
impl Sha3WasmHasher {
    /// Create a new SHA-3 hasher for the specified variant
    ///
    /// # Arguments
    /// * `variant` - SHA-3 variant: "sha3-224", "sha3-256", "sha3-384", "sha3-512", "shake128", or "shake256"
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// const hasher = await Sha3WasmHasher.new("sha3-256");
    /// ```
    pub async fn new(variant: &str) -> Result<Sha3WasmHasher, JsValue> {
        let variant_enum = parse_variant(variant)?;

        // Create GPU context
        let context = GpuContext::new()
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to initialize GPU: {}", e)))?;

        // Create hasher
        let hasher = GpuSha3Hasher::new(context, variant_enum)
            .map_err(|e| JsValue::from_str(&format!("Failed to create hasher: {}", e)))?;

        Ok(Self { hasher, variant: variant_enum })
    }

    /// Hash a single input
    ///
    /// # Arguments
    /// * `input` - Input data as Uint8Array
    ///
    /// # Returns
    /// Uint8Array containing the hash
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// const input = new TextEncoder().encode("hello world");
    /// const hash = await hasher.hashSingle(input);
    /// console.log(Buffer.from(hash).toString('hex'));
    /// ```
    #[wasm_bindgen(js_name = hashSingle)]
    pub async fn hash_single(&self, input: &Uint8Array) -> Result<Uint8Array, JsValue> {
        let input_bytes = input.to_vec();
        let inputs = vec![input_bytes.as_slice()];

        let result = self
            .hasher
            .hash_batch(&inputs)
            .await
            .map_err(|e| JsValue::from_str(&format!("Hashing failed: {}", e)))?;

        Ok(Uint8Array::from(&result[..]))
    }

    /// Hash a batch of inputs (optimized for GPU)
    /// All inputs must be the same length for optimal performance
    ///
    /// # Arguments
    /// * `inputs` - JavaScript array of Uint8Array inputs
    ///
    /// # Returns
    /// Array of Uint8Array hashes (same order as inputs)
    ///
    /// # Example (JavaScript)
    /// ```javascript
    /// const inputs = [
    ///   new TextEncoder().encode("hello"),
    ///   new TextEncoder().encode("world"),
    ///   new TextEncoder().encode("batch")
    /// ];
    /// const hashes = await hasher.hashBatch(inputs);
    /// hashes.forEach((hash, i) => {
    ///   console.log(`Hash ${i}: ${Buffer.from(hash).toString('hex')}`);
    /// });
    /// ```
    #[wasm_bindgen(js_name = hashBatch)]
    pub async fn hash_batch(&self, inputs: &Array) -> Result<Array, JsValue> {
        if inputs.length() == 0 {
            return Ok(Array::new());
        }

        // Convert JS arrays to Rust vectors
        let mut rust_inputs: Vec<Vec<u8>> = Vec::new();
        for i in 0..inputs.length() {
            let val = inputs.get(i);
            let uint8_array = Uint8Array::from(val);
            rust_inputs.push(uint8_array.to_vec());
        }

        // Create slice references
        let input_refs: Vec<&[u8]> = rust_inputs.iter().map(|v| v.as_slice()).collect();

        // Execute batch hashing
        let result = self
            .hasher
            .hash_batch(&input_refs)
            .await
            .map_err(|e| JsValue::from_str(&format!("Batch hashing failed: {}", e)))?;

        // Split result into individual hashes
        let output_size = self.variant.output_bytes();
        let result_array = Array::new();

        for chunk in result.chunks(output_size) {
            result_array.push(&Uint8Array::from(chunk));
        }

        Ok(result_array)
    }

    /// Hash a batch with custom output length (for SHAKE variants only)
    ///
    /// # Arguments
    /// * `inputs` - JavaScript array of Uint8Array inputs
    /// * `output_length` - Desired output length in bytes
    ///
    /// # Returns
    /// Array of Uint8Array hashes with specified length
    #[wasm_bindgen(js_name = hashBatchWithLength)]
    pub async fn hash_batch_with_length(
        &self,
        inputs: &Array,
        output_length: usize,
    ) -> Result<Array, JsValue> {
        if inputs.length() == 0 {
            return Ok(Array::new());
        }

        // Convert JS arrays to Rust vectors
        let mut rust_inputs: Vec<Vec<u8>> = Vec::new();
        for i in 0..inputs.length() {
            let val = inputs.get(i);
            let uint8_array = Uint8Array::from(val);
            rust_inputs.push(uint8_array.to_vec());
        }

        // Validate all inputs same length
        let input_length = rust_inputs[0].len();
        if !rust_inputs.iter().all(|v| v.len() == input_length) {
            return Err(JsValue::from_str(
                "All inputs must have the same length for batch processing",
            ));
        }

        // Create batch parameters
        let params = BatchHashParams::new(self.variant, rust_inputs.len(), input_length)
            .with_output_length(output_length);

        // Create slice references
        let input_refs: Vec<&[u8]> = rust_inputs.iter().map(|v| v.as_slice()).collect();

        // Execute batch hashing
        let result = self
            .hasher
            .hash_batch_with_params(&input_refs, &params)
            .await
            .map_err(|e| JsValue::from_str(&format!("Batch hashing failed: {}", e)))?;

        // Split result into individual hashes
        let result_array = Array::new();
        for chunk in result.chunks(output_length) {
            result_array.push(&Uint8Array::from(chunk));
        }

        Ok(result_array)
    }

    /// Get the SHA-3 variant name
    #[wasm_bindgen(js_name = getVariant)]
    pub fn get_variant(&self) -> String {
        match self.variant {
            Sha3Variant::Sha3_224 => "sha3-224".to_string(),
            Sha3Variant::Sha3_256 => "sha3-256".to_string(),
            Sha3Variant::Sha3_384 => "sha3-384".to_string(),
            Sha3Variant::Sha3_512 => "sha3-512".to_string(),
            Sha3Variant::Shake128 => "shake128".to_string(),
            Sha3Variant::Shake256 => "shake256".to_string(),
        }
    }

    /// Get the output size in bytes (0 for SHAKE variants)
    #[wasm_bindgen(js_name = getOutputSize)]
    pub fn get_output_size(&self) -> usize {
        self.variant.output_bytes()
    }
}

/// Convenience function: Hash a single input with specified variant
///
/// # Example (JavaScript)
/// ```javascript
/// const hash = await sha3("sha3-256", new TextEncoder().encode("hello"));
/// console.log(Buffer.from(hash).toString('hex'));
/// ```
#[wasm_bindgen]
pub async fn sha3(variant: &str, input: &Uint8Array) -> Result<Uint8Array, JsValue> {
    let hasher = Sha3WasmHasher::new(variant).await?;
    hasher.hash_single(input).await
}

/// Convenience function: Hash a batch of inputs with specified variant
///
/// # Example (JavaScript)
/// ```javascript
/// const inputs = [
///   new TextEncoder().encode("hello"),
///   new TextEncoder().encode("world")
/// ];
/// const hashes = await sha3Batch("sha3-256", inputs);
/// ```
#[wasm_bindgen(js_name = sha3Batch)]
pub async fn sha3_batch(variant: &str, inputs: &Array) -> Result<Array, JsValue> {
    let hasher = Sha3WasmHasher::new(variant).await?;
    hasher.hash_batch(inputs).await
}
