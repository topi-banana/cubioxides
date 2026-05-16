//! `Biome::name` parity vs cubiomes' `biome2str`. Covers MC 1.13,
//! 1.18, 1.21 across biome IDs 0..=200.

#![allow(clippy::missing_panics_doc, clippy::doc_markdown)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable, cast_slice};
use cubioxides::biome::Biome;
use cubioxides::mc_version::MCVersion;

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
struct Biome2StrRecord {
    mc: i32,
    id: i32,
    name_len: i32,
    has_name: i32,
    name: [u8; 32],
}

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 90;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("biome2str.bin")
}

fn mc_from_ord(o: i32) -> MCVersion {
    match o {
        16 => MCVersion::V1_13,
        22 => MCVersion::V1_18,
        28 => MCVersion::V1_21,
        _ => panic!("unsupported mc ord {o}"),
    }
}

#[test]
fn biome2str_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[Biome2StrRecord] = cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    for r in recs {
        let mc = mc_from_ord(r.mc);
        let got = Biome::name(mc, r.id);
        if r.has_name == 0 {
            assert!(
                got.is_none(),
                "id={}, mc={:?}: rust returned {:?}, cubiomes returned NULL",
                r.id,
                mc,
                got
            );
        } else {
            let got_name = got.unwrap_or_else(|| {
                panic!(
                    "id={}, mc={:?}: rust returned None, cubiomes returned non-NULL",
                    r.id, mc
                )
            });
            let len = r.name_len as usize;
            let cubiomes_name =
                std::str::from_utf8(&r.name[..len.min(32)]).expect("cubiomes name should be UTF-8");
            assert_eq!(
                got_name, cubiomes_name,
                "id={}, mc={:?}: rust {:?} vs cubiomes {:?}",
                r.id, mc, got_name, cubiomes_name
            );
        }
    }
}
