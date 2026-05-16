//! `is_viable_end_city_terrain` parity vs cubiomes'
//! `isViableEndCityTerrain`. Cross-checks the rotation-aware
//! surface-height check used to gate End City placement.

#![allow(clippy::missing_panics_doc, clippy::doc_markdown)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable, cast_slice};
use cubioxides::biomenoise::surface::SurfaceNoise;
use cubioxides::finder::viability::is_viable_end_city_terrain;
use cubioxides::generator::Generator;
use cubioxides::mc_version::{Dimension, MCVersion};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Header {
    magic: [u8; 4],
    format_version: u16,
    kind: u16,
    record_count: u64,
    reserved: [u64; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct ViableEndCityTerrainRecord {
    mc: i32,
    padding: i32,
    seed: u64,
    x: i32,
    z: i32,
    height: i32,
    padding2: i32,
}

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 84;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("viable_end_city_terrain.bin")
}

fn mc_from_ord(o: i32) -> MCVersion {
    match o {
        12 => MCVersion::V1_9,
        17 => MCVersion::V1_14,
        22 => MCVersion::V1_18,
        28 => MCVersion::V1_21,
        _ => panic!("unsupported mc ord {o}"),
    }
}

#[test]
fn is_viable_end_city_terrain_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[ViableEndCityTerrainRecord] = cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    for (i, r) in recs.iter().enumerate() {
        let mc = mc_from_ord(r.mc);
        let mut g = Generator::new(mc, 0);
        g.apply_seed(Dimension::End, r.seed);
        let sn = SurfaceNoise::init(Dimension::End, r.seed);
        let got = is_viable_end_city_terrain(&g, &sn, r.x, r.z).unwrap_or(0);
        assert_eq!(
            got, r.height,
            "record {i} (mc={:?}, seed={:#x}, x={}, z={}): height mismatch — rust {} vs cubiomes {}",
            mc, r.seed, r.x, r.z, got, r.height
        );
    }
}
