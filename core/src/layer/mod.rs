//! Pre-1.18 layered biome generation pipeline.
//!
//! Ports of cubiomes' `layers.c`. Each `mapfunc_t` from the C source
//! becomes a free function under `layer::ops` taking a slice for the
//! output grid and a `start_seed` (the seed produced by the world-seed
//! pipeline for this particular layer). Higher-level structures
//! (`Layer`, `LayerStack`, the DAG dispatch) land in later M3
//! sub-stages as more `mapfunc_t`s come online.

pub mod ops;

pub use ops::bamboo::map_bamboo;
pub use ops::biome::map_biome;
pub use ops::biome_edge::map_biome_edge;
pub use ops::continent::map_continent;
pub use ops::deep_ocean::map_deep_ocean;
pub use ops::hills::map_hills;
pub use ops::island::map_island;
pub use ops::land::{map_land, map_land_b18, map_land16};
pub use ops::mushroom::map_mushroom;
pub use ops::noise::map_noise;
pub use ops::ocean_mix::map_ocean_mix;
pub use ops::ocean_temp::map_ocean_temp;
pub use ops::river::map_river;
pub use ops::river_mix::map_river_mix;
pub use ops::shore::map_shore;
pub use ops::smooth::map_smooth;
pub use ops::snow::{map_snow, map_snow16};
pub use ops::special::map_special;
pub use ops::sunflower::map_sunflower;
pub use ops::swamp_river::map_swamp_river;
pub use ops::temperature::{map_cool, map_heat};
pub use ops::voronoi::map_voronoi114;
pub use ops::zoom::{map_zoom, map_zoom_fuzzy};
