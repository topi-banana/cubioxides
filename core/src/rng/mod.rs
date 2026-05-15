//! Random number generators that match Minecraft's Java edition exactly.
//!
//! Ports of the inline helpers in `cubiomes/rng.h`. Each generator stores its
//! state as a `u64` even though Java's RNG only uses the lower 48 bits; this
//! keeps multiplication and addition inside Rust's native machine word.

pub mod java;
pub mod mc_seed;
pub mod xoroshiro;

pub use java::JavaRng;
pub use mc_seed::{
    get_chunk_seed, get_layer_salt, get_start_salt, get_start_seed, mc_first_int, mc_first_is_zero,
    mc_step_seed, mul_inv,
};
pub use xoroshiro::Xoroshiro;
