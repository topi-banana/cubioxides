//! Smoke test for the cubiomes FFI link.
//!
//! Confirms that bindgen + cc produced a usable static library and that
//! the `cubioxides-core` [`MCVersion`] enum agrees with cubiomes' enum on
//! at least one representative discriminant.

#![allow(unsafe_code)]

use std::ffi::CStr;

use cubioxides::MCVersion;
use ffi_tests::mc2str;

#[test]
fn mc2str_returns_1_18_for_mc_1_18() {
    // SAFETY: `mc2str` returns a pointer to a static string with no aliasing
    // or lifetime concerns. We immediately copy the bytes into an owned
    // `String`, so the pointer does not outlive the assertion.
    let name = unsafe {
        let ptr = mc2str(MCVersion::V1_18.ord().into());
        assert!(!ptr.is_null(), "mc2str returned null");
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    };
    assert_eq!(
        name, "1.18",
        "cubiomes mc2str(MCVersion::V1_18) should be \"1.18\""
    );
}
