//! Parity tests: cubioxides noise primitives vs cubiomes via fixtures.
//!
//! Loads the binary records produced by `fixtures-gen noise` and runs
//! the equivalent Rust paths through `PerlinNoise::{sample,
//! sample_simplex_2d}` for both Java and Xoroshiro seeding. `f64`
//! outputs are compared by `to_bits` so the result is bit-exact.

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::noise::{DoublePerlinNoise, OctaveNoise, PerlinNoise};
use cubioxides::rng::{JavaRng, Xoroshiro};

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Header {
    magic: [u8; 4],
    format_version: u16,
    kind: u16,
    record_count: u64,
    reserved: [u64; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PerlinRecord {
    seed: u64,
    x: f64,
    y: f64,
    z: f64,
    yamp: f64,
    ymin: f64,
    java_sample_bits: u64,
    xoroshiro_sample_bits: u64,
    java_simplex_bits: u64,
    xoroshiro_simplex_bits: u64,
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("fixtures")
        .join("noise")
}

fn load_fixture<R: Pod>(name: &str, expected_kind: u16) -> Vec<R> {
    let path = fixture_dir().join(name);
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (header_bytes, body_bytes) = bytes.split_at(std::mem::size_of::<Header>());
    let header: &Header = bytemuck::from_bytes(header_bytes);
    assert_eq!(header.magic, MAGIC, "wrong magic in {}", path.display());
    assert_eq!(
        header.format_version,
        FORMAT_VERSION,
        "unsupported format version in {}",
        path.display()
    );
    assert_eq!(
        header.kind,
        expected_kind,
        "wrong fixture kind in {}",
        path.display()
    );
    let records: &[R] = bytemuck::cast_slice(body_bytes);
    assert_eq!(
        records.len() as u64,
        header.record_count,
        "record count mismatch in {}",
        path.display()
    );
    records.to_vec()
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct OctaveRecord {
    seed: u64,
    x: f64,
    y: f64,
    z: f64,
    java_sample_bits: u64,
    xoroshiro_sample_bits: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct DoublePerlinRecord {
    seed: u64,
    x: f64,
    y: f64,
    z: f64,
    java_sample_bits: u64,
    xoroshiro_sample_bits: u64,
}

const OCT_OMIN: i32 = -3;
const OCT_LEN: i32 = 4;
const OCT_AMPS: [f64; 4] = [1.0, 1.0, 1.0, 1.0];

#[test]
fn perlin_matches_cubiomes() {
    let records: Vec<PerlinRecord> = load_fixture("perlin.bin", 4);
    assert!(!records.is_empty());

    for (i, rec) in records.iter().enumerate() {
        // sample_perlin via Java seed.
        let mut rng = JavaRng::new(rec.seed);
        let pn = PerlinNoise::from_java(&mut rng);
        let v = pn.sample(rec.x, rec.y, rec.z, rec.yamp, rec.ymin);
        assert_eq!(
            v.to_bits(),
            rec.java_sample_bits,
            "java sample at record {i} (seed = {})",
            rec.seed
        );

        // sample_simplex_2d via Java seed.
        let s = pn.sample_simplex_2d(rec.x, rec.z);
        assert_eq!(
            s.to_bits(),
            rec.java_simplex_bits,
            "java simplex at record {i} (seed = {})",
            rec.seed
        );

        // sample_perlin via Xoroshiro seed.
        let mut xr = Xoroshiro::new(rec.seed);
        let pn = PerlinNoise::from_xoroshiro(&mut xr);
        let v = pn.sample(rec.x, rec.y, rec.z, rec.yamp, rec.ymin);
        assert_eq!(
            v.to_bits(),
            rec.xoroshiro_sample_bits,
            "xoroshiro sample at record {i} (seed = {})",
            rec.seed
        );

        // sample_simplex_2d via Xoroshiro seed.
        let s = pn.sample_simplex_2d(rec.x, rec.z);
        assert_eq!(
            s.to_bits(),
            rec.xoroshiro_simplex_bits,
            "xoroshiro simplex at record {i} (seed = {})",
            rec.seed
        );
    }
}

#[test]
fn octave_matches_cubiomes() {
    let records: Vec<OctaveRecord> = load_fixture("octave.bin", 5);
    assert!(!records.is_empty());

    for (i, rec) in records.iter().enumerate() {
        // Java octaveInit + sampleOctave.
        let mut rng = JavaRng::new(rec.seed);
        let oct = OctaveNoise::from_java(&mut rng, OCT_OMIN, OCT_LEN);
        let v = oct.sample(rec.x, rec.y, rec.z);
        assert_eq!(
            v.to_bits(),
            rec.java_sample_bits,
            "java octave sample at record {i} (seed = {})",
            rec.seed
        );

        // Xoroshiro xOctaveInit + sampleOctave (amplitudes = [1; 4]).
        let mut xr = Xoroshiro::new(rec.seed);
        let oct = OctaveNoise::from_xoroshiro(&mut xr, &OCT_AMPS, OCT_OMIN, None);
        let v = oct.sample(rec.x, rec.y, rec.z);
        assert_eq!(
            v.to_bits(),
            rec.xoroshiro_sample_bits,
            "xoroshiro octave sample at record {i} (seed = {})",
            rec.seed
        );
    }
}

#[test]
fn double_perlin_matches_cubiomes() {
    let records: Vec<DoublePerlinRecord> = load_fixture("double_perlin.bin", 6);
    assert!(!records.is_empty());

    for (i, rec) in records.iter().enumerate() {
        // Java doublePerlinInit + sampleDoublePerlin.
        let mut rng = JavaRng::new(rec.seed);
        let dp = DoublePerlinNoise::from_java(&mut rng, OCT_OMIN, OCT_LEN);
        let v = dp.sample(rec.x, rec.y, rec.z);
        assert_eq!(
            v.to_bits(),
            rec.java_sample_bits,
            "java double_perlin sample at record {i} (seed = {})",
            rec.seed
        );

        // Xoroshiro xDoublePerlinInit + sampleDoublePerlin.
        let mut xr = Xoroshiro::new(rec.seed);
        let dp = DoublePerlinNoise::from_xoroshiro(&mut xr, &OCT_AMPS, OCT_OMIN, None);
        let v = dp.sample(rec.x, rec.y, rec.z);
        assert_eq!(
            v.to_bits(),
            rec.xoroshiro_sample_bits,
            "xoroshiro double_perlin sample at record {i} (seed = {})",
            rec.seed
        );
    }
}
