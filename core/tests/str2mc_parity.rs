//! `MCVersion::from_str` parity vs cubiomes' `str2mc`. Covers all
//! the supported names plus a few unrecognised strings.

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
struct Str2McRecord {
    name_len: i32,
    mc: i32,
    name: [u8; 32],
}

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 93;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("str2mc.bin")
}

#[test]
fn str2mc_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[Str2McRecord] = cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    for r in recs {
        let len = r.name_len as usize;
        let name = std::str::from_utf8(&r.name[..len.min(32)]).expect("name UTF-8");
        let rust_mc = MCVersion::from_str(name);
        let rust_ord = rust_mc.ord() as i32;
        assert_eq!(
            rust_ord, r.mc,
            "name={:?}: rust {} ({:?}) vs cubiomes {}",
            name, rust_ord, rust_mc, r.mc
        );
    }
}
