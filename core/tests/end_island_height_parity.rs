//! Parity test for `map_end_island_height` vs cubiomes'
//! `mapEndIslandHeight`. Fixture stores `(min, max, digest)` of the
//! per-record height grid — full grids would inflate the fixture.

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::biomenoise::end::EndNoise;
use cubioxides::finder::map_end_island_height;
use cubioxides::mc_version::MCVersion;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 58;

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
struct EndIslandHeightRecord {
    mc: i32,
    x: i32,
    z: i32,
    w: i32,
    h: i32,
    scale: i32,
    seed: u64,
    y_min_bits: u32,
    y_max_bits: u32,
    digest: u32,
    pad: u32,
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("end_island_height.bin")
}

fn mc_from_ord(ord: i32) -> MCVersion {
    match ord {
        17 => MCVersion::V1_13,
        20 => MCVersion::V1_16,
        22 => MCVersion::V1_18,
        28 => MCVersion::V1_21,
        other => panic!("unsupported MC ordinal: {other}"),
    }
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
fn map_end_island_height_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[EndIslandHeightRecord] = bytemuck::cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    for (i, r) in recs.iter().enumerate() {
        let mc = mc_from_ord(r.mc);
        let en = EndNoise::set_seed(mc, r.seed);
        let mut y = vec![0.0_f32; (r.w * r.h) as usize];
        map_end_island_height(&mut y, &en, r.seed, r.x, r.z, r.w, r.h, r.scale);

        let mut y_min = f32::INFINITY;
        let mut y_max = f32::NEG_INFINITY;
        let mut digest: u32 = 0;
        for &v in &y {
            if v < y_min {
                y_min = v;
            }
            if v > y_max {
                y_max = v;
            }
            digest = hash32(digest.wrapping_add(v.to_bits()));
        }
        assert!(
            y_min.to_bits() == r.y_min_bits
                && y_max.to_bits() == r.y_max_bits
                && digest == r.digest,
            "end_island_height mismatch at record {i} (mc={mc:?}, seed={:#x}, x={}, z={}, w={}, h={}, scale={}): got (min={}/{:#x}, max={}/{:#x}, digest={:#x}), want (min={:#x}, max={:#x}, digest={:#x})",
            r.seed,
            r.x,
            r.z,
            r.w,
            r.h,
            r.scale,
            y_min,
            y_min.to_bits(),
            y_max,
            y_max.to_bits(),
            digest,
            r.y_min_bits,
            r.y_max_bits,
            r.digest
        );
    }
}
