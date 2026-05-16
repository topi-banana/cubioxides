//! `Biome::dimension_id` parity vs cubiomes' `getDimension`.
//! Cross-checks every biome id 0..256.

#![allow(clippy::missing_panics_doc, clippy::doc_markdown)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable, cast_slice};
use cubioxides::biome::Biome;
use cubioxides::mc_version::Dimension;

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
struct GetDimensionRecord {
    id: i32,
    dim: i32,
}

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 89;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("get_dimension.bin")
}

fn dim_to_ord(d: Dimension) -> i32 {
    match d {
        Dimension::Nether => -1,
        Dimension::Overworld => 0,
        Dimension::End => 1,
        _ => panic!("unknown Dimension variant"),
    }
}

#[test]
fn get_dimension_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[GetDimensionRecord] = cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    for r in recs {
        let got = dim_to_ord(Biome::dimension_id(r.id));
        assert_eq!(
            got, r.dim,
            "id={}: dim mismatch — rust {} vs cubiomes {}",
            r.id, got, r.dim
        );
    }
}
