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
use cubioxides::layer::{map_continent, map_land, map_zoom, map_zoom_fuzzy};
use cubioxides::rng::{get_start_salt, get_start_seed};

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

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ZoomRecord {
    world_seed: u64,
    parent_salt: u64,
    zoom_salt: u64,
    x: i32,
    z: i32,
    w: u32,
    h: u32,
    digest: u32,
    pad: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct LandRecord {
    world_seed: u64,
    parent_salt: u64,
    land_salt: u64,
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

fn run_zoom_record(rec: &ZoomRecord, kind: ZoomKind) -> u32 {
    let x = rec.x;
    let z = rec.z;
    let w = rec.w as usize;
    let h = rec.h as usize;

    // Compute parent rectangle (same arithmetic as cubiomes / our zoom_impl).
    let parent_x = x >> 1;
    let parent_z = z >> 1;
    let parent_w = (((x + w as i32) >> 1) - parent_x + 1) as usize;
    let parent_h = (((z + h as i32) >> 1) - parent_z + 1) as usize;

    let parent_start_seed = get_start_seed(rec.world_seed, rec.parent_salt);
    let mut parent_buf = vec![Biome::NONE; parent_w * parent_h];
    map_continent(
        parent_start_seed,
        &mut parent_buf,
        parent_x,
        parent_z,
        parent_w,
        parent_h,
    );

    let zoom_start_salt = get_start_salt(rec.world_seed, rec.zoom_salt);
    let zoom_start_seed = get_start_seed(rec.world_seed, rec.zoom_salt);
    let mut out = vec![Biome::NONE; w * h];

    match kind {
        ZoomKind::Fuzzy => map_zoom_fuzzy(
            zoom_start_salt,
            zoom_start_seed,
            &parent_buf,
            parent_x,
            parent_z,
            parent_w,
            parent_h,
            &mut out,
            x,
            z,
            w,
            h,
        ),
        ZoomKind::Majority => map_zoom(
            zoom_start_salt,
            zoom_start_seed,
            &parent_buf,
            parent_x,
            parent_z,
            parent_w,
            parent_h,
            &mut out,
            x,
            z,
            w,
            h,
        ),
    }

    let mut digest: u32 = 0;
    for cell in &out {
        digest ^= hash32(cell.id() as u32);
    }
    digest
}

#[derive(Copy, Clone)]
enum ZoomKind {
    Fuzzy,
    Majority,
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

#[test]
fn map_zoom_fuzzy_matches_cubiomes() {
    let records: Vec<ZoomRecord> = load_fixture("zoom_fuzzy.bin", 8);
    assert!(!records.is_empty());
    for (i, rec) in records.iter().enumerate() {
        let digest = run_zoom_record(rec, ZoomKind::Fuzzy);
        assert_eq!(
            digest, rec.digest,
            "map_zoom_fuzzy digest mismatch at record {i} (world={:#x}, x={}, z={}, w={}, h={})",
            rec.world_seed, rec.x, rec.z, rec.w, rec.h
        );
    }
}

#[test]
fn map_zoom_matches_cubiomes() {
    let records: Vec<ZoomRecord> = load_fixture("zoom.bin", 9);
    assert!(!records.is_empty());
    for (i, rec) in records.iter().enumerate() {
        let digest = run_zoom_record(rec, ZoomKind::Majority);
        assert_eq!(
            digest, rec.digest,
            "map_zoom digest mismatch at record {i} (world={:#x}, x={}, z={}, w={}, h={})",
            rec.world_seed, rec.x, rec.z, rec.w, rec.h
        );
    }
}

#[test]
fn map_land_matches_cubiomes() {
    let records: Vec<LandRecord> = load_fixture("land.bin", 10);
    assert!(!records.is_empty());

    for (i, rec) in records.iter().enumerate() {
        let x = rec.x;
        let z = rec.z;
        let w = rec.w as usize;
        let h = rec.h as usize;

        // map_land's parent rectangle is (w + 2) × (h + 2) starting at
        // (x - 1, z - 1). Build it with map_continent, mirroring
        // cubiomes_call_map_land.
        let parent_w = w + 2;
        let parent_h = h + 2;
        let parent_x = x - 1;
        let parent_z = z - 1;
        let parent_start_seed = get_start_seed(rec.world_seed, rec.parent_salt);
        let mut parent_buf = vec![Biome::NONE; parent_w * parent_h];
        map_continent(
            parent_start_seed,
            &mut parent_buf,
            parent_x,
            parent_z,
            parent_w,
            parent_h,
        );

        let land_start_salt = get_start_salt(rec.world_seed, rec.land_salt);
        let land_start_seed = get_start_seed(rec.world_seed, rec.land_salt);
        let mut out = vec![Biome::NONE; w * h];
        map_land(
            land_start_salt,
            land_start_seed,
            &parent_buf,
            &mut out,
            x,
            z,
            w,
            h,
        );

        let mut digest: u32 = 0;
        for cell in &out {
            digest ^= hash32(cell.id() as u32);
        }
        assert_eq!(
            digest, rec.digest,
            "map_land digest mismatch at record {i} (world={:#x}, x={}, z={}, w={}, h={})",
            rec.world_seed, rec.x, rec.z, rec.w, rec.h
        );
    }
}
