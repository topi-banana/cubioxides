//! Nether-Fortress piece-tree parity vs cubiomes. Each fixture
//! record stores (mc, seed, chunk_x, chunk_z, count, digest); the
//! digest is the same `hash32`-rolled BB hash used for End City.

#![allow(clippy::missing_panics_doc, clippy::doc_markdown)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable, cast_slice};
use cubioxides::finder::fortress::{FortressPiece, get_fortress_pieces};
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
struct FortressPiecesRecord {
    mc: i32,
    padding: i32,
    seed: u64,
    chunk_x: i32,
    chunk_z: i32,
    count: u32,
    digest: u32,
}

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 82;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("fortress_pieces.bin")
}

fn hash32(mut x: u32) -> u32 {
    x ^= x >> 15;
    x = x.wrapping_mul(0xd168_aaad);
    x ^= x >> 15;
    x = x.wrapping_mul(0xaf72_3597);
    x ^= x >> 15;
    x
}

fn digest_pieces(pieces: &[FortressPiece]) -> u32 {
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

fn mc_from_ord(o: i32) -> MCVersion {
    match o {
        15 => MCVersion::V1_12,
        17 => MCVersion::V1_14,
        19 => MCVersion::V1_16_1,
        22 => MCVersion::V1_18,
        _ => panic!("unsupported mc ord {o}"),
    }
}

#[test]
fn fortress_pieces_match_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[FortressPiecesRecord] = cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    for (i, r) in recs.iter().enumerate() {
        let pieces = get_fortress_pieces(mc_from_ord(r.mc), r.seed, r.chunk_x, r.chunk_z, 512);
        assert_eq!(
            pieces.len() as u32,
            r.count,
            "record {i} (mc={}, seed={:#x}, cx={}, cz={}): count mismatch — rust {} vs cubiomes {}",
            r.mc,
            r.seed,
            r.chunk_x,
            r.chunk_z,
            pieces.len(),
            r.count
        );
        let d = digest_pieces(&pieces);
        assert_eq!(
            d, r.digest,
            "record {i} (mc={}, seed={:#x}, cx={}, cz={}): digest mismatch — rust {:#x} vs cubiomes {:#x}",
            r.mc, r.seed, r.chunk_x, r.chunk_z, d, r.digest
        );
    }
}
