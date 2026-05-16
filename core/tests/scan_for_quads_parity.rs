//! Parity test for `scan_for_quads` (`Swamp_Hut` + radius=128 path)
//! vs cubiomes' `scanForQuads`. Covers the inner-loop pos/cnt path
//! across 64 random 48-bit seed windows.

#![allow(clippy::missing_panics_doc, clippy::needless_range_loop)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::finder::{
    LOW20_QUAD_IDEAL, Pos, StructureType, get_structure_config, scan_for_quads,
};
use cubioxides::mc_version::MCVersion;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 71;

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
struct ScanForQuadsRecord {
    x: i32,
    z: i32,
    w: i32,
    h: i32,
    cnt: i32,
    pad: i32,
    s48: u64,
    out_xz: [i32; 16],
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("scan_for_quads.bin")
}

#[test]
fn scan_for_quads_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[ScanForQuadsRecord] = bytemuck::cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    let sconf =
        get_structure_config(StructureType::SwampHut, MCVersion::V1_18).expect("Swamp_Hut config");
    let salt: u64 = sconf.salt as i64 as u64;

    for (i, r) in recs.iter().enumerate() {
        let mut qplist: Vec<Pos> = Vec::with_capacity(8);
        let cnt = scan_for_quads(
            sconf,
            128,
            r.s48,
            LOW20_QUAD_IDEAL,
            20,
            salt,
            r.x as i64,
            r.z as i64,
            r.w as i64,
            r.h as i64,
            &mut qplist,
            8,
        );
        assert_eq!(
            cnt as i32, r.cnt,
            "scan_for_quads cnt mismatch at record {i} (s48={:#x}, x={}, z={}, w={}, h={}): got {}, want {}",
            r.s48, r.x, r.z, r.w, r.h, cnt, r.cnt,
        );
        for k in 0..(cnt.min(8)) {
            assert_eq!(
                qplist[k].x,
                r.out_xz[k * 2],
                "scan_for_quads x[{k}] mismatch at record {i}",
            );
            assert_eq!(
                qplist[k].z,
                r.out_xz[k * 2 + 1],
                "scan_for_quads z[{k}] mismatch at record {i}",
            );
        }
    }
}
