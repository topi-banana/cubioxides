//! Parity test: cubioxides' `BiomeNoiseBeta::sample` vs cubiomes'
//! `setBetaBiomeSeed` + `sampleBiomeNoiseBeta`. Reads the binary
//! fixture produced by `fixtures-gen noise` (kind = 45) and compares
//! the chosen biome id plus the underlying clamped `(t, h)` doubles
//! for 1024 random `(seed, x, z)` combinations.

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::biomenoise::BiomeNoiseBeta;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 45;

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
struct BiomeNoiseBetaRecord {
    seed: u64,
    t_bits: u64,
    h_bits: u64,
    x: i32,
    z: i32,
    biome_id: i32,
    pad: u32,
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("fixtures")
        .join("noise")
        .join("biome_noise_beta.bin")
}

#[test]
fn biome_noise_beta_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (header_bytes, body_bytes) = bytes.split_at(std::mem::size_of::<Header>());
    let header: &Header = bytemuck::from_bytes(header_bytes);
    assert_eq!(header.magic, MAGIC);
    assert_eq!(header.format_version, FORMAT_VERSION);
    assert_eq!(header.kind, KIND);
    let records: &[BiomeNoiseBetaRecord] = bytemuck::cast_slice(body_bytes);
    assert_eq!(records.len() as u64, header.record_count);

    for (i, rec) in records.iter().enumerate() {
        let bnb = BiomeNoiseBeta::set_seed(rec.seed);
        let (biome, t, h) = bnb.sample(rec.x, rec.z);
        assert_eq!(
            biome.id(),
            rec.biome_id,
            "biome mismatch at {i} (seed={:#x}, x={}, z={}): got {}, want {}",
            rec.seed,
            rec.x,
            rec.z,
            biome.id(),
            rec.biome_id
        );
        assert_eq!(
            t.to_bits(),
            rec.t_bits,
            "t mismatch at {i} (seed={:#x}, x={}, z={}): got {t:?}, want {:?}",
            rec.seed,
            rec.x,
            rec.z,
            f64::from_bits(rec.t_bits)
        );
        assert_eq!(
            h.to_bits(),
            rec.h_bits,
            "h mismatch at {i} (seed={:#x}, x={}, z={}): got {h:?}, want {:?}",
            rec.seed,
            rec.x,
            rec.z,
            f64::from_bits(rec.h_bits)
        );
    }
}
