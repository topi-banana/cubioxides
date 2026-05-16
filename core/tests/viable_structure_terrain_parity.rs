//! `is_viable_structure_terrain` parity vs cubiomes' `isViableStructureTerrain`.
//! Validates the 1.18+ depth-gate for Desert Pyramid / Jungle Temple /
//! Mansion at four-corner sample points.

#![allow(clippy::missing_panics_doc, clippy::doc_markdown)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable, cast_slice};
use cubioxides::finder::StructureType;
use cubioxides::finder::viability::is_viable_structure_terrain;
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
struct ViableTerrainRecord {
    mc: i32,
    structure_type: i32,
    seed: u64,
    x: i32,
    z: i32,
    viable: i32,
    padding: i32,
}

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 83;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("viable_structure_terrain.bin")
}

fn mc_from_ord(o: i32) -> MCVersion {
    match o {
        10 => MCVersion::V1_7,
        22 => MCVersion::V1_18,
        28 => MCVersion::V1_21,
        _ => panic!("unsupported mc ord {o}"),
    }
}

fn sty_from_ord(o: i32) -> StructureType {
    match o {
        1 => StructureType::DesertPyramid,
        2 => StructureType::JungleTemple,
        9 => StructureType::Mansion,
        _ => panic!("unsupported structure type {o}"),
    }
}

#[test]
fn is_viable_structure_terrain_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[ViableTerrainRecord] = cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    for (i, r) in recs.iter().enumerate() {
        let mc = mc_from_ord(r.mc);
        let sty = sty_from_ord(r.structure_type);
        let mut g = Generator::new(mc, 0);
        g.apply_seed(Dimension::Overworld, r.seed);
        let got = i32::from(is_viable_structure_terrain(sty, &g, r.x, r.z));
        assert_eq!(
            got, r.viable,
            "record {i} (mc={:?}, sty={:?}, seed={:#x}, x={}, z={}): viable mismatch — rust {} vs cubiomes {}",
            mc, sty, r.seed, r.x, r.z, got, r.viable
        );
    }
}
