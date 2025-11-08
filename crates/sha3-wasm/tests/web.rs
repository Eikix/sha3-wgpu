//! WASM tests using wasm-bindgen-test

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_sha3_wasm() {
    // TODO: Add WASM-specific tests
    assert!(true);
}

