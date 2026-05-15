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
    map_bamboo, map_biome, map_continent, map_cool, map_deep_ocean, map_heat, map_hills,
    map_island, map_land, map_land_b18, map_land16, map_mushroom, map_noise, map_ocean_mix,
    map_ocean_temp, map_river, map_river_mix, map_shore, map_smooth, map_snow, map_snow16,
    map_special, map_sunflower, map_swamp_river, map_voronoi, map_voronoi114, map_zoom,
    map_zoom_fuzzy, ocean_land_bbox, voronoi_access_3d,
};
use cubioxides::mc_version::MCVersion;
use cubioxides::noise::PerlinNoise;
use cubioxides::rng::JavaRng;
use cubioxides::rng::{get_start_salt, get_start_seed};
use cubioxides::sha::voronoi_sha;

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

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct FourHopRecord {
    world_seed: u64,
    continent_salt: u64,
    snow_salt: u64,
    biome_salt: u64,
    child_salt: u64,
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

#[derive(Copy, Clone)]
enum FourHopKind {
    Noise,
    Bamboo,
    SwampRiver,
    Sunflower,
}

fn run_four_hop_record(rec: &FourHopRecord, kind: FourHopKind) -> u32 {
    let x = rec.x;
    let z = rec.z;
    let w = rec.w as usize;
    let h = rec.h as usize;

    // 4-hop chain: continent (w+4, h+4) -> snow (w+2, h+2) ->
    // biome (w, h) -> child (w, h).
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

    // Trim snow_buf's centre window for map_biome (it expects no padding).
    let biome_seed = get_start_seed(rec.world_seed, rec.biome_salt);
    let mut biome_parent = vec![Biome::NONE; w * h];
    for jj in 0..h {
        for ii in 0..w {
            biome_parent[ii + jj * w] = snow_buf[(ii + 1) + (jj + 1) * snow_w];
        }
    }
    let mut biome_buf = vec![Biome::NONE; w * h];
    map_biome(
        MCVersion::V1_7,
        biome_seed,
        &biome_parent,
        &mut biome_buf,
        x,
        z,
        w,
        h,
    );

    let child_seed = get_start_seed(rec.world_seed, rec.child_salt);
    let mut out = vec![Biome::NONE; w * h];
    match kind {
        FourHopKind::Noise => {
            map_noise(
                MCVersion::V1_7,
                child_seed,
                &biome_buf,
                &mut out,
                x,
                z,
                w,
                h,
            );
        }
        FourHopKind::Bamboo => {
            map_bamboo(child_seed, &biome_buf, &mut out, x, z, w, h);
        }
        FourHopKind::SwampRiver => {
            map_swamp_river(child_seed, &biome_buf, &mut out, x, z, w, h);
        }
        FourHopKind::Sunflower => {
            map_sunflower(child_seed, &biome_buf, &mut out, x, z, w, h);
        }
    }

    let mut digest: u32 = 0;
    for cell in &out {
        digest ^= hash32(cell.id() as u32);
    }
    digest
}

#[test]
fn map_noise_matches_cubiomes() {
    let records: Vec<FourHopRecord> = load_fixture("noise.bin", 22);
    assert!(!records.is_empty());
    for (i, rec) in records.iter().enumerate() {
        let digest = run_four_hop_record(rec, FourHopKind::Noise);
        assert_eq!(digest, rec.digest, "map_noise mismatch at record {i}");
    }
}

#[test]
fn map_bamboo_matches_cubiomes() {
    let records: Vec<FourHopRecord> = load_fixture("bamboo.bin", 23);
    assert!(!records.is_empty());
    for (i, rec) in records.iter().enumerate() {
        let digest = run_four_hop_record(rec, FourHopKind::Bamboo);
        assert_eq!(digest, rec.digest, "map_bamboo mismatch at record {i}");
    }
}

#[test]
fn map_swamp_river_matches_cubiomes() {
    let records: Vec<FourHopRecord> = load_fixture("swamp_river.bin", 24);
    assert!(!records.is_empty());
    for (i, rec) in records.iter().enumerate() {
        let digest = run_four_hop_record(rec, FourHopKind::SwampRiver);
        assert_eq!(digest, rec.digest, "map_swamp_river mismatch at record {i}");
    }
}

#[test]
fn map_sunflower_matches_cubiomes() {
    let records: Vec<FourHopRecord> = load_fixture("sunflower_layer.bin", 25);
    assert!(!records.is_empty());
    for (i, rec) in records.iter().enumerate() {
        let digest = run_four_hop_record(rec, FourHopKind::Sunflower);
        assert_eq!(digest, rec.digest, "map_sunflower mismatch at record {i}");
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct OceanTempRecord {
    world_seed: u64,
    x: i32,
    z: i32,
    w: u32,
    h: u32,
    digest: u32,
    pad: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Voronoi114FixtureRecord {
    world_seed: u64,
    parent_salt: u64,
    voronoi_salt: u64,
    x: i32,
    z: i32,
    w: u32,
    h: u32,
    digest: u32,
    pad: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ShoreFixtureRecord {
    world_seed: u64,
    parent_salt: u64,
    shore_salt: u64,
    x: i32,
    z: i32,
    w: u32,
    h: u32,
    digest: u32,
    pad: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct HillsFixtureRecord {
    world_seed: u64,
    biome_parent_salt: u64,
    river_parent_salt: u64,
    hills_salt: u64,
    x: i32,
    z: i32,
    w: u32,
    h: u32,
    digest: u32,
    pad: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PostBiomeRecord {
    world_seed: u64,
    primary_salt: u64,
    secondary_salt: u64,
    target_salt: u64,
    x: i32,
    z: i32,
    w: u32,
    h: u32,
    digest: u32,
    pad: u32,
}

#[test]
fn map_river_matches_cubiomes() {
    let records: Vec<PostBiomeRecord> = load_fixture("river.bin", 30);
    assert!(!records.is_empty());
    for (i, rec) in records.iter().enumerate() {
        let w = rec.w as usize;
        let h = rec.h as usize;
        let parent_w = w + 2;
        let parent_h = h + 2;
        let parent_seed = get_start_seed(rec.world_seed, rec.primary_salt);
        let mut parent_buf = vec![Biome::NONE; parent_w * parent_h];
        map_continent(
            parent_seed,
            &mut parent_buf,
            rec.x - 1,
            rec.z - 1,
            parent_w,
            parent_h,
        );
        let mut out = vec![Biome::NONE; w * h];
        map_river(MCVersion::V1_18, &parent_buf, &mut out, w, h);
        let mut digest: u32 = 0;
        for cell in &out {
            digest ^= hash32(cell.id() as u32);
        }
        assert_eq!(digest, rec.digest, "map_river mismatch at record {i}");
    }
}

#[test]
fn map_smooth_matches_cubiomes() {
    let records: Vec<PostBiomeRecord> = load_fixture("smooth.bin", 31);
    assert!(!records.is_empty());
    for (i, rec) in records.iter().enumerate() {
        let w = rec.w as usize;
        let h = rec.h as usize;
        let parent_w = w + 2;
        let parent_h = h + 2;
        let parent_seed = get_start_seed(rec.world_seed, rec.primary_salt);
        let mut parent_buf = vec![Biome::NONE; parent_w * parent_h];
        map_continent(
            parent_seed,
            &mut parent_buf,
            rec.x - 1,
            rec.z - 1,
            parent_w,
            parent_h,
        );
        let smooth_start_seed = get_start_seed(rec.world_seed, rec.target_salt);
        let mut out = vec![Biome::NONE; w * h];
        map_smooth(smooth_start_seed, &parent_buf, &mut out, rec.x, rec.z, w, h);
        let mut digest: u32 = 0;
        for cell in &out {
            digest ^= hash32(cell.id() as u32);
        }
        assert_eq!(digest, rec.digest, "map_smooth mismatch at record {i}");
    }
}

#[test]
fn map_river_mix_matches_cubiomes() {
    let records: Vec<PostBiomeRecord> = load_fixture("river_mix.bin", 32);
    assert!(!records.is_empty());
    for (i, rec) in records.iter().enumerate() {
        let w = rec.w as usize;
        let h = rec.h as usize;
        let biome_seed = get_start_seed(rec.world_seed, rec.primary_salt);
        let river_seed = get_start_seed(rec.world_seed, rec.secondary_salt);
        let mut biome_buf = vec![Biome::NONE; w * h];
        let mut river_buf = vec![Biome::NONE; w * h];
        map_continent(biome_seed, &mut biome_buf, rec.x, rec.z, w, h);
        map_continent(river_seed, &mut river_buf, rec.x, rec.z, w, h);
        let mut out = vec![Biome::NONE; w * h];
        map_river_mix(MCVersion::V1_18, &biome_buf, &river_buf, &mut out, w, h);
        let mut digest: u32 = 0;
        for cell in &out {
            digest ^= hash32(cell.id() as u32);
        }
        assert_eq!(digest, rec.digest, "map_river_mix mismatch at record {i}");
    }
}

#[test]
fn map_hills_matches_cubiomes() {
    let records: Vec<HillsFixtureRecord> = load_fixture("hills.bin", 29);
    assert!(!records.is_empty());

    for (i, rec) in records.iter().enumerate() {
        let x = rec.x;
        let z = rec.z;
        let w = rec.w as usize;
        let h = rec.h as usize;

        let parent_w = w + 2;
        let parent_h = h + 2;
        let parent_x = x - 1;
        let parent_z = z - 1;

        let biome_start_seed = get_start_seed(rec.world_seed, rec.biome_parent_salt);
        let mut biome_parent = vec![Biome::NONE; parent_w * parent_h];
        map_continent(
            biome_start_seed,
            &mut biome_parent,
            parent_x,
            parent_z,
            parent_w,
            parent_h,
        );

        let river_start_seed = get_start_seed(rec.world_seed, rec.river_parent_salt);
        let mut river_parent = vec![Biome::NONE; parent_w * parent_h];
        map_continent(
            river_start_seed,
            &mut river_parent,
            parent_x,
            parent_z,
            parent_w,
            parent_h,
        );

        let hills_start_salt = get_start_salt(rec.world_seed, rec.hills_salt);
        let hills_start_seed = get_start_seed(rec.world_seed, rec.hills_salt);
        let mut out = vec![Biome::NONE; w * h];
        map_hills(
            MCVersion::V1_18,
            hills_start_salt,
            hills_start_seed,
            &biome_parent,
            &river_parent,
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
            "map_hills digest mismatch at record {i}"
        );
    }
}

#[test]
fn map_shore_matches_cubiomes() {
    let records: Vec<ShoreFixtureRecord> = load_fixture("shore.bin", 28);
    assert!(!records.is_empty());

    for (i, rec) in records.iter().enumerate() {
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

        let mut out = vec![Biome::NONE; w * h];
        map_shore(MCVersion::V1_18, &parent_buf, &mut out, w, h);

        let mut digest: u32 = 0;
        for cell in &out {
            digest ^= hash32(cell.id() as u32);
        }
        assert_eq!(
            digest, rec.digest,
            "map_shore digest mismatch at record {i}"
        );
    }
}

#[test]
fn map_voronoi114_matches_cubiomes() {
    let records: Vec<Voronoi114FixtureRecord> = load_fixture("voronoi114.bin", 27);
    assert!(!records.is_empty());

    for (i, rec) in records.iter().enumerate() {
        let x = rec.x;
        let z = rec.z;
        let w = rec.w as usize;
        let h = rec.h as usize;

        // Parent rect matches cubiomes' `x -= 2; px = x >> 2; ...` math.
        let vx = x - 2;
        let vz = z - 2;
        let parent_x = vx >> 2;
        let parent_z = vz >> 2;
        let parent_w = (((vx + w as i32) >> 2) - parent_x + 2) as usize;
        let parent_h = (((vz + h as i32) >> 2) - parent_z + 2) as usize;

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

        let voronoi_start_salt = get_start_salt(rec.world_seed, rec.voronoi_salt);
        let voronoi_start_seed = get_start_seed(rec.world_seed, rec.voronoi_salt);
        let mut out = vec![Biome::NONE; w * h];
        map_voronoi114(
            voronoi_start_salt,
            voronoi_start_seed,
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
        );

        let mut digest: u32 = 0;
        for cell in &out {
            digest ^= hash32(cell.id() as u32);
        }
        assert_eq!(
            digest, rec.digest,
            "map_voronoi114 digest mismatch at record {i}"
        );
    }
}

#[test]
fn map_ocean_temp_matches_cubiomes() {
    let records: Vec<OceanTempRecord> = load_fixture("ocean_temp.bin", 26);
    assert!(!records.is_empty());
    for (i, rec) in records.iter().enumerate() {
        let w = rec.w as usize;
        let h = rec.h as usize;
        // cubiomes' wrapper builds the PerlinNoise via
        // `setSeed(&s, world_seed); perlinInit(&noise, &s);`. Mirror
        // it on the Rust side.
        let mut rng = JavaRng::new(rec.world_seed);
        let noise = PerlinNoise::from_java(&mut rng);
        let mut out = vec![Biome::NONE; w * h];
        map_ocean_temp(&noise, &mut out, rec.x, rec.z, w, h);
        let mut digest: u32 = 0;
        for cell in &out {
            digest ^= hash32(cell.id() as u32);
        }
        assert_eq!(
            digest, rec.digest,
            "map_ocean_temp digest mismatch at record {i}"
        );
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct OceanMixRecord {
    world_seed: u64,
    biome_salt: u64,
    x: i32,
    z: i32,
    w: u32,
    h: u32,
    digest: u32,
    pad: u32,
}

#[test]
fn map_ocean_mix_matches_cubiomes() {
    let records: Vec<OceanMixRecord> = load_fixture("ocean_mix.bin", 33);
    assert!(!records.is_empty());
    for (i, rec) in records.iter().enumerate() {
        let w = rec.w as usize;
        let h = rec.h as usize;

        let mut rng = JavaRng::new(rec.world_seed);
        let noise = PerlinNoise::from_java(&mut rng);
        let mut ocean = vec![Biome::NONE; w * h];
        map_ocean_temp(&noise, &mut ocean, rec.x, rec.z, w, h);

        let (lx0, lx1, lz0, lz1) = ocean_land_bbox(&ocean, w, h);
        let lw = (lx1 - lx0) as usize;
        let lh = (lz1 - lz0) as usize;
        let biome_seed = get_start_seed(rec.world_seed, rec.biome_salt);
        let mut land = vec![Biome::NONE; lw * lh];
        map_continent(biome_seed, &mut land, rec.x + lx0, rec.z + lz0, lw, lh);

        let mut out = vec![Biome::NONE; w * h];
        map_ocean_mix(&ocean, &land, &mut out, w, h, lx0, lz0, lw, lh);

        let mut digest: u32 = 0;
        for cell in &out {
            digest ^= hash32(cell.id() as u32);
        }
        assert_eq!(
            digest, rec.digest,
            "map_ocean_mix digest mismatch at record {i} \
             (world={:#x}, biome_salt={:#x}, x={}, z={}, w={}, h={})",
            rec.world_seed, rec.biome_salt, rec.x, rec.z, rec.w, rec.h
        );
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct VoronoiRecord {
    world_seed: u64,
    biome_salt: u64,
    x: i32,
    z: i32,
    w: u32,
    h: u32,
    digest: u32,
    pad: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct VoronoiAccessRecord {
    world_seed: u64,
    x: i32,
    y: i32,
    z: i32,
    x4: i32,
    y4: i32,
    z4: i32,
    pad: u64,
}

#[test]
fn map_voronoi_matches_cubiomes() {
    let records: Vec<VoronoiRecord> = load_fixture("voronoi.bin", 35);
    assert!(!records.is_empty());
    for (i, rec) in records.iter().enumerate() {
        let w = rec.w as usize;
        let h = rec.h as usize;
        // mapVoronoi: x -= 2; z -= 2; px = x >> 2; pz = z >> 2;
        // pw = ((x + w) >> 2) - px + 2; ph = ((z + h) >> 2) - pz + 2.
        let sx = rec.x - 2;
        let sz = rec.z - 2;
        let parent_x = sx >> 2;
        let parent_z = sz >> 2;
        let parent_w = (((sx + w as i32) >> 2) - parent_x + 2) as usize;
        let parent_h = (((sz + h as i32) >> 2) - parent_z + 2) as usize;

        let biome_seed = get_start_seed(rec.world_seed, rec.biome_salt);
        let mut parent = vec![Biome::NONE; parent_w * parent_h];
        map_continent(
            biome_seed,
            &mut parent,
            parent_x,
            parent_z,
            parent_w,
            parent_h,
        );

        let sha = voronoi_sha(rec.world_seed);
        let mut out = vec![Biome::NONE; w * h];
        map_voronoi(
            sha, &parent, parent_x, parent_z, parent_w, parent_h, &mut out, rec.x, rec.z, w, h,
        );

        let mut digest: u32 = 0;
        for cell in &out {
            digest ^= hash32(cell.id() as u32);
        }
        assert_eq!(
            digest, rec.digest,
            "map_voronoi digest mismatch at record {i} \
             (world={:#x}, biome_salt={:#x}, x={}, z={}, w={}, h={})",
            rec.world_seed, rec.biome_salt, rec.x, rec.z, rec.w, rec.h
        );
    }
}

#[test]
fn voronoi_access_3d_matches_cubiomes() {
    let records: Vec<VoronoiAccessRecord> = load_fixture("voronoi_access.bin", 36);
    assert!(!records.is_empty());
    for (i, rec) in records.iter().enumerate() {
        let sha = voronoi_sha(rec.world_seed);
        let (x4, y4, z4) = voronoi_access_3d(sha, rec.x, rec.y, rec.z);
        assert_eq!(
            (x4, y4, z4),
            (rec.x4, rec.y4, rec.z4),
            "voronoi_access_3d mismatch at record {i} \
             (world={:#x}, x={}, y={}, z={})",
            rec.world_seed,
            rec.x,
            rec.y,
            rec.z
        );
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
