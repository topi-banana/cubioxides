//! `inverf` parity vs cubiomes' helper. Cubiomes uses glibc's
//! `erf` while Rust uses the `libm` crate's port; the two converge
//! to within a few ulps. We assert relative-error ≤ 1e-12 rather
//! than bit-exact identity.

#![allow(clippy::missing_panics_doc, clippy::doc_markdown)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable, cast_slice};
use cubioxides::math::inverf;

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
struct InverfRecord {
    x: f64,
    result_bits: u64,
}

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 97;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("inverf.bin")
}

#[test]
fn inverf_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[InverfRecord] = cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    for r in recs {
        let got = inverf(r.x);
        let cubiomes_val = f64::from_bits(r.result_bits);
        if cubiomes_val == 0.0 {
            assert!(got.abs() < 1e-10, "x={}: rust {} should be ~0", r.x, got);
        } else {
            let rel = ((got - cubiomes_val) / cubiomes_val).abs();
            assert!(
                rel < 1e-12,
                "x={}: rust {} vs cubiomes {} (rel err {})",
                r.x,
                got,
                cubiomes_val,
                rel
            );
        }
    }
}
