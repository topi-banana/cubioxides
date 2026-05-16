//! Parity test: `estimate_spawn` vs cubiomes' `estimateSpawn`.
//! Three branches exercised: Beta returns origin, layered uses
//! biome search, modern (1.18+) runs the fittest-position spiral.

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::finder::estimate_spawn;
use cubioxides::generator::Generator;
use cubioxides::mc_version::{Dimension, MCVersion};

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 55;

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
struct EstimateSpawnRecord {
    mc: i32,
    pad: i32,
    seed: u64,
    spawn_x: i32,
    spawn_z: i32,
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("estimate_spawn.bin")
}

fn mc_from_ord(ord: i32) -> MCVersion {
    match ord {
        3 => MCVersion::V1_0,
        10 => MCVersion::V1_7,
        15 => MCVersion::V1_12,
        22 => MCVersion::V1_18,
        28 => MCVersion::V1_21,
        other => panic!("unsupported MC ordinal: {other}"),
    }
}

#[test]
fn estimate_spawn_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[EstimateSpawnRecord] = bytemuck::cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    for (i, r) in recs.iter().enumerate() {
        let mc = mc_from_ord(r.mc);
        let mut g = Generator::new(mc, 0);
        g.apply_seed(Dimension::Overworld, r.seed);
        let got = estimate_spawn(&g, None);
        assert!(
            got.x == r.spawn_x && got.z == r.spawn_z,
            "spawn mismatch at record {i} (mc={mc:?}, seed={:#x}): got ({}, {}), want ({}, {})",
            r.seed,
            got.x,
            got.z,
            r.spawn_x,
            r.spawn_z
        );
    }
}
