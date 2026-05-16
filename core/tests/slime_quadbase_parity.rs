//! Parity tests for `is_slime_chunk` (kind = 49) and
//! `is_quad_base_feature_24*` / `get_quad_hut_cst` (kind = 50).

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::finder::{
    QuadHutCst, StructureType, get_quad_hut_cst, get_structure_config, is_quad_base_feature_24,
    is_quad_base_feature_24_classic, is_slime_chunk,
};
use cubioxides::mc_version::MCVersion;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;

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
struct SlimeRecord {
    seed: u64,
    cx: i32,
    cz: i32,
    is_slime: i32,
    pad: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct QuadbaseRecord {
    seed: u64,
    structure_type: i32,
    mc: i32,
    classic_radius_bits: u32,
    feature24_radius_bits: u32,
    cst: i32,
    low20: u32,
}

fn fixture(name: &str) -> Vec<u8> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join(name);
    let mut bytes = Vec::new();
    File::open(&p).unwrap().read_to_end(&mut bytes).unwrap();
    bytes
}

fn split<R: Pod>(bytes: &[u8], expected_kind: u16) -> &[R] {
    let (h, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, expected_kind);
    let r: &[R] = bytemuck::cast_slice(body);
    assert_eq!(r.len() as u64, h.record_count);
    r
}

fn mc_from_ord(ord: i32) -> MCVersion {
    match ord {
        15 => MCVersion::V1_12,
        19 => MCVersion::V1_16_1,
        22 => MCVersion::V1_18,
        other => panic!("unsupported MC ordinal: {other}"),
    }
}

#[test]
fn slime_chunks_match_cubiomes() {
    let bytes = fixture("slime_chunks.bin");
    let recs: &[SlimeRecord] = split(&bytes, 49);
    for (i, r) in recs.iter().enumerate() {
        let got = is_slime_chunk(r.seed, r.cx, r.cz);
        assert_eq!(
            got,
            r.is_slime != 0,
            "slime mismatch at {i} (seed={:#x}, cx={}, cz={})",
            r.seed,
            r.cx,
            r.cz
        );
    }
}

#[test]
fn quadbase_matches_cubiomes() {
    let bytes = fixture("quadbase.bin");
    let recs: &[QuadbaseRecord] = split(&bytes, 50);
    for (i, r) in recs.iter().enumerate() {
        let ty = StructureType::from_ord(r.structure_type).unwrap();
        let mc = mc_from_ord(r.mc);
        let sconf = get_structure_config(ty, mc).expect("config");

        // classic: cubiomes returns 1.0 if quad-classic, else 0.0
        let want_classic = f32::from_bits(r.classic_radius_bits) > 0.0;
        let got_classic = is_quad_base_feature_24_classic(sconf, r.seed);
        assert_eq!(
            got_classic, want_classic,
            "classic mismatch at {i} (seed={:#x}, ty={:?})",
            r.seed, ty
        );

        // feature24: cubiomes returns the radius in blocks, 0.0 for fail.
        let want_radius_bits = r.feature24_radius_bits;
        let want_radius = f32::from_bits(want_radius_bits);
        let got_radius = is_quad_base_feature_24(sconf, r.seed, 8, 8, 10);
        match (got_radius, want_radius == 0.0) {
            (None, true) => {}
            (Some(g), false) => assert_eq!(
                g.to_bits(),
                want_radius_bits,
                "feature24 radius bits mismatch at {i} (seed={:#x}, ty={:?}): got {g} want {want_radius}",
                r.seed,
                ty
            ),
            (Some(g), true) => panic!(
                "feature24 unexpectedly matched at {i} (seed={:#x}, ty={:?}, got radius {g})",
                r.seed, ty
            ),
            (None, false) => panic!(
                "feature24 unexpectedly failed at {i} (seed={:#x}, ty={:?}, want radius {want_radius})",
                r.seed, ty
            ),
        }

        // cst
        let want_cst = match r.cst {
            0 => QuadHutCst::None,
            1 => QuadHutCst::Ideal,
            2 => QuadHutCst::Classic,
            3 => QuadHutCst::Normal,
            4 => QuadHutCst::Barely,
            other => panic!("unknown cst {other}"),
        };
        let got_cst = get_quad_hut_cst(r.low20 as u64);
        assert_eq!(
            got_cst, want_cst,
            "cst mismatch at {i} (low20={:#x})",
            r.low20
        );
    }
}
