//! Noise-based biome generation primitives.
//!
//! Mirrors cubiomes' `biomenoise.{h,c}`:
//! - [`surface`] — `SurfaceNoise`, the 60-octave Perlin stack used
//!   by 1.13+ surface-height sampling.
//! - [`surface_beta`] — `SurfaceNoiseBeta` plus the sea-level
//!   column-noise helpers used by Beta 1.7/1.8 ocean override.
//! - [`nether`] — `NetherNoise` (the 1.16+ 3D climate sampler).
//! - [`end`] — `EndNoise` (the 1.9+ End-dim biome sampler).
//! - [`biome_noise`] — `BiomeNoise` (1.18+ climate noise + spline
//!   stack + `BiomeTree` decision).
//! - [`beta`] — `BiomeNoiseBeta` plus the Beta sea-level oceans
//!   bridge into `gen_biome_noise_beta_scaled`.
//! - [`climate`] — `climate_to_biome` decision-tree lookup (1.18+).
//! - [`spline`] — depth-spline builder + sampler.
//! - [`end_surface`] — End surface-height sampler (`mapEndSurfaceHeight`
//!   + `getEndSurfaceHeight`).

pub mod beta;
pub mod biome_noise;
pub mod climate;
pub mod end;
pub mod end_surface;
pub mod nether;
pub mod spline;
pub mod surface;
pub mod surface_beta;

pub use beta::{
    BiomeNoiseBeta, gen_biome_noise_beta_scaled, gen_biome_noise_beta_scaled_simple,
    get_old_beta_biome,
};
pub use biome_noise::{
    BiomeNoise, NP_CONTINENTALNESS, NP_DEPTH, NP_EROSION, NP_HUMIDITY, NP_MAX, NP_SHIFT,
    NP_TEMPERATURE, NP_WEIRDNESS, SAMPLE_NO_BIOME, SAMPLE_NO_DEPTH, SAMPLE_NO_SHIFT,
};
pub use climate::{BiomeTree, climate_to_biome, get_np_dist, get_resulting_node, tree_for_version};
pub use end::EndNoise;
pub use end_surface::{get_end_surface_height, map_end_surface_height};
pub use nether::NetherNoise;
pub use spline::{
    SplineAxis, SplineBranch, SplineNode, SplineStack, build_overworld_spline, sample_spline,
};
pub use surface::{SurfaceNoise, maintain_precision};
pub use surface_beta::{
    SeaLevelColumnNoiseBeta, SurfaceNoiseBeta, approx_surface_beta, gen_column_noise,
    process_column_noise,
};
