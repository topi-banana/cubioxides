//! 1.13+ noise-based biome generation primitives.
//!
//! Mirrors cubiomes' `biomenoise.{h,c}`. Modules land in order of
//! dependency: [`surface`] (`SurfaceNoise`, the 60-octave Perlin
//! stack) is the first to ship; `NetherNoise`, `EndNoise`, the
//! 1.18+ `BiomeNoise` (climate sampling + spline stack + `BiomeTree`
//! decision), and `BiomeNoiseBeta` follow in subsequent commits.

pub mod biome_noise;
pub mod climate;
pub mod end;
pub mod nether;
pub mod spline;
pub mod surface;

pub use biome_noise::{
    BiomeNoise, NP_CONTINENTALNESS, NP_DEPTH, NP_EROSION, NP_HUMIDITY, NP_MAX, NP_SHIFT,
    NP_TEMPERATURE, NP_WEIRDNESS, SAMPLE_NO_BIOME, SAMPLE_NO_DEPTH, SAMPLE_NO_SHIFT,
};
pub use climate::{BiomeTree, climate_to_biome, get_np_dist, get_resulting_node, tree_for_version};
pub use end::EndNoise;
pub use nether::NetherNoise;
pub use spline::{
    SplineAxis, SplineBranch, SplineNode, SplineStack, build_overworld_spline, sample_spline,
};
pub use surface::{SurfaceNoise, maintain_precision};
