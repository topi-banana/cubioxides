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
use cubioxides::layer::{
    map_biome, map_continent, map_cool, map_deep_ocean, map_heat, map_island, map_land,
    map_land_b18, map_land16, map_mushroom, map_snow, map_snow16, map_special, map_zoom,
    map_zoom_fuzzy,
};
use cubioxides::mc_version::MCVersion;
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

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SingleHopRecord {
    world_seed: u64,
    parent_salt: u64,
    child_salt: u64,
    x: i32,
    z: i32,
    w: u32,
    h: u32,
    digest: u32,
    pad: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TempRecord {
    world_seed: u64,
    continent_salt: u64,
    snow_salt: u64,
    child_salt: u64,
    x: i32,
    z: i32,
    w: u32,
    h: u32,
    digest: u32,
    pad: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BiomeFixtureRecord {
    world_seed: u64,
    continent_salt: u64,
    snow_salt: u64,
    biome_salt: u64,
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

#[derive(Copy, Clone)]
enum LandKind {
    Modern,
    Land16,
    B18,
}

fn run_land_record(rec: &LandRecord, kind: LandKind) -> u32 {
    let x = rec.x;
    let z = rec.z;
    let w = rec.w as usize;
    let h = rec.h as usize;

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

    match kind {
        LandKind::Modern => map_land(
            land_start_salt,
            land_start_seed,
            &parent_buf,
            &mut out,
            x,
            z,
            w,
            h,
        ),
        LandKind::Land16 => map_land16(
            land_start_salt,
            land_start_seed,
            &parent_buf,
            &mut out,
            x,
            z,
            w,
            h,
        ),
        LandKind::B18 => {
            map_land_b18(land_start_seed, &parent_buf, &mut out, x, z, w, h);
        }
    }

    let mut digest: u32 = 0;
    for cell in &out {
        digest ^= hash32(cell.id() as u32);
    }
    digest
}

#[test]
fn map_land_matches_cubiomes() {
    let records: Vec<LandRecord> = load_fixture("land.bin", 10);
    assert!(!records.is_empty());

    for (i, rec) in records.iter().enumerate() {
        let digest = run_land_record(rec, LandKind::Modern);
        assert_eq!(
            digest, rec.digest,
            "map_land digest mismatch at record {i} (world={:#x}, x={}, z={}, w={}, h={})",
            rec.world_seed, rec.x, rec.z, rec.w, rec.h
        );
    }
}

#[test]
fn map_land16_matches_cubiomes() {
    let records: Vec<LandRecord> = load_fixture("land16.bin", 11);
    assert!(!records.is_empty());
    for (i, rec) in records.iter().enumerate() {
        let digest = run_land_record(rec, LandKind::Land16);
        assert_eq!(
            digest, rec.digest,
            "map_land16 digest mismatch at record {i} (world={:#x}, x={}, z={}, w={}, h={})",
            rec.world_seed, rec.x, rec.z, rec.w, rec.h
        );
    }
}

#[derive(Copy, Clone)]
enum SingleHopKind {
    Island,
    Snow16,
    Snow,
    Special,
    Mushroom,
    DeepOcean,
}

fn run_single_hop_record(rec: &SingleHopRecord, kind: SingleHopKind) -> u32 {
    let x = rec.x;
    let z = rec.z;
    let w = rec.w as usize;
    let h = rec.h as usize;

    let parent_w = w + 2;
    let parent_h = h + 2;
    let parent_x = x - 1;
    let parent_z = z - 1;
    let parent_start_seed = get_start_seed(rec.world_seed, rec.parent_salt);

    // For map_special the parent rectangle coincides with (x, z, w, h).
    // For the others it is (x-1, z-1, w+2, h+2).
    let (px, pz, pw, ph) = if matches!(kind, SingleHopKind::Special) {
        (x, z, w, h)
    } else {
        (parent_x, parent_z, parent_w, parent_h)
    };
    let mut parent_buf = vec![Biome::NONE; pw * ph];
    map_continent(parent_start_seed, &mut parent_buf, px, pz, pw, ph);

    let child_start_salt = get_start_salt(rec.world_seed, rec.child_salt);
    let child_start_seed = get_start_seed(rec.world_seed, rec.child_salt);
    let mut out = vec![Biome::NONE; w * h];

    match kind {
        SingleHopKind::Island => {
            map_island(child_start_seed, &parent_buf, &mut out, x, z, w, h);
        }
        SingleHopKind::Snow16 => {
            map_snow16(child_start_seed, &parent_buf, &mut out, x, z, w, h);
        }
        SingleHopKind::Snow => {
            map_snow(child_start_seed, &parent_buf, &mut out, x, z, w, h);
        }
        SingleHopKind::Special => {
            map_special(
                child_start_salt,
                child_start_seed,
                &parent_buf,
                &mut out,
                x,
                z,
                w,
                h,
            );
        }
        SingleHopKind::Mushroom => {
            map_mushroom(child_start_seed, &parent_buf, &mut out, x, z, w, h);
        }
        SingleHopKind::DeepOcean => {
            map_deep_ocean(&parent_buf, &mut out, w, h);
        }
    }

    let mut digest: u32 = 0;
    for cell in &out {
        digest ^= hash32(cell.id() as u32);
    }
    digest
}

#[test]
fn map_island_matches_cubiomes() {
    let records: Vec<SingleHopRecord> = load_fixture("island.bin", 13);
    assert!(!records.is_empty());
    for (i, rec) in records.iter().enumerate() {
        let digest = run_single_hop_record(rec, SingleHopKind::Island);
        assert_eq!(digest, rec.digest, "map_island mismatch at record {i}");
    }
}

#[test]
fn map_snow16_matches_cubiomes() {
    let records: Vec<SingleHopRecord> = load_fixture("snow16.bin", 14);
    assert!(!records.is_empty());
    for (i, rec) in records.iter().enumerate() {
        let digest = run_single_hop_record(rec, SingleHopKind::Snow16);
        assert_eq!(digest, rec.digest, "map_snow16 mismatch at record {i}");
    }
}

#[test]
fn map_snow_matches_cubiomes() {
    let records: Vec<SingleHopRecord> = load_fixture("snow.bin", 15);
    assert!(!records.is_empty());
    for (i, rec) in records.iter().enumerate() {
        let digest = run_single_hop_record(rec, SingleHopKind::Snow);
        assert_eq!(digest, rec.digest, "map_snow mismatch at record {i}");
    }
}

#[test]
fn map_special_matches_cubiomes() {
    let records: Vec<SingleHopRecord> = load_fixture("special.bin", 16);
    assert!(!records.is_empty());
    for (i, rec) in records.iter().enumerate() {
        let digest = run_single_hop_record(rec, SingleHopKind::Special);
        assert_eq!(digest, rec.digest, "map_special mismatch at record {i}");
    }
}

#[test]
fn map_mushroom_matches_cubiomes() {
    let records: Vec<SingleHopRecord> = load_fixture("mushroom.bin", 17);
    assert!(!records.is_empty());
    for (i, rec) in records.iter().enumerate() {
        let digest = run_single_hop_record(rec, SingleHopKind::Mushroom);
        assert_eq!(digest, rec.digest, "map_mushroom mismatch at record {i}");
    }
}

#[test]
fn map_deep_ocean_matches_cubiomes() {
    let records: Vec<SingleHopRecord> = load_fixture("deep_ocean.bin", 18);
    assert!(!records.is_empty());
    for (i, rec) in records.iter().enumerate() {
        let digest = run_single_hop_record(rec, SingleHopKind::DeepOcean);
        assert_eq!(digest, rec.digest, "map_deep_ocean mismatch at record {i}");
    }
}

#[derive(Copy, Clone)]
enum TempKind {
    Cool,
    Heat,
}

fn run_temp_record(rec: &TempRecord, kind: TempKind) -> u32 {
    let x = rec.x;
    let z = rec.z;
    let w = rec.w as usize;
    let h = rec.h as usize;

    // Cool / heat read (w+2, h+2) at (x-1, z-1). Snow in turn reads
    // (w+4, h+4) at (x-2, z-2) from the continent layer.
    let cont_w = w + 4;
    let cont_h = h + 4;
    let cont_x = x - 2;
    let cont_z = z - 2;
    let cont_start_seed = get_start_seed(rec.world_seed, rec.continent_salt);
    let mut cont_buf = vec![Biome::NONE; cont_w * cont_h];
    map_continent(
        cont_start_seed,
        &mut cont_buf,
        cont_x,
        cont_z,
        cont_w,
        cont_h,
    );

    let snow_w = w + 2;
    let snow_h = h + 2;
    let snow_x = x - 1;
    let snow_z = z - 1;
    let snow_start_seed = get_start_seed(rec.world_seed, rec.snow_salt);
    let mut snow_buf = vec![Biome::NONE; snow_w * snow_h];
    map_snow(
        snow_start_seed,
        &cont_buf,
        &mut snow_buf,
        snow_x,
        snow_z,
        snow_w,
        snow_h,
    );

    let mut out = vec![Biome::NONE; w * h];
    match kind {
        TempKind::Cool => map_cool(&snow_buf, &mut out, w, h),
        TempKind::Heat => map_heat(&snow_buf, &mut out, w, h),
    }

    let mut digest: u32 = 0;
    for cell in &out {
        digest ^= hash32(cell.id() as u32);
    }
    digest
}

#[test]
fn map_cool_matches_cubiomes() {
    let records: Vec<TempRecord> = load_fixture("cool.bin", 19);
    assert!(!records.is_empty());
    for (i, rec) in records.iter().enumerate() {
        let digest = run_temp_record(rec, TempKind::Cool);
        assert_eq!(digest, rec.digest, "map_cool mismatch at record {i}");
    }
}

#[test]
fn map_heat_matches_cubiomes() {
    let records: Vec<TempRecord> = load_fixture("heat.bin", 20);
    assert!(!records.is_empty());
    for (i, rec) in records.iter().enumerate() {
        let digest = run_temp_record(rec, TempKind::Heat);
        assert_eq!(digest, rec.digest, "map_heat mismatch at record {i}");
    }
}

#[test]
fn map_biome_matches_cubiomes() {
    let records: Vec<BiomeFixtureRecord> = load_fixture("biome.bin", 21);
    assert!(!records.is_empty());
    for (i, rec) in records.iter().enumerate() {
        let x = rec.x;
        let z = rec.z;
        let w = rec.w as usize;
        let h = rec.h as usize;

        // 3-hop chain: continent (w+4, h+4) -> snow (w+2, h+2) -> biome (w, h).
        let cont_w = w + 4;
        let cont_h = h + 4;
        let cont_x = x - 2;
        let cont_z = z - 2;
        let cont_seed = get_start_seed(rec.world_seed, rec.continent_salt);
        let mut cont_buf = vec![Biome::NONE; cont_w * cont_h];
        map_continent(cont_seed, &mut cont_buf, cont_x, cont_z, cont_w, cont_h);

        let snow_w = w + 2;
        let snow_h = h + 2;
        let snow_x = x - 1;
        let snow_z = z - 1;
        let snow_seed = get_start_seed(rec.world_seed, rec.snow_salt);
        let mut snow_buf = vec![Biome::NONE; snow_w * snow_h];
        map_snow(
            snow_seed,
            &cont_buf,
            &mut snow_buf,
            snow_x,
            snow_z,
            snow_w,
            snow_h,
        );

        let biome_seed = get_start_seed(rec.world_seed, rec.biome_salt);
        let mut out = vec![Biome::NONE; w * h];
        // map_biome reads a (w, h) parent (no padding), so take the
        // centre window of snow_buf.
        let mut biome_parent = vec![Biome::NONE; w * h];
        for jj in 0..h {
            for ii in 0..w {
                biome_parent[ii + jj * w] = snow_buf[(ii + 1) + (jj + 1) * snow_w];
            }
        }
        map_biome(
            MCVersion::V1_7,
            biome_seed,
            &biome_parent,
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
        assert_eq!(digest, rec.digest, "map_biome mismatch at record {i}");
    }
}

#[test]
fn map_cool_passes_through_uniform_input() {
    // map_cool is pure (no seed). Quick smoke test: uniform Warm input
    // stays Warm because no Cold/Freezing neighbour exists.
    let parent_w = 4 + 2;
    let parent_h = 4 + 2;
    let parent = vec![Biome(1); parent_w * parent_h]; // 1 = Warm
    let mut out = vec![Biome::NONE; 4 * 4];
    map_cool(&parent, &mut out, 4, 4);
    for cell in &out {
        assert_eq!(*cell, Biome(1));
    }
}

#[test]
fn map_heat_passes_through_uniform_input() {
    let parent_w = 4 + 2;
    let parent_h = 4 + 2;
    let parent = vec![Biome(4); parent_w * parent_h]; // 4 = Freezing
    let mut out = vec![Biome::NONE; 4 * 4];
    map_heat(&parent, &mut out, 4, 4);
    for cell in &out {
        assert_eq!(*cell, Biome(4));
    }
}

#[test]
fn map_land_b18_matches_cubiomes() {
    let records: Vec<LandRecord> = load_fixture("land_b18.bin", 12);
    assert!(!records.is_empty());
    for (i, rec) in records.iter().enumerate() {
        let digest = run_land_record(rec, LandKind::B18);
        assert_eq!(
            digest, rec.digest,
            "map_land_b18 digest mismatch at record {i} (world={:#x}, x={}, z={}, w={}, h={})",
            rec.world_seed, rec.x, rec.z, rec.w, rec.h
        );
    }
}
