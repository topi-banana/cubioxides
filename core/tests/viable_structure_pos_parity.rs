//! Parity test for the Nether and End branches of
//! `is_viable_structure_pos` vs cubiomes' `isViableStructurePos`.
//! Overworld coverage requires the `mapViableBiome` layer-hook
//! machinery and is deferred to a follow-up stage.

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::finder::{StructureType, is_viable_structure_pos};
use cubioxides::generator::Generator;
use cubioxides::mc_version::{Dimension, MCVersion};

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 67;

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
struct ViableStructurePosRecord {
    mc: i32,
    dim: i32,
    structure_type: i32,
    viable: i32,
    seed: u64,
    x: i32,
    z: i32,
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("viable_structure_pos.bin")
}

fn mc_from_ord(ord: i32) -> MCVersion {
    match ord {
        10 => MCVersion::V1_7,
        15 => MCVersion::V1_12,
        17 => MCVersion::V1_14,
        18 => MCVersion::V1_15,
        19 => MCVersion::V1_16_1,
        21 => MCVersion::V1_17,
        22 => MCVersion::V1_18,
        23 => MCVersion::V1_19_2,
        26 => MCVersion::V1_21_1,
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

#[test]
fn viable_structure_pos_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[ViableStructurePosRecord] = bytemuck::cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    for (i, r) in recs.iter().enumerate() {
        let mc = mc_from_ord(r.mc);
        let dim = dim_from_ord(r.dim);
        let sty = StructureType::from_ord(r.structure_type)
            .unwrap_or_else(|| panic!("unknown structure type ord {}", r.structure_type));
        let mut g = Generator::new(mc, 0);
        g.apply_seed(dim, r.seed);
        let got = is_viable_structure_pos(sty, &g, r.x, r.z, 0);
        let want = r.viable != 0;
        assert!(
            got == want,
            "viable_structure_pos mismatch at record {i} (mc={mc:?}, dim={dim:?}, struct={sty:?}, seed={:#x}, x={}, z={}): got {}, want {}",
            r.seed,
            r.x,
            r.z,
            got,
            want,
        );
    }
}
