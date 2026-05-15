//! Random number generators that match Minecraft's Java edition exactly.
//!
//! Ports of the inline helpers in `cubiomes/rng.h`. Each generator stores its
//! state as a `u64` even though Java's RNG only uses the lower 48 bits; this
//! keeps multiplication and addition inside Rust's native machine word.

pub mod java;

pub use java::JavaRng;
