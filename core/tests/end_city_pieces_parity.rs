//! End City piece-tree parity vs cubiomes. For each fixture
//! record, compare Rust's piece count + `hash32`-rolled digest
//! against cubiomes' `getEndCityPieces` output.

#![allow(clippy::missing_panics_doc, clippy::doc_markdown)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable, cast_slice};
use cubioxides::finder::end_city::{Piece, get_end_city_pieces};

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
struct EndCityPiecesRecord {
    seed: u64,
    chunk_x: i32,
    chunk_z: i32,
    count: u32,
    digest: u32,
}

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 80;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("end_city_pieces.bin")
}

fn hash32(mut x: u32) -> u32 {
    x ^= x >> 15;
    x = x.wrapping_mul(0xd168_aaad);
    x ^= x >> 15;
    x = x.wrapping_mul(0xaf72_3597);
    x ^= x >> 15;
    x
}

fn digest_pieces(pieces: &[Piece]) -> u32 {
    let mut h: u32 = 0;
    for p in pieces {
        for v in [
            p.bb0.x,
            p.bb0.y,
            p.bb0.z,
            p.bb1.x,
            p.bb1.y,
            p.bb1.z,
            p.pos.x,
            p.pos.y,
            p.pos.z,
            p.rot as i32,
            p.kind as i32,
        ] {
            h = hash32(h.wrapping_add(v as u32));
        }
    }
    h
}

#[test]
fn end_city_pieces_match_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[EndCityPiecesRecord] = cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    for (i, r) in recs.iter().enumerate() {
        let pieces = get_end_city_pieces(r.seed, r.chunk_x, r.chunk_z);
        assert_eq!(
            pieces.len() as u32,
            r.count,
            "record {i} (seed={:#x}, cx={}, cz={}): count mismatch — rust {} vs cubiomes {}",
            r.seed,
            r.chunk_x,
            r.chunk_z,
            pieces.len(),
            r.count
        );
        let d = digest_pieces(&pieces);
        assert_eq!(
            d, r.digest,
            "record {i} (seed={:#x}, cx={}, cz={}): digest mismatch — rust {:#x} vs cubiomes {:#x}",
            r.seed, r.chunk_x, r.chunk_z, d, r.digest
        );
    }
}
