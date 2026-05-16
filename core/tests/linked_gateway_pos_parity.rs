//! Parity test for `get_linked_gateway_pos` vs cubiomes'
//! `getLinkedGatewayPos`. Covers the three dispatch paths
//! (≤1.16 full search, 1.17+ trivial corner, 1.18+ modern).

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::biomenoise::end::EndNoise;
use cubioxides::biomenoise::surface::SurfaceNoise;
use cubioxides::finder::{Pos, get_linked_gateway_pos};
use cubioxides::mc_version::{Dimension, MCVersion};

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 70;

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
struct LinkedGatewayPosRecord {
    mc: i32,
    src_x: i32,
    src_z: i32,
    dst_x: i32,
    dst_z: i32,
    pad: i32,
    seed: u64,
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("linked_gateway_pos.bin")
}

fn mc_from_ord(ord: i32) -> MCVersion {
    match ord {
        17 => MCVersion::V1_14,
        20 => MCVersion::V1_16,
        22 => MCVersion::V1_18,
        28 => MCVersion::V1_21,
        other => panic!("unsupported MC ordinal: {other}"),
    }
}

#[test]
fn linked_gateway_pos_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[LinkedGatewayPosRecord] = bytemuck::cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    for (i, r) in recs.iter().enumerate() {
        let mc = mc_from_ord(r.mc);
        let en = EndNoise::set_seed(mc, r.seed);
        let sn = SurfaceNoise::init(Dimension::End, r.seed);
        let src = Pos {
            x: r.src_x,
            z: r.src_z,
        };
        let dst = get_linked_gateway_pos(&en, &sn, r.seed, src);
        assert!(
            dst.x == r.dst_x && dst.z == r.dst_z,
            "linked_gateway_pos mismatch at record {i} (mc={mc:?}, seed={:#x}, src=({}, {})): got ({}, {}), want ({}, {})",
            r.seed,
            r.src_x,
            r.src_z,
            dst.x,
            dst.z,
            r.dst_x,
            r.dst_z,
        );
    }
}
