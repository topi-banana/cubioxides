//! `wilson` parity vs cubiomes' helper of the same name. f64
//! `to_bits()` comparison so any floating-point divergence is
//! caught immediately.

#![allow(clippy::missing_panics_doc, clippy::doc_markdown)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable, cast_slice};
use cubioxides::math::wilson;

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
struct WilsonRecord {
    n: f64,
    p: f64,
    z: f64,
    lo_bits: u64,
    hi_bits: u64,
}

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 96;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("wilson.bin")
}

#[test]
fn wilson_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[WilsonRecord] = cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    for (i, r) in recs.iter().enumerate() {
        let (lo, hi) = wilson(r.n, r.p, r.z);
        assert_eq!(
            lo.to_bits(),
            r.lo_bits,
            "case {i} (n={}, p={}, z={}): lo mismatch — rust {:#x} vs cubiomes {:#x}",
            r.n,
            r.p,
            r.z,
            lo.to_bits(),
            r.lo_bits
        );
        assert_eq!(
            hi.to_bits(),
            r.hi_bits,
            "case {i} (n={}, p={}, z={}): hi mismatch — rust {:#x} vs cubiomes {:#x}",
            r.n,
            r.p,
            r.z,
            hi.to_bits(),
            r.hi_bits
        );
    }
}
