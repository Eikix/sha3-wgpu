//! WASM bindings for SHA-3 GPU acceleration

use wasm_bindgen::prelude::*;
use sha3_wgpu::GpuSha3Hasher;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub struct Sha3 {
    #[allow(dead_code)] // Will be used once implementation is complete
    hasher: GpuSha3Hasher,
}

#[wasm_bindgen]
impl Sha3 {
    #[wasm_bindgen(constructor)]
    pub fn new(_variant: &str) -> Result<Sha3, JsValue> {
        // TODO: Initialize GPU context and create hasher
        // For now, return an error since implementation is pending
        Err(JsValue::from_str("Not implemented"))
    }
    
    #[wasm_bindgen]
    pub async fn hash(&self, _input: &[u8]) -> Result<Vec<u8>, JsValue> {
        // TODO: Call GPU hasher and return result
        // For now, return an error since implementation is pending
        Err(JsValue::from_str("Not implemented"))
    }
}

#[wasm_bindgen]
pub fn greet() {
    alert("Hello, sha3-wgpu!");
}

