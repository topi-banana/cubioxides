//! Parity test for `is_end_chunk_empty` vs cubiomes'
//! `isEndChunkEmpty`. Compares the boolean (empty / not-empty)
//! verdict across 240 random chunks.

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::biomenoise::end::EndNoise;
use cubioxides::biomenoise::surface::SurfaceNoise;
use cubioxides::finder::is_end_chunk_empty;
use cubioxides::mc_version::{Dimension, MCVersion};

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 61;

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
struct EndChunkEmptyRecord {
    mc: i32,
    chunk_x: i32,
    chunk_z: i32,
    empty: i32,
    seed: u64,
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("end_chunk_empty.bin")
}

fn mc_from_ord(ord: i32) -> MCVersion {
    match ord {
        17 => MCVersion::V1_13,
        22 => MCVersion::V1_18,
        28 => MCVersion::V1_21,
        other => panic!("unsupported MC ordinal: {other}"),
    }
}

#[test]
fn is_end_chunk_empty_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[EndChunkEmptyRecord] = bytemuck::cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    for (i, r) in recs.iter().enumerate() {
        let mc = mc_from_ord(r.mc);
        let en = EndNoise::set_seed(mc, r.seed);
        let sn = SurfaceNoise::init(Dimension::End, r.seed);
        let got = is_end_chunk_empty(&en, &sn, r.seed, r.chunk_x, r.chunk_z);
        let want = r.empty != 0;
        assert!(
            got == want,
            "end_chunk_empty mismatch at record {i} (mc={mc:?}, seed={:#x}, cx={}, cz={}): got {}, want {}",
            r.seed,
            r.chunk_x,
            r.chunk_z,
            got,
            want
        );
    }
}
