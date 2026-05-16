//! `get_house_list` parity vs cubiomes' `getHouseList`. Checks the
//! 9-entry house counts AND the post-call RNG seed.

#![allow(clippy::missing_panics_doc, clippy::doc_markdown)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable, cast_slice};
use cubioxides::finder::village_houses::{HOUSE_NUM, get_house_list};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Header {
    magic: [u8; 4],
    format_version: u16,
    kind: u16,
    record_count: u64,
    reserved: [u64; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct GetHouseListRecord {
    seed: u64,
    chunk_x: i32,
    chunk_z: i32,
    houses: [i32; HOUSE_NUM],
    padding: i32,
    rng_final: u64,
}

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 81;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("get_house_list.bin")
}

#[test]
fn get_house_list_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[GetHouseListRecord] = cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    for (i, r) in recs.iter().enumerate() {
        let (houses, rng_final) = get_house_list(r.seed, r.chunk_x, r.chunk_z);
        assert_eq!(
            houses, r.houses,
            "record {i} (seed={:#x}, cx={}, cz={}): houses mismatch — rust {:?} vs cubiomes {:?}",
            r.seed, r.chunk_x, r.chunk_z, houses, r.houses
        );
        assert_eq!(
            rng_final, r.rng_final,
            "record {i} (seed={:#x}, cx={}, cz={}): rng_final mismatch — rust {:#x} vs cubiomes {:#x}",
            r.seed, r.chunk_x, r.chunk_z, rng_final, r.rng_final
        );
    }
}
