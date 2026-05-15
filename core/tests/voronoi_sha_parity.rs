//! Parity test: `cubioxides::sha::voronoi_sha` vs cubiomes'
//! `getVoronoiSHA`. Reads the binary fixture produced by
//! `fixtures-gen layers` (kind = 34, ~4096 (seed, digest) pairs
//! including a hand-picked edge-case prefix) and compares one-to-one.

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::sha::voronoi_sha;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 34;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Header {
    magic: [u8; 4],
    format_version: u16,
    kind: u16,
    record_count: u64,
    reserved: [u64; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct VoronoiShaRecord {
    seed: u64,
    digest: u64,
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("fixtures")
        .join("layers")
        .join("voronoi_sha.bin")
}

#[test]
fn voronoi_sha_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (header_bytes, body_bytes) = bytes.split_at(std::mem::size_of::<Header>());
    let header: &Header = bytemuck::from_bytes(header_bytes);
    assert_eq!(header.magic, MAGIC, "wrong magic");
    assert_eq!(
        header.format_version, FORMAT_VERSION,
        "unsupported format version"
    );
    assert_eq!(header.kind, KIND, "wrong fixture kind");
    let records: &[VoronoiShaRecord] = bytemuck::cast_slice(body_bytes);
    assert_eq!(records.len() as u64, header.record_count);
    assert!(!records.is_empty());

    for (i, rec) in records.iter().enumerate() {
        let got = voronoi_sha(rec.seed);
        assert_eq!(
            got, rec.digest,
            "voronoi_sha mismatch at record {i} (seed = {:#018x}, expected {:#018x}, got {:#018x})",
            rec.seed, rec.digest, got
        );
    }
}
