//! `get_largest_rec` parity vs cubiomes' `getLargestRec`. Bit-exact
//! check on a small set of hand-picked grids (full-match grids are
//! excluded because cubiomes' internal stack can overflow).

#![allow(clippy::missing_panics_doc, clippy::doc_markdown)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable, cast_slice};
use cubioxides::finder::largest_rec::get_largest_rec;

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
struct GetLargestRecRecord {
    target: i32,
    sx: i32,
    sz: i32,
    area: i32,
    p0x: i32,
    p0z: i32,
    p1x: i32,
    p1z: i32,
    ids: [i32; 64],
}

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 86;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("get_largest_rec.bin")
}

#[test]
fn get_largest_rec_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[GetLargestRecRecord] = cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    for (i, r) in recs.iter().enumerate() {
        let n = (r.sx as usize) * (r.sz as usize);
        let result = get_largest_rec(r.target, &r.ids[..n], r.sx, r.sz);
        assert_eq!(
            result.area, r.area,
            "record {i} (target={}, {}x{}): area mismatch — rust {} vs cubiomes {}",
            r.target, r.sx, r.sz, result.area, r.area
        );
        if r.area > 0 {
            assert_eq!(
                (result.p0.0, result.p0.1, result.p1.0, result.p1.1),
                (r.p0x, r.p0z, r.p1x, r.p1z),
                "record {i}: corner mismatch — rust ({},{})-({},{}) vs cubiomes ({},{})-({},{})",
                result.p0.0,
                result.p0.1,
                result.p1.0,
                result.p1.1,
                r.p0x,
                r.p0z,
                r.p1x,
                r.p1z,
            );
        }
    }
}
