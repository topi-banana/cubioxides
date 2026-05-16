//! Parity test for `get_fixed_end_gateways` vs cubiomes'
//! `getFixedEndGateways`. 192 records (3 MC versions × 64 random
//! seeds).

#![allow(clippy::missing_panics_doc, clippy::needless_range_loop)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::finder::{Pos, get_fixed_end_gateways};
use cubioxides::mc_version::MCVersion;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 69;

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
struct FixedEndGatewaysRecord {
    mc: i32,
    pad: i32,
    seed: u64,
    xs: [i32; 20],
    zs: [i32; 20],
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("fixed_end_gateways.bin")
}

fn mc_from_ord(ord: i32) -> MCVersion {
    match ord {
        17 => MCVersion::V1_14,
        22 => MCVersion::V1_18,
        28 => MCVersion::V1_21,
        other => panic!("unsupported MC ordinal: {other}"),
    }
}

#[test]
fn fixed_end_gateways_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[FixedEndGatewaysRecord] = bytemuck::cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    for (i, r) in recs.iter().enumerate() {
        let mc = mc_from_ord(r.mc);
        let mut src = [Pos::default(); 20];
        get_fixed_end_gateways(mc, r.seed, &mut src);
        for k in 0..20 {
            assert!(
                src[k].x == r.xs[k] && src[k].z == r.zs[k],
                "fixed_end_gateways[{k}] mismatch at record {i} (mc={mc:?}, seed={:#x}): got ({}, {}), want ({}, {})",
                r.seed,
                src[k].x,
                src[k].z,
                r.xs[k],
                r.zs[k],
            );
        }
    }
}
