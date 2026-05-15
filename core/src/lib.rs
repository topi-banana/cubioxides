//! Rust port of [cubiomes](https://github.com/Cubitect/cubiomes) — a fast and
//! version-accurate Minecraft world generation library.
//!
//! This crate aims for bit-for-bit numerical compatibility with cubiomes for
//! Minecraft Java Edition versions Beta 1.7 through 1.21+. The output of
//! every biome lookup, noise sample, and structure position calculation
//! matches the reference C implementation when given the same seed and
//! coordinates.
//!
//! # wasm32 compatibility
//!
//! This crate is intended to build and run on `wasm32-unknown-unknown`. It
//! avoids `std::fs`, `std::thread`, `std::process`, `std::env`, and
//! `std::time::SystemTime`. Features that require host facilities (currently
//! `parallel` and `colors`) are gated on `not(target_arch = "wasm32")`.

#![forbid(unsafe_code)]
#![cfg_attr(not(test), warn(missing_docs))]

pub mod biome;
pub mod layer;
pub mod math;
pub mod mc_version;
pub mod noise;
pub mod rng;
pub mod sha;

pub use biome::Biome;
pub use mc_version::{Dimension, MCVersion};
pub use noise::{DoublePerlinNoise, OctaveNoise, PerlinNoise};
pub use rng::{JavaRng, Xoroshiro};
