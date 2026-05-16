//! `BiomeSet::add` / `BiomeSet::contains` parity vs cubiomes'
//! `idSetAdd` / `idSetTest` static-inline helpers. Cross-checks
//! 200 biome IDs against 4 sample mask pairs.

#![allow(clippy::missing_panics_doc, clippy::doc_markdown)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable, cast_slice};
use cubioxides::biome_set::BiomeSet;

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
struct IdSetRecord {
    id: i32,
    padding: i32,
    added_m_l: u64,
    added_m_m: u64,
    test_m_l: u64,
    test_m_m: u64,
    test_result: i32,
    padding2: i32,
}

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 88;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("id_set.bin")
}

#[test]
fn id_set_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[IdSetRecord] = cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    for (i, r) in recs.iter().enumerate() {
        // Test add: empty set + add(id) should give cubiomes' bitfield.
        let mut s = BiomeSet::new();
        s.add(r.id);
        assert_eq!(
            (s.m_low, s.m_mut),
            (r.added_m_l, r.added_m_m),
            "record {i} (id={}): add mismatch — rust ({:#x}, {:#x}) vs cubiomes ({:#x}, {:#x})",
            r.id,
            s.m_low,
            s.m_mut,
            r.added_m_l,
            r.added_m_m
        );
        // Test contains.
        let s = BiomeSet {
            m_low: r.test_m_l,
            m_mut: r.test_m_m,
        };
        let got = i32::from(s.contains(r.id));
        assert_eq!(
            got, r.test_result,
            "record {i} (id={}, mL={:#x}, mM={:#x}): contains mismatch — rust {} vs cubiomes {}",
            r.id, r.test_m_l, r.test_m_m, got, r.test_result
        );
    }
}
