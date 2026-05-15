//! Parity test: cubioxides' `NetherNoise` vs cubiomes'
//! `setNetherSeed` / `getNetherBiome` / `mapNether2D`. Reads the
//! binary fixture (kind = 41) and compares both the per-cell
//! `(biome, ndel)` return and the XOR-folded digest of a small
//! `mapNether2D` grid.

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::biomenoise::NetherNoise;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 41;

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
struct NetherRecord {
    seed: u64,
    x: i32,
    y: i32,
    z: i32,
    w: u32,
    h: u32,
    single_biome: i32,
    single_ndel_bits: u32,
    grid_digest: u32,
    pad: [u32; 2],
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("fixtures")
        .join("noise")
        .join("nether.bin")
}

fn hash32(mut x: u32) -> u32 {
    x ^= x >> 15;
    x = x.wrapping_mul(0xd168_aaad);
    x ^= x >> 15;
    x = x.wrapping_mul(0xaf72_3597);
    x ^= x >> 15;
    x
}

#[test]
fn nether_noise_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (header_bytes, body_bytes) = bytes.split_at(std::mem::size_of::<Header>());
    let header: &Header = bytemuck::from_bytes(header_bytes);
    assert_eq!(header.magic, MAGIC);
    assert_eq!(header.format_version, FORMAT_VERSION);
    assert_eq!(header.kind, KIND);
    let records: &[NetherRecord] = bytemuck::cast_slice(body_bytes);
    assert_eq!(records.len() as u64, header.record_count);

    for (i, rec) in records.iter().enumerate() {
        let nn = NetherNoise::set_seed(rec.seed);

        // Single-cell sample.
        let (biome, ndel) = nn.get_nether_biome(rec.x, rec.y, rec.z);
        assert_eq!(
            biome.id(),
            rec.single_biome,
            "get_nether_biome biome mismatch at {i} (seed={:#x}, x={}, y={}, z={})",
            rec.seed,
            rec.x,
            rec.y,
            rec.z
        );
        assert_eq!(
            ndel.to_bits(),
            rec.single_ndel_bits,
            "get_nether_biome ndel mismatch at {i} (seed={:#x}, x={}, y={}, z={}): got {ndel:?}, want {:?}",
            rec.seed,
            rec.x,
            rec.y,
            rec.z,
            f32::from_bits(rec.single_ndel_bits)
        );

        // Grid digest.
        let w = rec.w as usize;
        let h = rec.h as usize;
        let mut out = vec![0i32; w * h];
        nn.map_nether_2d(&mut out, rec.x, rec.z, w, h);
        let mut digest: u32 = 0;
        for v in &out {
            digest ^= hash32(*v as u32);
        }
        assert_eq!(
            digest, rec.grid_digest,
            "map_nether_2d digest mismatch at {i} (seed={:#x}, x={}, z={}, w={}, h={})",
            rec.seed, rec.x, rec.z, rec.w, rec.h
        );
    }
}
