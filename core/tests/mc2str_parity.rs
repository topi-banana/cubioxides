//! `MCVersion::name` parity vs cubiomes' `mc2str`. Covers ords
//! 0..30 (Undef..V1_21 plus a few invalid ords for the "?" return).

#![allow(clippy::missing_panics_doc, clippy::doc_markdown)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable, cast_slice};
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
struct Mc2StrRecord {
    mc: i32,
    name_len: i32,
    has_name: i32,
    padding: i32,
    name: [u8; 32],
}

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 92;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("mc2str.bin")
}

fn mc_from_ord(o: i32) -> Option<MCVersion> {
    Some(match o {
        0 => MCVersion::Undef,
        1 => MCVersion::B1_7,
        2 => MCVersion::B1_8,
        3 => MCVersion::V1_0,
        4 => MCVersion::V1_1,
        5 => MCVersion::V1_2,
        6 => MCVersion::V1_3,
        7 => MCVersion::V1_4,
        8 => MCVersion::V1_5,
        9 => MCVersion::V1_6,
        10 => MCVersion::V1_7,
        11 => MCVersion::V1_8,
        12 => MCVersion::V1_9,
        13 => MCVersion::V1_10,
        14 => MCVersion::V1_11,
        15 => MCVersion::V1_12,
        16 => MCVersion::V1_13,
        17 => MCVersion::V1_14,
        18 => MCVersion::V1_15,
        19 => MCVersion::V1_16_1,
        20 => MCVersion::V1_16,
        21 => MCVersion::V1_17,
        22 => MCVersion::V1_18,
        23 => MCVersion::V1_19_2,
        24 => MCVersion::V1_19,
        25 => MCVersion::V1_20,
        26 => MCVersion::V1_21_1,
        27 => MCVersion::V1_21_3,
        28 => MCVersion::V1_21,
        _ => return None,
    })
}

#[test]
fn mc2str_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[Mc2StrRecord] = cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    for r in recs {
        let rust_mc = mc_from_ord(r.mc);
        let got = rust_mc.and_then(MCVersion::name);
        // cubiomes returns "?" for unknown ords (not NULL).
        let len = r.name_len as usize;
        let cubiomes_name = if r.has_name != 0 {
            Some(
                std::str::from_utf8(&r.name[..len.min(32)]).expect("cubiomes name should be UTF-8"),
            )
        } else {
            None
        };
        // Out-of-range ords: cubiomes returns "?", Rust returns None.
        // For Undef (ord 0), cubiomes returns "?" too. So compare:
        if rust_mc.is_none() {
            // Out of our enum range — cubiomes likely returns "?".
            assert_eq!(
                cubiomes_name,
                Some("?"),
                "mc={}: rust unknown, cubiomes {:?}",
                r.mc,
                cubiomes_name
            );
            continue;
        }
        if rust_mc == Some(MCVersion::Undef) {
            // Undef in cubiomes also lands in "?" (no case match).
            assert_eq!(cubiomes_name, Some("?"));
            continue;
        }
        let got = got.expect("known MC version must have a name");
        assert_eq!(
            Some(got),
            cubiomes_name,
            "mc={}: rust {:?} vs cubiomes {:?}",
            r.mc,
            got,
            cubiomes_name
        );
    }
}
