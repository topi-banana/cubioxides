//! Parity tests: cubioxides layer ops vs cubiomes via fixtures.
//!
//! Loads the binary records produced by `fixtures-gen layers` and runs
//! the equivalent Rust layer function over the same rectangle. The
//! cubiomes output is captured as a hashed digest so the fixture stays
//! small; if the Rust output disagrees, the digest mismatch flags it.

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::biome::Biome;
use cubioxides::layer::map_continent;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;

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
struct ContinentRecord {
    start_seed: u64,
    x: i32,
    z: i32,
    w: u32,
    h: u32,
    digest: u32,
    pad: u32,
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("fixtures")
        .join("layers")
}

fn load_fixture<R: Pod>(name: &str, expected_kind: u16) -> Vec<R> {
    let path = fixture_dir().join(name);
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (header_bytes, body_bytes) = bytes.split_at(std::mem::size_of::<Header>());
    let header: &Header = bytemuck::from_bytes(header_bytes);
    assert_eq!(header.magic, MAGIC, "wrong magic in {}", path.display());
    assert_eq!(
        header.format_version,
        FORMAT_VERSION,
        "unsupported format version in {}",
        path.display()
    );
    assert_eq!(
        header.kind,
        expected_kind,
        "wrong fixture kind in {}",
        path.display()
    );
    let records: &[R] = bytemuck::cast_slice(body_bytes);
    assert_eq!(
        records.len() as u64,
        header.record_count,
        "record count mismatch in {}",
        path.display()
    );
    records.to_vec()
}

fn hash32(mut x: u32) -> u32 {
    x ^= x >> 15;
    x = x.wrapping_mul(0xd168_aaad);
    x ^= x >> 15;
    x = x.wrapping_mul(0xaf72_3597);
    x ^= x >> 15;
    x
}

#[test]
fn map_continent_matches_cubiomes() {
    let records: Vec<ContinentRecord> = load_fixture("continent.bin", 7);
    assert!(!records.is_empty());

    for (i, rec) in records.iter().enumerate() {
        let cells = (rec.w as usize) * (rec.h as usize);
        let mut buf = vec![Biome::NONE; cells];
        map_continent(
            rec.start_seed,
            &mut buf,
            rec.x,
            rec.z,
            rec.w as usize,
            rec.h as usize,
        );
        let mut digest: u32 = 0;
        for cell in &buf {
            digest ^= hash32(cell.id() as u32);
        }
        assert_eq!(
            digest, rec.digest,
            "map_continent digest mismatch at record {i} (seed={:#x}, x={}, z={}, w={}, h={})",
            rec.start_seed, rec.x, rec.z, rec.w, rec.h
        );
    }
}
