//! Parity test for `search_all_48` vs cubiomes' `searchAll48Thread`
//! inner-loop. Sequential `Swamp_Hut` + radius=128 enumeration
//! across small 48-bit seed windows.

#![allow(clippy::missing_panics_doc, clippy::needless_range_loop)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::finder::{
    LOW20_QUAD_IDEAL, StructureType, get_structure_config, is_quad_base, search_all_48,
};
use cubioxides::mc_version::MCVersion;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 72;

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
struct SearchAll48Record {
    start: u64,
    end: u64,
    cnt: i32,
    pad: i32,
    seeds: [u64; 8],
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("search_all_48.bin")
}

#[test]
fn search_all_48_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[SearchAll48Record] = bytemuck::cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    let sconf =
        get_structure_config(StructureType::SwampHut, MCVersion::V1_18).expect("Swamp_Hut config");
    for (i, r) in recs.iter().enumerate() {
        let seeds = search_all_48(r.start..=r.end, LOW20_QUAD_IDEAL, 20, |s| {
            is_quad_base(sconf, s, 128).is_some()
        });
        // Cubiomes capped at 8; compare prefix.
        let cnt = seeds.len().min(8) as i32;
        assert_eq!(
            cnt, r.cnt,
            "search_all_48 cnt mismatch at record {i} (start={:#x}, end={:#x}): got {}, want {}",
            r.start, r.end, cnt, r.cnt,
        );
        for k in 0..(cnt as usize) {
            assert_eq!(
                seeds[k], r.seeds[k],
                "search_all_48 seed[{k}] mismatch at record {i}",
            );
        }
    }
}
