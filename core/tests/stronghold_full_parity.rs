//! Biome-checked stronghold iteration parity. For each `(mc,
//! seed)` pair we build a `Generator`, apply the seed, run three
//! `next_stronghold` calls, and compare the resulting `(x, z)`
//! pairs with cubiomes' end-to-end `setupGenerator + applySeed +
//! initFirstStronghold + nextStronghold ×3` flow.

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::finder::{init_first_stronghold, next_stronghold};
use cubioxides::generator::Generator;
use cubioxides::mc_version::{Dimension, MCVersion};

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 54;

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
struct StrongholdFullRecord {
    mc: i32,
    pad: i32,
    seed: u64,
    pos_xz: [i32; 6],
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("stronghold_full.bin")
}

fn mc_from_ord(ord: i32) -> MCVersion {
    match ord {
        10 => MCVersion::V1_7,
        15 => MCVersion::V1_12,
        22 => MCVersion::V1_18,
        28 => MCVersion::V1_21,
        other => panic!("unsupported MC ordinal: {other}"),
    }
}

#[test]
fn stronghold_iteration_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[StrongholdFullRecord] = bytemuck::cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    for (i, r) in recs.iter().enumerate() {
        let mc = mc_from_ord(r.mc);
        let mut g = Generator::new(mc, 0);
        g.apply_seed(Dimension::Overworld, r.seed);
        let (_first_approx, mut sh) = init_first_stronghold(mc, r.seed);
        for step in 0..3 {
            next_stronghold(&mut sh, &g);
            assert_eq!(
                sh.pos.x,
                r.pos_xz[step * 2],
                "stronghold #{step} x mismatch at record {i} (mc={:?}, seed={:#x})",
                mc,
                r.seed
            );
            assert_eq!(
                sh.pos.z,
                r.pos_xz[step * 2 + 1],
                "stronghold #{step} z mismatch at record {i} (mc={:?}, seed={:#x})",
                mc,
                r.seed
            );
        }
    }
}
