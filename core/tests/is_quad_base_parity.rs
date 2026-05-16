//! Parity test for `is_quad_base` dispatcher vs cubiomes'
//! `isQuadBase`. Covers `Swamp_Hut` radius=128 (fast path),
//! `Swamp_Hut` radius=160 (generic path), `Outpost`,
//! `Desert_Pyramid`, `Ocean_Ruin`.

#![allow(clippy::missing_panics_doc, clippy::float_cmp)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::finder::{StructureType, get_structure_config, is_quad_base};
use cubioxides::mc_version::MCVersion;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 73;

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
struct IsQuadBaseRecord {
    mc: i32,
    sty: i32,
    radius: i32,
    hit: i32,
    seed: u64,
    sqrad_bits: u32,
    pad: u32,
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("is_quad_base.bin")
}

fn mc_from_ord(ord: i32) -> MCVersion {
    match ord {
        22 => MCVersion::V1_18,
        other => panic!("unsupported MC ordinal: {other}"),
    }
}

#[test]
fn is_quad_base_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[IsQuadBaseRecord] = bytemuck::cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    for (i, r) in recs.iter().enumerate() {
        let mc = mc_from_ord(r.mc);
        let sty = StructureType::from_ord(r.sty).unwrap();
        let sconf = get_structure_config(sty, mc).expect("structure config");
        let got = is_quad_base(sconf, r.seed, r.radius);
        let want_hit = r.hit != 0;
        match (got, want_hit) {
            (Some(rad), true) => {
                assert!(
                    rad.to_bits() == r.sqrad_bits,
                    "is_quad_base sqrad mismatch at record {i} (mc={mc:?}, sty={sty:?}, r={}, seed={:#x}): got {} ({:#x}), want {:#x}",
                    r.radius,
                    r.seed,
                    rad,
                    rad.to_bits(),
                    r.sqrad_bits,
                );
            }
            (None, false) => {}
            (got, want) => panic!(
                "is_quad_base hit/miss mismatch at record {i} (mc={mc:?}, sty={sty:?}, r={}, seed={:#x}): got {:?}, want hit={}",
                r.radius, r.seed, got, want,
            ),
        }
    }
}
