//! Parity tests for `get_population_seed` and `get_end_islands` vs
//! cubiomes' `getPopulationSeed` / `getEndIslands`.

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::finder::{EndIsland, get_end_islands, get_population_seed};
use cubioxides::mc_version::MCVersion;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND_POPULATION_SEED: u16 = 56;
const KIND_END_ISLANDS: u16 = 57;

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
struct PopulationSeedRecord {
    mc: i32,
    x: i32,
    z: i32,
    pad: i32,
    ws: u64,
    pop_seed: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct EndIslandsRecord {
    mc: i32,
    chunk_x: i32,
    chunk_z: i32,
    n: i32,
    seed: u64,
    islands: [i32; 8],
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join(name)
}

fn mc_from_ord(ord: i32) -> MCVersion {
    match ord {
        8 => MCVersion::V1_12,
        17 => MCVersion::V1_13,
        20 => MCVersion::V1_16,
        21 => MCVersion::V1_17,
        22 => MCVersion::V1_18,
        28 => MCVersion::V1_21,
        other => panic!("unsupported MC ordinal: {other}"),
    }
}

fn read_fixture(name: &str, kind: u16) -> (Header, Vec<u8>) {
    let path = fixture_path(name);
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: Header = *bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, kind);
    (h, body.to_vec())
}

#[test]
fn population_seed_matches_cubiomes() {
    let (h, body) = read_fixture("population_seed.bin", KIND_POPULATION_SEED);
    let recs: &[PopulationSeedRecord] = bytemuck::cast_slice(&body);
    assert_eq!(recs.len() as u64, h.record_count);

    for (i, r) in recs.iter().enumerate() {
        let mc = mc_from_ord(r.mc);
        let got = get_population_seed(mc, r.ws, r.x, r.z);
        assert!(
            got == r.pop_seed,
            "population_seed mismatch at record {i} (mc={mc:?}, ws={:#x}, x={}, z={}): got {:#x}, want {:#x}",
            r.ws,
            r.x,
            r.z,
            got,
            r.pop_seed
        );
    }
}

#[test]
fn end_islands_matches_cubiomes() {
    let (h, body) = read_fixture("end_islands.bin", KIND_END_ISLANDS);
    let recs: &[EndIslandsRecord] = bytemuck::cast_slice(&body);
    assert_eq!(recs.len() as u64, h.record_count);

    for (i, r) in recs.iter().enumerate() {
        let mc = mc_from_ord(r.mc);
        let mut islands = [EndIsland::default(); 2];
        let n = get_end_islands(&mut islands, mc, r.seed, r.chunk_x, r.chunk_z);
        assert!(
            n as i32 == r.n,
            "end_islands count mismatch at record {i} (mc={mc:?}, seed={:#x}, cx={}, cz={}): got {}, want {}",
            r.seed,
            r.chunk_x,
            r.chunk_z,
            n,
            r.n
        );
        for (k, island) in islands.iter().enumerate().take(n) {
            let want = [
                r.islands[k * 4],
                r.islands[k * 4 + 1],
                r.islands[k * 4 + 2],
                r.islands[k * 4 + 3],
            ];
            let got = [island.x, island.y, island.z, island.r];
            assert!(
                got == want,
                "end_islands[{k}] mismatch at record {i} (mc={mc:?}, seed={:#x}, cx={}, cz={}): got {:?}, want {:?}",
                r.seed,
                r.chunk_x,
                r.chunk_z,
                got,
                want
            );
        }
    }
}
