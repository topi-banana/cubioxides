//! Rust port of [cubiomes](https://github.com/Cubitect/cubiomes) — a fast and
//! version-accurate Minecraft world generation library.
//!
//! This crate aims for bit-for-bit numerical compatibility with cubiomes for
//! Minecraft Java Edition versions Beta 1.7 through 1.21+. The output of
//! every biome lookup, noise sample, and structure position calculation
//! matches the reference C implementation when given the same seed and
//! coordinates.
//!
//! # Example
//!
//! ```
//! use cubioxides::{Biome, Dimension, Generator, MCVersion, Range};
//!
//! let mut g = Generator::new(MCVersion::V1_21, 0);
//! g.apply_seed(Dimension::Overworld, 0xdead_beef);
//!
//! // Single-cell biome lookup at the 1:4 grid.
//! let _biome = g.biome_at(4, 0, 64, 0);
//!
//! // Bulk fill a 16×16 area at scale=4 in one call.
//! let mut cache = vec![Biome::NONE; 16 * 16];
//! g.gen_biomes(
//!     &mut cache,
//!     Range { scale: 4, x: 0, z: 0, sx: 16, sz: 16, y: 64, sy: 1 },
//! );
//! ```
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
pub mod biome_set;
pub mod biomenoise;
pub mod colors;
pub mod finder;
pub mod generator;
pub mod layer;
pub mod math;
pub mod mc_version;
pub mod noise;
pub mod rng;
pub mod sha;
pub mod tables;

pub use biome::Biome;
pub use generator::{FORCE_OCEAN_VARIANTS, Generator, LARGE_BIOMES, NO_BETA_OCEAN, Range};
pub use mc_version::{Dimension, MCVersion};
pub use noise::{DoublePerlinNoise, OctaveNoise, PerlinNoise};
pub use rng::{JavaRng, Xoroshiro};
