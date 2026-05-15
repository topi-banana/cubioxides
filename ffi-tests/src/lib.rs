//! FFI bindings for the cubiomes C library used by differential tests.
//!
//! The `cubioxides-core` crate is the implementation under test; this
//! crate provides a thin Rust wrapper around the C reference so that
//! tests under `tests/` can compare the two implementations directly.

#![allow(
    unsafe_code,
    missing_docs,
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code
)]
#![allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
