//! Pre-1.18 layered biome generation pipeline.
//!
//! Ports of cubiomes' `layers.c`. Each `mapfunc_t` from the C source
//! becomes a free function under `layer::ops` taking a slice for the
//! output grid and a `start_seed` (the seed produced by the world-seed
//! pipeline for this particular layer). Higher-level structures
//! (`Layer`, `LayerStack`, the DAG dispatch) land in later M3
//! sub-stages as more `mapfunc_t`s come online.

pub mod ops;

pub use ops::continent::map_continent;
pub use ops::land::{map_land, map_land_b18, map_land16};
pub use ops::zoom::{map_zoom, map_zoom_fuzzy};
