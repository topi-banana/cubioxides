//! Perlin / Simplex / Octave / `DoublePerlin` noise.
//!
//! Ports of `cubiomes/noise.c` (and the noise constants buried inside
//! `biomenoise.c`). All sample functions accept and return `f64` to stay
//! bit-exact with the C reference; clients downstream can downcast to
//! `f32` as needed.

pub mod double_perlin;
pub mod octave;
pub mod perlin;

pub use double_perlin::DoublePerlinNoise;
pub use octave::OctaveNoise;
pub use perlin::PerlinNoise;
