//! `StructureType::name` parity vs cubiomes' `struct2str`. Covers
//! ordinals 0..32 (well past all valid cubiomes types) to verify
//! the NULL return path for unknown types.

#![allow(clippy::missing_panics_doc, clippy::doc_markdown)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable, cast_slice};
use cubioxides::finder::StructureType;

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
struct Struct2StrRecord {
    stype: i32,
    name_len: i32,
    has_name: i32,
    padding: i32,
    name: [u8; 32],
}

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 91;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("struct2str.bin")
}

#[test]
fn struct2str_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[Struct2StrRecord] = cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    for r in recs {
        let got_name = StructureType::from_ord(r.stype).and_then(StructureType::name);
        if r.has_name == 0 {
            assert!(
                got_name.is_none(),
                "stype={}: rust returned {:?}, cubiomes returned NULL",
                r.stype,
                got_name
            );
        } else {
            let got = got_name.unwrap_or_else(|| {
                panic!(
                    "stype={}: rust returned None, cubiomes returned non-NULL",
                    r.stype
                )
            });
            let len = r.name_len as usize;
            let cubiomes_name =
                std::str::from_utf8(&r.name[..len.min(32)]).expect("cubiomes name should be UTF-8");
            assert_eq!(
                got, cubiomes_name,
                "stype={}: rust {:?} vs cubiomes {:?}",
                r.stype, got, cubiomes_name
            );
        }
    }
}
