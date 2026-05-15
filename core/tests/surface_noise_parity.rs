//! Parity test: `cubioxides::biomenoise::SurfaceNoise::sample` /
//! `sample_between` vs cubiomes' `sampleSurfaceNoise` /
//! `sampleSurfaceNoiseBetween`. Reads the binary fixture produced by
//! `fixtures-gen noise` (kind = 40) and compares `f64` outputs by raw
//! bit pattern.

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::biomenoise::SurfaceNoise;
use cubioxides::mc_version::Dimension;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 40;

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
struct SurfaceNoiseRecord {
    dim: i32,
    x: i32,
    y: i32,
    z: i32,
    seed: u64,
    noise_min: f64,
    noise_max: f64,
    sample_bits: u64,
    between_bits: u64,
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("fixtures")
        .join("noise")
        .join("surface_noise.bin")
}

#[test]
fn surface_noise_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (header_bytes, body_bytes) = bytes.split_at(std::mem::size_of::<Header>());
    let header: &Header = bytemuck::from_bytes(header_bytes);
    assert_eq!(header.magic, MAGIC);
    assert_eq!(header.format_version, FORMAT_VERSION);
    assert_eq!(header.kind, KIND);
    let records: &[SurfaceNoiseRecord] = bytemuck::cast_slice(body_bytes);
    assert_eq!(records.len() as u64, header.record_count);
    assert!(!records.is_empty());

    for (i, rec) in records.iter().enumerate() {
        let dim = match rec.dim {
            0 => Dimension::Overworld,
            1 => Dimension::End,
            other => panic!("unsupported dim ord {other}"),
        };
        let sn = SurfaceNoise::init(dim, rec.seed);
        let sample = sn.sample(rec.x, rec.y, rec.z);
        let between = sn.sample_between(rec.x, rec.y, rec.z, rec.noise_min, rec.noise_max);
        assert_eq!(
            sample.to_bits(),
            rec.sample_bits,
            "sample mismatch at record {i} (dim={:?}, seed={:#x}, x={}, y={}, z={}): got {sample:?}, want {:?}",
            dim,
            rec.seed,
            rec.x,
            rec.y,
            rec.z,
            f64::from_bits(rec.sample_bits)
        );
        assert_eq!(
            between.to_bits(),
            rec.between_bits,
            "between mismatch at record {i} (dim={:?}, seed={:#x}, x={}, y={}, z={}, nmin={}, nmax={}): got {between:?}, want {:?}",
            dim,
            rec.seed,
            rec.x,
            rec.y,
            rec.z,
            rec.noise_min,
            rec.noise_max,
            f64::from_bits(rec.between_bits)
        );
    }
}
