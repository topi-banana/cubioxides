//! Parity test: cubioxides' `get_structure_pos` vs cubiomes'
//! `getStructurePos`. Reads the binary fixture (kind = 48) and
//! compares both the `valid` flag and the in-region attempt
//! position for 2048 random `(structure_type, mc, seed, reg_x,
//! reg_z)` tuples spanning every supported structure type.

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::finder::{Pos, StructureType, get_structure_pos};
use cubioxides::mc_version::MCVersion;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 48;

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
struct StructurePosRecord {
    structure_type: i32,
    mc: i32,
    seed: u64,
    reg_x: i32,
    reg_z: i32,
    pos_x: i32,
    pos_z: i32,
    valid: i32,
    pad: i32,
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("fixtures")
        .join("layers")
        .join("structure_pos.bin")
}

fn mc_from_ord(ord: i32) -> MCVersion {
    match ord {
        2 => MCVersion::B1_8,
        3 => MCVersion::V1_0,
        10 => MCVersion::V1_7,
        15 => MCVersion::V1_12,
        19 => MCVersion::V1_16_1,
        22 => MCVersion::V1_18,
        25 => MCVersion::V1_20,
        28 => MCVersion::V1_21,
        other => panic!("unsupported MC ordinal: {other}"),
    }
}

#[test]
fn structure_pos_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (header_bytes, body_bytes) = bytes.split_at(std::mem::size_of::<Header>());
    let header: &Header = bytemuck::from_bytes(header_bytes);
    assert_eq!(header.magic, MAGIC);
    assert_eq!(header.format_version, FORMAT_VERSION);
    assert_eq!(header.kind, KIND);
    let records: &[StructurePosRecord] = bytemuck::cast_slice(body_bytes);
    assert_eq!(records.len() as u64, header.record_count);

    for (i, rec) in records.iter().enumerate() {
        let ty = StructureType::from_ord(rec.structure_type)
            .unwrap_or_else(|| panic!("unsupported structure type ord {}", rec.structure_type));
        let mc = mc_from_ord(rec.mc);
        let got = get_structure_pos(ty, mc, rec.seed, rec.reg_x, rec.reg_z);
        let want = if rec.valid != 0 {
            Some(Pos {
                x: rec.pos_x,
                z: rec.pos_z,
            })
        } else {
            None
        };
        assert_eq!(
            got, want,
            "structure_pos mismatch at {i} (ty={:?}, mc={:?}, seed={:#x}, reg=({}, {}))",
            ty, mc, rec.seed, rec.reg_x, rec.reg_z
        );
    }
}
