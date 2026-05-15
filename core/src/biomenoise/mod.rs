//! 1.13+ noise-based biome generation primitives.
//!
//! Mirrors cubiomes' `biomenoise.{h,c}`. Modules land in order of
//! dependency: [`surface`] (`SurfaceNoise`, the 60-octave Perlin
//! stack) is the first to ship; `NetherNoise`, `EndNoise`, the
//! 1.18+ `BiomeNoise` (climate sampling + spline stack + `BiomeTree`
//! decision), and `BiomeNoiseBeta` follow in subsequent commits.

pub mod climate;
pub mod end;
pub mod nether;
pub mod surface;

pub use climate::{BiomeTree, climate_to_biome, get_np_dist, get_resulting_node, tree_for_version};
pub use end::EndNoise;
pub use nether::NetherNoise;
pub use surface::{SurfaceNoise, maintain_precision};
