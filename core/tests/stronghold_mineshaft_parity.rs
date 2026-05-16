//! Parity tests: `init_first_stronghold` (kind = 51) +
//! `get_mineshafts` (kind = 52).

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::finder::{Pos, get_mineshafts, init_first_stronghold};
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
struct StrongholdInitRecord {
    mc: i32,
    pad: i32,
    seed: u64,
    first_x: i32,
    first_z: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct MineshaftRecord {
    mc: i32,
    cx0: i32,
    cz0: i32,
    cx1: i32,
    cz1: i32,
    count: i32,
    digest: u32,
    pad0: u32,
    seed: u64,
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
        3 => MCVersion::V1_0,
        10 => MCVersion::V1_7,
        12 => MCVersion::V1_9,
        15 => MCVersion::V1_12,
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
fn init_first_stronghold_matches_cubiomes() {
    let bytes = fixture("stronghold_init.bin");
    let recs: &[StrongholdInitRecord] = split(&bytes, 51);
    for (i, r) in recs.iter().enumerate() {
        let mc = mc_from_ord(r.mc);
        let (p, _) = init_first_stronghold(mc, r.seed);
        assert_eq!(
            p,
            Pos {
                x: r.first_x,
                z: r.first_z
            },
            "mismatch at {i} (mc={:?}, seed={:#x})",
            mc,
            r.seed
        );
    }
}

#[test]
fn get_mineshafts_matches_cubiomes() {
    let bytes = fixture("mineshaft.bin");
    let recs: &[MineshaftRecord] = split(&bytes, 52);
    for (i, r) in recs.iter().enumerate() {
        let mc = mc_from_ord(r.mc);
        let positions = get_mineshafts(mc, r.seed, r.cx0, r.cz0, r.cx1, r.cz1, 4096);
        assert_eq!(
            positions.len() as i32,
            r.count,
            "mineshaft count mismatch at {i} (mc={:?}, seed={:#x}, rect=({},{})..({},{}))",
            mc,
            r.seed,
            r.cx0,
            r.cz0,
            r.cx1,
            r.cz1
        );
        let mut digest: u32 = 0;
        for p in &positions {
            digest ^= hash32(p.x as u32);
            digest ^= hash32(p.z as u32);
        }
        assert_eq!(
            digest, r.digest,
            "mineshaft digest mismatch at {i} (mc={:?}, seed={:#x}, count={})",
            mc, r.seed, r.count
        );
    }
}
