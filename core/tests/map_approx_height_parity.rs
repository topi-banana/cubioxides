//! Parity test for `map_approx_height` vs cubiomes'
//! `mapApproxHeight`. Covers all 4 dispatch branches:
//! - Overworld legacy (1.0-1.17)
//! - Overworld 1.18+ (`BiomeNoise` `NP_DEPTH`)
//! - End (delegates to `map_end_surface_height` at scale 4)
//! - Nether (returns 127, `y` unwritten)

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::biome::Biome;
use cubioxides::biomenoise::surface::SurfaceNoise;
use cubioxides::generator::{Generator, map_approx_height};
use cubioxides::mc_version::{Dimension, MCVersion};

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 64;

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
struct MapApproxHeightRecord {
    mc: i32,
    dim: i32,
    x: i32,
    z: i32,
    w: i32,
    h: i32,
    rc: i32,
    pad: i32,
    seed: u64,
    y_min_bits: u32,
    y_max_bits: u32,
    y_digest: u32,
    ids_digest: u32,
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("map_approx_height.bin")
}

fn mc_from_ord(ord: i32) -> MCVersion {
    match ord {
        10 => MCVersion::V1_7,
        17 => MCVersion::V1_13,
        19 => MCVersion::V1_16_1,
        22 => MCVersion::V1_18,
        28 => MCVersion::V1_21,
        other => panic!("unsupported MC ordinal: {other}"),
    }
}

fn dim_from_ord(ord: i32) -> Dimension {
    match ord {
        -1 => Dimension::Nether,
        0 => Dimension::Overworld,
        1 => Dimension::End,
        other => panic!("unsupported dim ordinal: {other}"),
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
fn map_approx_height_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[MapApproxHeightRecord] = bytemuck::cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    for (i, r) in recs.iter().enumerate() {
        let mc = mc_from_ord(r.mc);
        let dim = dim_from_ord(r.dim);
        let mut g = Generator::new(mc, 0);
        g.apply_seed(dim, r.seed);
        let sn = SurfaceNoise::init(dim, r.seed);

        let mut y = vec![0.0_f32; (r.w * r.h) as usize];
        let mut ids = vec![Biome::default(); (r.w * r.h) as usize];
        let rc = map_approx_height(&mut y, Some(&mut ids), &g, &sn, r.x, r.z, r.w, r.h);

        assert!(
            rc == r.rc,
            "rc mismatch at record {i} (mc={mc:?}, dim={dim:?}): got {rc}, want {}",
            r.rc
        );
        if r.rc != 0 {
            // 127 (Nether) or 1 (pre-1.9 End): y unwritten, skip.
            continue;
        }

        let mut y_min = f32::INFINITY;
        let mut y_max = f32::NEG_INFINITY;
        let mut y_digest: u32 = 0;
        for &v in &y {
            if v < y_min {
                y_min = v;
            }
            if v > y_max {
                y_max = v;
            }
            y_digest = hash32(y_digest.wrapping_add(v.to_bits()));
        }
        let mut ids_digest: u32 = 0;
        for &id in &ids {
            ids_digest = hash32(ids_digest.wrapping_add(id.0 as u32));
        }
        assert!(
            y_min.to_bits() == r.y_min_bits
                && y_max.to_bits() == r.y_max_bits
                && y_digest == r.y_digest
                && ids_digest == r.ids_digest,
            "map_approx_height mismatch at record {i} (mc={mc:?}, dim={dim:?}, seed={:#x}, x={}, z={}): got (y_min={:#x}, y_max={:#x}, y_digest={:#x}, ids_digest={:#x}), want (y_min={:#x}, y_max={:#x}, y_digest={:#x}, ids_digest={:#x})",
            r.seed,
            r.x,
            r.z,
            y_min.to_bits(),
            y_max.to_bits(),
            y_digest,
            ids_digest,
            r.y_min_bits,
            r.y_max_bits,
            r.y_digest,
            r.ids_digest,
        );
    }
}
