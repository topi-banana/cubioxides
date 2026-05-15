//! Reference fixture generator for cubioxides.
//!
//! Links cubiomes via FFI and emits deterministic input/output records
//! that the `cubioxides-core` test suite consumes for parity checks.

#![allow(unsafe_code)]
#![allow(missing_docs)]

use std::ffi::{CStr, c_int};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use bytemuck::{Pod, Zeroable};

/// Discriminant of `MC_1_18` (alias for `MC_1_18_2`) in `cubiomes/biomes.h`.
/// Update alongside the upstream enum whenever cubiomes inserts new versions.
const MC_1_18: c_int = 22;

/// Number of records per fixture. Large enough to expose state-dependent
/// off-by-one bugs while still fitting comfortably in git.
const RECORD_COUNT: u64 = 10_000;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map_or("help", String::as_str);

    match cmd {
        "help" | "--help" | "-h" => {
            print_help();
            ExitCode::SUCCESS
        }
        "verify" => verify_ffi(),
        "rng" => regenerate_rng(),
        "noise" => regenerate_noise(),
        "layers" => regenerate_layers(),
        "regenerate-all" => {
            let r = regenerate_rng();
            if r != ExitCode::SUCCESS {
                return r;
            }
            let r = regenerate_noise();
            if r != ExitCode::SUCCESS {
                return r;
            }
            regenerate_layers()
        }
        unknown => {
            eprintln!("unknown subcommand: {unknown}");
            print_help();
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    eprintln!("Usage: fixtures-gen <subcommand>");
    eprintln!();
    eprintln!("Subcommands:");
    eprintln!("  verify           FFI smoke-test against cubiomes (must print \"1.18\")");
    eprintln!("  rng              Generate RNG fixtures (java / xoroshiro / mc_seed)");
    eprintln!("  noise            Generate noise fixtures (perlin / octave / double_perlin)");
    eprintln!("  layers           Generate layer fixtures (continent)");
    eprintln!("  regenerate-all   Regenerate every fixture under fixtures/");
    eprintln!("  help             Show this help");
}

fn verify_ffi() -> ExitCode {
    let mc_name = unsafe {
        let ptr = ffi::mc2str(MC_1_18);
        if ptr.is_null() {
            eprintln!("cubiomes mc2str returned a null pointer");
            return ExitCode::FAILURE;
        }
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    };
    println!("cubiomes mc2str(MC_1_18) = {mc_name:?}");
    if mc_name != "1.18" {
        eprintln!(
            "verify failed: expected \"1.18\" from cubiomes mc2str, got {mc_name:?}. \
             The MC_1_18 ordinal in fixtures-gen may be out of sync with upstream."
        );
        return ExitCode::FAILURE;
    }
    println!("FFI smoke test passed.");
    ExitCode::SUCCESS
}

fn regenerate_rng() -> ExitCode {
    let fixtures_dir = workspace_root().join("fixtures").join("rng");
    if let Err(err) = fs::create_dir_all(&fixtures_dir) {
        eprintln!("failed to create {}: {err}", fixtures_dir.display());
        return ExitCode::FAILURE;
    }

    if let Err(err) = write_java_fixture(&fixtures_dir.join("java.bin")) {
        eprintln!("java fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_xoroshiro_fixture(&fixtures_dir.join("xoroshiro.bin")) {
        eprintln!("xoroshiro fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_mc_seed_fixture(&fixtures_dir.join("mc_seed.bin")) {
        eprintln!("mc_seed fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    println!(
        "Wrote {RECORD_COUNT} records each into {}",
        fixtures_dir.display()
    );
    ExitCode::SUCCESS
}

fn workspace_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .expect("crate sits inside a workspace")
        .to_path_buf()
}

/// Header preamble for every fixture file. 32 bytes including padding.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Header {
    pub magic: [u8; 4],
    pub format_version: u16,
    pub kind: u16,
    pub record_count: u64,
    pub reserved: [u64; 2],
}

/// Magic bytes identifying a cubioxides fixture (`b"CUBX"`).
pub const MAGIC: [u8; 4] = *b"CUBX";
/// Current fixture format version.
pub const FORMAT_VERSION: u16 = 1;

/// Java RNG record: one input seed, all derived first-call outputs.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct JavaRecord {
    pub seed: u64,
    pub next_32: i32,
    pub next_int_24: i32,
    pub next_long: u64,
    pub next_float_bits: u32,
    pub pad0: u32,
    pub next_double_bits: u64,
    pub skip_42_raw: u64,
}

/// Xoroshiro record.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct XoroshiroRecord {
    pub seed: u64,
    pub state_lo: u64,
    pub state_hi: u64,
    pub next_long: u64,
    pub next_long_j: u64,
    pub next_int_24: i32,
    pub next_int_j_24: i32,
    pub next_double_bits: u64,
    pub next_float_bits: u32,
    pub pad0: u32,
    pub skip_42_lo: u64,
    pub skip_42_hi: u64,
}

/// MC seed pipeline record.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct McSeedRecord {
    pub seed: u64,
    pub salt: u64,
    pub step_seed: u64,
    pub first_int_24: i32,
    pub pad0: u32,
    pub layer_salt: u64,
    pub start_salt: u64,
    pub start_seed: u64,
    pub chunk_seed: u64,
    pub mul_inv: u64,
}

fn write_header<W: Write>(w: &mut W, kind: u16, count: u64) -> std::io::Result<()> {
    let header = Header {
        magic: MAGIC,
        format_version: FORMAT_VERSION,
        kind,
        record_count: count,
        reserved: [0; 2],
    };
    w.write_all(bytemuck::bytes_of(&header))
}

/// Deterministic 64-bit PRNG used to pick fixture seeds.
fn lcg_step(state: u64) -> u64 {
    state.wrapping_mul(0x5851_f42d_4c95_7f2d).wrapping_add(1)
}

fn write_java_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 1, RECORD_COUNT)?;

    let mut rng_state: u64 = 0xdead_beef_cafe;
    for i in 0..RECORD_COUNT {
        rng_state = lcg_step(rng_state);
        let seed = rng_state ^ (i << 1);
        let rec = java_record(seed);
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

fn java_record(seed: u64) -> JavaRecord {
    let mut rec = JavaRecord::zeroed();
    rec.seed = seed;

    // next(32)
    let mut s = unsafe { ffi::cubiomes_set_seed(seed) };
    rec.next_32 = unsafe { ffi::cubiomes_next(&raw mut s, 32) };

    // next_int_24
    s = unsafe { ffi::cubiomes_set_seed(seed) };
    rec.next_int_24 = unsafe { ffi::cubiomes_next_int_24(&raw mut s) };

    // next_long
    s = unsafe { ffi::cubiomes_set_seed(seed) };
    rec.next_long = unsafe { ffi::cubiomes_next_long(&raw mut s) };

    // next_float
    s = unsafe { ffi::cubiomes_set_seed(seed) };
    let f = unsafe { ffi::cubiomes_next_float(&raw mut s) };
    rec.next_float_bits = f.to_bits();

    // next_double
    s = unsafe { ffi::cubiomes_set_seed(seed) };
    let d = unsafe { ffi::cubiomes_next_double(&raw mut s) };
    rec.next_double_bits = d.to_bits();

    // skip_next_n(42) -> raw seed
    s = unsafe { ffi::cubiomes_set_seed(seed) };
    unsafe { ffi::cubiomes_skip_next_n(&raw mut s, 42) };
    rec.skip_42_raw = s;

    rec
}

fn write_xoroshiro_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 2, RECORD_COUNT)?;

    let mut rng_state: u64 = 0x00ab_add0_0d42;
    for i in 0..RECORD_COUNT {
        rng_state = lcg_step(rng_state);
        let seed = rng_state.wrapping_add(i.wrapping_mul(3));
        let rec = xoroshiro_record(seed);
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

fn xoroshiro_record(seed: u64) -> XoroshiroRecord {
    let mut rec = XoroshiroRecord::zeroed();
    rec.seed = seed;

    // After xSetSeed, capture the initial state.
    let mut xr = ffi::Xoroshiro { lo: 0, hi: 0 };
    unsafe { ffi::cubiomes_x_set_seed(&raw mut xr, seed) };
    rec.state_lo = xr.lo;
    rec.state_hi = xr.hi;

    // next_long
    unsafe { ffi::cubiomes_x_set_seed(&raw mut xr, seed) };
    rec.next_long = unsafe { ffi::cubiomes_x_next_long(&raw mut xr) };

    // next_long_j
    unsafe { ffi::cubiomes_x_set_seed(&raw mut xr, seed) };
    rec.next_long_j = unsafe { ffi::cubiomes_x_next_long_j(&raw mut xr) };

    // next_int(24)
    unsafe { ffi::cubiomes_x_set_seed(&raw mut xr, seed) };
    rec.next_int_24 = unsafe { ffi::cubiomes_x_next_int(&raw mut xr, 24) };

    // next_int_j(24)
    unsafe { ffi::cubiomes_x_set_seed(&raw mut xr, seed) };
    rec.next_int_j_24 = unsafe { ffi::cubiomes_x_next_int_j(&raw mut xr, 24) };

    // next_double
    unsafe { ffi::cubiomes_x_set_seed(&raw mut xr, seed) };
    let d = unsafe { ffi::cubiomes_x_next_double(&raw mut xr) };
    rec.next_double_bits = d.to_bits();

    // next_float
    unsafe { ffi::cubiomes_x_set_seed(&raw mut xr, seed) };
    let f = unsafe { ffi::cubiomes_x_next_float(&raw mut xr) };
    rec.next_float_bits = f.to_bits();

    // skip_n(42) then capture state
    unsafe { ffi::cubiomes_x_set_seed(&raw mut xr, seed) };
    unsafe { ffi::cubiomes_x_skip_n(&raw mut xr, 42) };
    rec.skip_42_lo = xr.lo;
    rec.skip_42_hi = xr.hi;

    rec
}

fn write_mc_seed_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 3, RECORD_COUNT)?;

    let mut rng_state: u64 = 0x1234_5678_9abc;
    for i in 0..RECORD_COUNT {
        rng_state = lcg_step(rng_state);
        let seed = rng_state ^ i;
        rng_state = lcg_step(rng_state);
        let salt = rng_state.rotate_left(17);
        let rec = mc_seed_record(seed, salt);
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

fn regenerate_noise() -> ExitCode {
    let fixtures_dir = workspace_root().join("fixtures").join("noise");
    if let Err(err) = fs::create_dir_all(&fixtures_dir) {
        eprintln!("failed to create {}: {err}", fixtures_dir.display());
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_perlin_fixture(&fixtures_dir.join("perlin.bin")) {
        eprintln!("perlin fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_octave_fixture(&fixtures_dir.join("octave.bin")) {
        eprintln!("octave fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_double_perlin_fixture(&fixtures_dir.join("double_perlin.bin")) {
        eprintln!("double_perlin fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    println!("Wrote noise fixtures into {}", fixtures_dir.display());
    ExitCode::SUCCESS
}

/// Perlin noise record (kind = 4). One seed, one (x, y, z, yamp, ymin) sample
/// point, four derived `f64` outputs stored as bit patterns.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct PerlinRecord {
    pub seed: u64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yamp: f64,
    pub ymin: f64,
    pub java_sample_bits: u64,
    pub xoroshiro_sample_bits: u64,
    pub java_simplex_bits: u64,
    pub xoroshiro_simplex_bits: u64,
}

fn write_perlin_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 4, RECORD_COUNT)?;

    let mut rng_state: u64 = 0xc01d_cafe_1337;
    for i in 0..RECORD_COUNT {
        rng_state = lcg_step(rng_state);
        let seed = rng_state ^ i;
        // Spread inputs across a few orders of magnitude so the floor()
        // logic in samplePerlin sees both positive and negative whole-cell
        // boundaries.
        rng_state = lcg_step(rng_state);
        let x = u64_to_double_signed(rng_state) * 1000.0;
        rng_state = lcg_step(rng_state);
        let y = u64_to_double_signed(rng_state) * 16.0;
        rng_state = lcg_step(rng_state);
        let z = u64_to_double_signed(rng_state) * 1000.0;
        // Half the records exercise yamp != 0 (sampleOctaveAmp pathway).
        let (yamp, ymin) = if i % 2 == 0 {
            (0.0, 0.0)
        } else {
            rng_state = lcg_step(rng_state);
            let yamp = (u64_to_double_signed(rng_state).abs() * 4.0).max(0.0001);
            rng_state = lcg_step(rng_state);
            let ymin = u64_to_double_signed(rng_state) * 4.0;
            (yamp, ymin)
        };

        let rec = perlin_record(seed, x, y, z, yamp, ymin);
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

fn u64_to_double_signed(bits: u64) -> f64 {
    // Maps u64 to [-1.0, 1.0) uniformly enough for fixture variety.
    let half = (bits >> 11) as f64 / (1u64 << 53) as f64;
    half * 2.0 - 1.0
}

fn perlin_record(seed: u64, x: f64, y: f64, z: f64, yamp: f64, ymin: f64) -> PerlinRecord {
    let java_sample_bits = unsafe {
        let mut s = ffi::cubiomes_set_seed(seed);
        let mut pn = std::mem::zeroed::<ffi::CPerlinNoise>();
        ffi::perlinInit(&raw mut pn, &raw mut s);
        ffi::samplePerlin(&raw const pn, x, y, z, yamp, ymin).to_bits()
    };
    let xoroshiro_sample_bits = unsafe {
        let mut xr = ffi::Xoroshiro { lo: 0, hi: 0 };
        ffi::cubiomes_x_set_seed(&raw mut xr, seed);
        let mut pn = std::mem::zeroed::<ffi::CPerlinNoise>();
        ffi::xPerlinInit(&raw mut pn, &raw mut xr);
        ffi::samplePerlin(&raw const pn, x, y, z, yamp, ymin).to_bits()
    };
    let java_simplex_bits = unsafe {
        let mut s = ffi::cubiomes_set_seed(seed);
        let mut pn = std::mem::zeroed::<ffi::CPerlinNoise>();
        ffi::perlinInit(&raw mut pn, &raw mut s);
        ffi::sampleSimplex2D(&raw const pn, x, z).to_bits()
    };
    let xoroshiro_simplex_bits = unsafe {
        let mut xr = ffi::Xoroshiro { lo: 0, hi: 0 };
        ffi::cubiomes_x_set_seed(&raw mut xr, seed);
        let mut pn = std::mem::zeroed::<ffi::CPerlinNoise>();
        ffi::xPerlinInit(&raw mut pn, &raw mut xr);
        ffi::sampleSimplex2D(&raw const pn, x, z).to_bits()
    };
    PerlinRecord {
        seed,
        x,
        y,
        z,
        yamp,
        ymin,
        java_sample_bits,
        xoroshiro_sample_bits,
        java_simplex_bits,
        xoroshiro_simplex_bits,
    }
}

#[allow(clippy::too_many_lines)]
fn regenerate_layers() -> ExitCode {
    let fixtures_dir = workspace_root().join("fixtures").join("layers");
    if let Err(err) = fs::create_dir_all(&fixtures_dir) {
        eprintln!("failed to create {}: {err}", fixtures_dir.display());
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_continent_fixture(&fixtures_dir.join("continent.bin")) {
        eprintln!("continent fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_zoom_fixture(&fixtures_dir.join("zoom_fuzzy.bin"), ZoomKind::Fuzzy) {
        eprintln!("zoom_fuzzy fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_zoom_fixture(&fixtures_dir.join("zoom.bin"), ZoomKind::Majority) {
        eprintln!("zoom fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_land_fixture(&fixtures_dir.join("land.bin"), LandKind::Modern) {
        eprintln!("land fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_land_fixture(&fixtures_dir.join("land16.bin"), LandKind::Land16) {
        eprintln!("land16 fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_land_fixture(&fixtures_dir.join("land_b18.bin"), LandKind::B18) {
        eprintln!("land_b18 fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    for kind in [
        SingleHopKind::Island,
        SingleHopKind::Snow16,
        SingleHopKind::Snow,
        SingleHopKind::Special,
        SingleHopKind::Mushroom,
        SingleHopKind::DeepOcean,
    ] {
        let path = fixtures_dir.join(format!("{}.bin", kind.name()));
        if let Err(err) = write_single_hop_fixture(&path, kind) {
            eprintln!("{} fixture failed: {err}", kind.name());
            return ExitCode::FAILURE;
        }
    }
    for kind in [TempKind::Cool, TempKind::Heat] {
        let path = fixtures_dir.join(format!("{}.bin", kind.name()));
        if let Err(err) = write_temp_fixture(&path, kind) {
            eprintln!("{} fixture failed: {err}", kind.name());
            return ExitCode::FAILURE;
        }
    }
    if let Err(err) = write_biome_fixture(&fixtures_dir.join("biome.bin")) {
        eprintln!("biome fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    for kind in [
        FourHopKind::Noise,
        FourHopKind::Bamboo,
        FourHopKind::SwampRiver,
        FourHopKind::Sunflower,
    ] {
        let path = fixtures_dir.join(format!("{}.bin", kind.name()));
        if let Err(err) = write_four_hop_fixture(&path, kind) {
            eprintln!("{} fixture failed: {err}", kind.name());
            return ExitCode::FAILURE;
        }
    }
    if let Err(err) = write_ocean_temp_fixture(&fixtures_dir.join("ocean_temp.bin")) {
        eprintln!("ocean_temp fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_voronoi114_fixture(&fixtures_dir.join("voronoi114.bin")) {
        eprintln!("voronoi114 fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_shore_fixture(&fixtures_dir.join("shore.bin")) {
        eprintln!("shore fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_hills_fixture(&fixtures_dir.join("hills.bin")) {
        eprintln!("hills fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    for kind in [
        PostBiomeKind::River,
        PostBiomeKind::Smooth,
        PostBiomeKind::RiverMix,
    ] {
        let path = fixtures_dir.join(format!("{}.bin", kind.name()));
        if let Err(err) = write_post_biome_fixture(&path, kind) {
            eprintln!("{} fixture failed: {err}", kind.name());
            return ExitCode::FAILURE;
        }
    }
    if let Err(err) = write_ocean_mix_fixture(&fixtures_dir.join("ocean_mix.bin")) {
        eprintln!("ocean_mix fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_voronoi_sha_fixture(&fixtures_dir.join("voronoi_sha.bin")) {
        eprintln!("voronoi_sha fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_voronoi_fixture(&fixtures_dir.join("voronoi.bin")) {
        eprintln!("voronoi fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_voronoi_access_fixture(&fixtures_dir.join("voronoi_access.bin")) {
        eprintln!("voronoi_access fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_layer_stack_fixture(&fixtures_dir.join("layer_stack.bin")) {
        eprintln!("layer_stack fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_gen_area_fixture(&fixtures_dir.join("gen_area.bin")) {
        eprintln!("gen_area fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_gen_area_entry1_fixture(&fixtures_dir.join("gen_area_entry1.bin")) {
        eprintln!("gen_area_entry1 fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    println!("Wrote layer fixtures into {}", fixtures_dir.display());
    ExitCode::SUCCESS
}

/// Layer mapContinent record (kind = 7). Each entry samples a small
/// rectangle and stores a digest of every output cell so the comparison
/// stays O(records) on the parity-test side. Laid out for `Pod` (no
/// implicit padding, total 32 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ContinentRecord {
    pub start_seed: u64,
    pub x: i32,
    pub z: i32,
    pub w: u32,
    pub h: u32,
    /// `hash32`-style fold over every emitted biome ID in row-major order.
    pub digest: u32,
    /// Explicit terminal padding so the struct's size is a multiple of
    /// its alignment (8 bytes); required by `bytemuck::Pod`.
    pub pad: u32,
}

/// Number of `map_continent` samples per fixture. Smaller than `RECORD_COUNT`
/// because each record allocates a `Vec<i32>` of `w * h` cells before
/// digesting; a sweep across many region sizes is plenty for parity.
const CONTINENT_RECORDS: u64 = 4096;

fn write_continent_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 7, CONTINENT_RECORDS)?;

    let mut rng_state: u64 = 0xc0c0_a1ce_b0ba;
    for _ in 0..CONTINENT_RECORDS {
        rng_state = lcg_step(rng_state);
        let start_seed = rng_state;
        rng_state = lcg_step(rng_state);
        // Random rectangle covering both small (1x1) and modest (32x32) cases.
        let w = ((rng_state & 0x1f) as u32) + 1; // 1..=32
        let h = ((rng_state >> 8) & 0x1f) as u32 + 1; // 1..=32
        rng_state = lcg_step(rng_state);
        let x = (rng_state as i32) % 64; // -63..=63 ish
        rng_state = lcg_step(rng_state);
        let z = (rng_state as i32) % 64;
        let rec = continent_record(start_seed, x, z, w, h);
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

fn continent_record(start_seed: u64, x: i32, z: i32, w: u32, h: u32) -> ContinentRecord {
    let mut out: Vec<i32> = vec![0; (w * h) as usize];
    unsafe {
        ffi::cubiomes_call_map_continent(
            start_seed,
            out.as_mut_ptr(),
            x,
            z,
            w as c_int,
            h as c_int,
        );
    }
    let digest = digest_i32_slice(&out);
    ContinentRecord {
        start_seed,
        x,
        z,
        w,
        h,
        digest,
        pad: 0,
    }
}

/// Folds an `i32` slice into a u32 by mixing each value with the same
/// 32-bit mixer cubiomes uses in `tests.c::hash32`.
fn digest_i32_slice(values: &[i32]) -> u32 {
    let mut h: u32 = 0;
    for v in values {
        h ^= hash32(*v as u32);
    }
    h
}

/// Mirror of cubiomes' `hash32` in tests.c (same constants).
fn hash32(mut x: u32) -> u32 {
    x ^= x >> 15;
    x = x.wrapping_mul(0xd168_aaad);
    x ^= x >> 15;
    x = x.wrapping_mul(0xaf72_3597);
    x ^= x >> 15;
    x
}

/// Zoom layer record (kind = 8 for fuzzy, kind = 9 for majority).
///
/// Laid out so the struct size (48 bytes) is a multiple of its 8-byte
/// alignment; `pad` is explicit terminal padding required by `Pod`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ZoomRecord {
    pub world_seed: u64,
    pub parent_salt: u64,
    pub zoom_salt: u64,
    pub x: i32,
    pub z: i32,
    pub w: u32,
    pub h: u32,
    pub digest: u32,
    pub pad: u32,
}

const ZOOM_RECORDS: u64 = 4096;

#[derive(Copy, Clone)]
enum ZoomKind {
    Fuzzy,
    Majority,
}

impl ZoomKind {
    const fn fixture_kind(self) -> u16 {
        match self {
            Self::Fuzzy => 8,
            Self::Majority => 9,
        }
    }
}

fn write_zoom_fixture(path: &Path, kind: ZoomKind) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, kind.fixture_kind(), ZOOM_RECORDS)?;

    let mut rng_state: u64 = 0xfeed_face_beef;
    for _ in 0..ZOOM_RECORDS {
        rng_state = lcg_step(rng_state);
        let world_seed = rng_state;
        rng_state = lcg_step(rng_state);
        let parent_salt = rng_state | 1; // non-zero so setLayerSeed takes the salt branch
        rng_state = lcg_step(rng_state);
        let zoom_salt = rng_state | 1;
        rng_state = lcg_step(rng_state);
        let w = ((rng_state & 0x1f) as u32) + 2; // 2..=33
        let h = ((rng_state >> 8) & 0x1f) as u32 + 2;
        rng_state = lcg_step(rng_state);
        let x = (rng_state as i32) % 64;
        rng_state = lcg_step(rng_state);
        let z = (rng_state as i32) % 64;
        let rec = zoom_record(kind, world_seed, parent_salt, zoom_salt, x, z, w, h);
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

fn zoom_record(
    kind: ZoomKind,
    world_seed: u64,
    parent_salt: u64,
    zoom_salt: u64,
    x: i32,
    z: i32,
    w: u32,
    h: u32,
) -> ZoomRecord {
    // cubiomes reuses the out buffer as scratch for the parent layer
    // output (pW * pH cells) plus the upscaled grid (newW * newH = 4*pW*pH
    // cells), so allocate the full 5 * pW * pH cells here. The window of
    // interest is still out[0..w*h] after the call returns.
    let p_w = (((x + w as i32) >> 1) - (x >> 1) + 1) as usize;
    let p_h = (((z + h as i32) >> 1) - (z >> 1) + 1) as usize;
    let buffer_size = 5 * p_w * p_h;
    let mut out: Vec<i32> = vec![0; buffer_size];
    unsafe {
        match kind {
            ZoomKind::Fuzzy => ffi::cubiomes_call_map_zoom_fuzzy(
                world_seed,
                parent_salt,
                zoom_salt,
                out.as_mut_ptr(),
                x,
                z,
                w as c_int,
                h as c_int,
            ),
            ZoomKind::Majority => ffi::cubiomes_call_map_zoom(
                world_seed,
                parent_salt,
                zoom_salt,
                out.as_mut_ptr(),
                x,
                z,
                w as c_int,
                h as c_int,
            ),
        }
    }
    let cells = (w * h) as usize;
    let digest = digest_i32_slice(&out[..cells]);
    ZoomRecord {
        world_seed,
        parent_salt,
        zoom_salt,
        x,
        z,
        w,
        h,
        digest,
        pad: 0,
    }
}

/// `map_land` record (kind = 10). Same shape as `ZoomRecord`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LandRecord {
    pub world_seed: u64,
    pub parent_salt: u64,
    pub land_salt: u64,
    pub x: i32,
    pub z: i32,
    pub w: u32,
    pub h: u32,
    pub digest: u32,
    pub pad: u32,
}

const LAND_RECORDS: u64 = 4096;

#[derive(Copy, Clone)]
enum LandKind {
    Modern,
    Land16,
    B18,
}

impl LandKind {
    const fn fixture_kind(self) -> u16 {
        match self {
            Self::Modern => 10,
            Self::Land16 => 11,
            Self::B18 => 12,
        }
    }

    const fn seed_xor(self) -> u64 {
        match self {
            Self::Modern => 0,
            Self::Land16 => 0xa5a5_a5a5_a5a5_a5a5,
            Self::B18 => 0x3c3c_3c3c_3c3c_3c3c,
        }
    }
}

fn write_land_fixture(path: &Path, kind: LandKind) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, kind.fixture_kind(), LAND_RECORDS)?;

    let mut rng_state: u64 = 0x0001_eafb_00b5 ^ kind.seed_xor();
    for _ in 0..LAND_RECORDS {
        rng_state = lcg_step(rng_state);
        let world_seed = rng_state;
        rng_state = lcg_step(rng_state);
        let parent_salt = rng_state | 1;
        rng_state = lcg_step(rng_state);
        let land_salt = rng_state | 1;
        rng_state = lcg_step(rng_state);
        let w = ((rng_state & 0x1f) as u32) + 2;
        let h = ((rng_state >> 8) & 0x1f) as u32 + 2;
        rng_state = lcg_step(rng_state);
        let x = (rng_state as i32) % 64;
        rng_state = lcg_step(rng_state);
        let z = (rng_state as i32) % 64;
        let rec = land_record(kind, world_seed, parent_salt, land_salt, x, z, w, h);
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

fn land_record(
    kind: LandKind,
    world_seed: u64,
    parent_salt: u64,
    land_salt: u64,
    x: i32,
    z: i32,
    w: u32,
    h: u32,
) -> LandRecord {
    // map_land's out buffer must hold the parent's (w + 2) x (h + 2) cells
    // before the final w * h window overwrites the first w * h cells.
    let p_cells = ((w + 2) * (h + 2)) as usize;
    let mut out: Vec<i32> = vec![0; p_cells];
    unsafe {
        match kind {
            LandKind::Modern => ffi::cubiomes_call_map_land(
                world_seed,
                parent_salt,
                land_salt,
                out.as_mut_ptr(),
                x,
                z,
                w as c_int,
                h as c_int,
            ),
            LandKind::Land16 => ffi::cubiomes_call_map_land16(
                world_seed,
                parent_salt,
                land_salt,
                out.as_mut_ptr(),
                x,
                z,
                w as c_int,
                h as c_int,
            ),
            LandKind::B18 => ffi::cubiomes_call_map_land_b18(
                world_seed,
                parent_salt,
                land_salt,
                out.as_mut_ptr(),
                x,
                z,
                w as c_int,
                h as c_int,
            ),
        }
    }
    let cells = (w * h) as usize;
    let digest = digest_i32_slice(&out[..cells]);
    LandRecord {
        world_seed,
        parent_salt,
        land_salt,
        x,
        z,
        w,
        h,
        digest,
        pad: 0,
    }
}

/// Generic single-hop layer record (kind 13..=18). Same shape as
/// `LandRecord`; the parent is always `mapContinent`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SingleHopRecord {
    pub world_seed: u64,
    pub parent_salt: u64,
    pub child_salt: u64,
    pub x: i32,
    pub z: i32,
    pub w: u32,
    pub h: u32,
    pub digest: u32,
    pub pad: u32,
}

const SINGLE_HOP_RECORDS: u64 = 4096;

#[derive(Copy, Clone)]
enum SingleHopKind {
    Island,
    Snow16,
    Snow,
    Special,
    Mushroom,
    DeepOcean,
}

impl SingleHopKind {
    const fn fixture_kind(self) -> u16 {
        match self {
            Self::Island => 13,
            Self::Snow16 => 14,
            Self::Snow => 15,
            Self::Special => 16,
            Self::Mushroom => 17,
            Self::DeepOcean => 18,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Island => "island",
            Self::Snow16 => "snow16",
            Self::Snow => "snow",
            Self::Special => "special",
            Self::Mushroom => "mushroom",
            Self::DeepOcean => "deep_ocean",
        }
    }

    const fn rng_seed(self) -> u64 {
        match self {
            Self::Island => 0x100a_d100_0a01,
            Self::Snow16 => 0x5061_71e5_1601,
            Self::Snow => 0x5061_71e5_0001,
            Self::Special => 0x5e1a_15e1_0001,
            Self::Mushroom => 0xb00b_a1de_a015,
            Self::DeepOcean => 0xdee5_b00b_a153,
        }
    }
}

fn write_single_hop_fixture(path: &Path, kind: SingleHopKind) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, kind.fixture_kind(), SINGLE_HOP_RECORDS)?;

    let mut rng_state: u64 = kind.rng_seed();
    for _ in 0..SINGLE_HOP_RECORDS {
        rng_state = lcg_step(rng_state);
        let world_seed = rng_state;
        rng_state = lcg_step(rng_state);
        let parent_salt = rng_state | 1;
        rng_state = lcg_step(rng_state);
        let child_salt = rng_state | 1;
        rng_state = lcg_step(rng_state);
        let w = ((rng_state & 0x1f) as u32) + 2;
        let h = ((rng_state >> 8) & 0x1f) as u32 + 2;
        rng_state = lcg_step(rng_state);
        let x = (rng_state as i32) % 64;
        rng_state = lcg_step(rng_state);
        let z = (rng_state as i32) % 64;
        let rec = single_hop_record(kind, world_seed, parent_salt, child_salt, x, z, w, h);
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

fn single_hop_record(
    kind: SingleHopKind,
    world_seed: u64,
    parent_salt: u64,
    child_salt: u64,
    x: i32,
    z: i32,
    w: u32,
    h: u32,
) -> SingleHopRecord {
    // mapSpecial reads a (w, h) parent (no padding); the others read
    // (w + 2, h + 2). Use the larger size for all of them; cubiomes
    // ignores the surplus cells.
    let p_cells = ((w + 2) * (h + 2)) as usize;
    let mut out: Vec<i32> = vec![0; p_cells];
    unsafe {
        let dispatch = match kind {
            SingleHopKind::Island => ffi::cubiomes_call_map_island,
            SingleHopKind::Snow16 => ffi::cubiomes_call_map_snow16,
            SingleHopKind::Snow => ffi::cubiomes_call_map_snow,
            SingleHopKind::Special => ffi::cubiomes_call_map_special,
            SingleHopKind::Mushroom => ffi::cubiomes_call_map_mushroom,
            SingleHopKind::DeepOcean => ffi::cubiomes_call_map_deep_ocean,
        };
        dispatch(
            world_seed,
            parent_salt,
            child_salt,
            out.as_mut_ptr(),
            x,
            z,
            w as c_int,
            h as c_int,
        );
    }
    let cells = (w * h) as usize;
    let digest = digest_i32_slice(&out[..cells]);
    SingleHopRecord {
        world_seed,
        parent_salt,
        child_salt,
        x,
        z,
        w,
        h,
        digest,
        pad: 0,
    }
}

/// 3-hop layer record (kind 19 = cool, 20 = heat). The chain is
/// `mapContinent -> mapSnow -> target`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct TempRecord {
    pub world_seed: u64,
    pub continent_salt: u64,
    pub snow_salt: u64,
    pub child_salt: u64,
    pub x: i32,
    pub z: i32,
    pub w: u32,
    pub h: u32,
    pub digest: u32,
    pub pad: u32,
}

const TEMP_RECORDS: u64 = 4096;

#[derive(Copy, Clone)]
enum TempKind {
    Cool,
    Heat,
}

impl TempKind {
    const fn fixture_kind(self) -> u16 {
        match self {
            Self::Cool => 19,
            Self::Heat => 20,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Cool => "cool",
            Self::Heat => "heat",
        }
    }

    const fn rng_seed(self) -> u64 {
        match self {
            Self::Cool => 0xc001_c00f_a001,
            Self::Heat => 0xfeed_b0ba_f0e1,
        }
    }
}

fn write_temp_fixture(path: &Path, kind: TempKind) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, kind.fixture_kind(), TEMP_RECORDS)?;

    let mut rng_state: u64 = kind.rng_seed();
    for _ in 0..TEMP_RECORDS {
        rng_state = lcg_step(rng_state);
        let world_seed = rng_state;
        rng_state = lcg_step(rng_state);
        let continent_salt = rng_state | 1;
        rng_state = lcg_step(rng_state);
        let snow_salt = rng_state | 1;
        rng_state = lcg_step(rng_state);
        let child_salt = rng_state | 1;
        rng_state = lcg_step(rng_state);
        let w = ((rng_state & 0x1f) as u32) + 2;
        let h = ((rng_state >> 8) & 0x1f) as u32 + 2;
        rng_state = lcg_step(rng_state);
        let x = (rng_state as i32) % 64;
        rng_state = lcg_step(rng_state);
        let z = (rng_state as i32) % 64;
        let rec = temp_record(
            kind,
            world_seed,
            continent_salt,
            snow_salt,
            child_salt,
            x,
            z,
            w,
            h,
        );
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

fn temp_record(
    kind: TempKind,
    world_seed: u64,
    continent_salt: u64,
    snow_salt: u64,
    child_salt: u64,
    x: i32,
    z: i32,
    w: u32,
    h: u32,
) -> TempRecord {
    // 3-hop chain: each layer adds a 1-cell border to the parent
    // request, so the deepest layer (mapContinent) writes a
    // (w+4, h+4) buffer. Pad by another 2 cells for safety.
    let p_cells = ((w + 6) * (h + 6)) as usize;
    let mut out: Vec<i32> = vec![0; p_cells];
    unsafe {
        let dispatch = match kind {
            TempKind::Cool => ffi::cubiomes_call_map_cool,
            TempKind::Heat => ffi::cubiomes_call_map_heat,
        };
        dispatch(
            world_seed,
            continent_salt,
            snow_salt,
            child_salt,
            out.as_mut_ptr(),
            x,
            z,
            w as c_int,
            h as c_int,
        );
    }
    let cells = (w * h) as usize;
    let digest = digest_i32_slice(&out[..cells]);
    TempRecord {
        world_seed,
        continent_salt,
        snow_salt,
        child_salt,
        x,
        z,
        w,
        h,
        digest,
        pad: 0,
    }
}

/// `map_biome` record (kind = 21). Same layout as `TempRecord` but
/// the underlying chain ends at `mapBiome` (which produces real biome
/// IDs from the temperature-category grid emitted by `mapSnow`).
/// The MC version is fixed at `MC_1_7` (ordinal = 10) — the smallest
/// 1.7+ value, which exercises the modern biome-selection path.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BiomeRecord {
    pub world_seed: u64,
    pub continent_salt: u64,
    pub snow_salt: u64,
    pub biome_salt: u64,
    pub x: i32,
    pub z: i32,
    pub w: u32,
    pub h: u32,
    pub digest: u32,
    pub pad: u32,
}

const BIOME_RECORDS: u64 = 4096;
/// cubiomes' `MC_1_7` ordinal (matches `MCVersion::V1_7.ord()`).
const MC_1_7: c_int = 10;

fn write_biome_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 21, BIOME_RECORDS)?;

    let mut rng_state: u64 = 0xb10e_b10e_b10e;
    for _ in 0..BIOME_RECORDS {
        rng_state = lcg_step(rng_state);
        let world_seed = rng_state;
        rng_state = lcg_step(rng_state);
        let continent_salt = rng_state | 1;
        rng_state = lcg_step(rng_state);
        let snow_salt = rng_state | 1;
        rng_state = lcg_step(rng_state);
        let biome_salt = rng_state | 1;
        rng_state = lcg_step(rng_state);
        let w = ((rng_state & 0x1f) as u32) + 2;
        let h = ((rng_state >> 8) & 0x1f) as u32 + 2;
        rng_state = lcg_step(rng_state);
        let x = (rng_state as i32) % 64;
        rng_state = lcg_step(rng_state);
        let z = (rng_state as i32) % 64;
        let rec = biome_record(
            world_seed,
            continent_salt,
            snow_salt,
            biome_salt,
            x,
            z,
            w,
            h,
        );
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

fn biome_record(
    world_seed: u64,
    continent_salt: u64,
    snow_salt: u64,
    biome_salt: u64,
    x: i32,
    z: i32,
    w: u32,
    h: u32,
) -> BiomeRecord {
    // map_biome reads a (w, h) parent (no padding), but the underlying
    // mapSnow + mapContinent chain demands (w+4, h+4) cells at most.
    let p_cells = ((w + 6) * (h + 6)) as usize;
    let mut out: Vec<i32> = vec![0; p_cells];
    unsafe {
        ffi::cubiomes_call_map_biome(
            world_seed,
            MC_1_7,
            continent_salt,
            snow_salt,
            biome_salt,
            out.as_mut_ptr(),
            x,
            z,
            w as c_int,
            h as c_int,
        );
    }
    let cells = (w * h) as usize;
    let digest = digest_i32_slice(&out[..cells]);
    BiomeRecord {
        world_seed,
        continent_salt,
        snow_salt,
        biome_salt,
        x,
        z,
        w,
        h,
        digest,
        pad: 0,
    }
}

/// 4-hop layer record (kinds 22..=25). Chain: continent -> snow ->
/// biome -> target, all with `mc = MC_1_7`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct FourHopRecord {
    pub world_seed: u64,
    pub continent_salt: u64,
    pub snow_salt: u64,
    pub biome_salt: u64,
    pub child_salt: u64,
    pub x: i32,
    pub z: i32,
    pub w: u32,
    pub h: u32,
    pub digest: u32,
    pub pad: u32,
}

const FOUR_HOP_RECORDS: u64 = 4096;

#[derive(Copy, Clone)]
enum FourHopKind {
    Noise,
    Bamboo,
    SwampRiver,
    Sunflower,
}

impl FourHopKind {
    const fn fixture_kind(self) -> u16 {
        match self {
            Self::Noise => 22,
            Self::Bamboo => 23,
            Self::SwampRiver => 24,
            Self::Sunflower => 25,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Noise => "noise",
            Self::Bamboo => "bamboo",
            Self::SwampRiver => "swamp_river",
            Self::Sunflower => "sunflower_layer",
        }
    }

    const fn rng_seed(self) -> u64 {
        match self {
            Self::Noise => 0x4001_5e4e_4e01,
            Self::Bamboo => 0xba11_b00b_5350,
            Self::SwampRiver => 0x5a73_4137_1733,
            Self::Sunflower => 0x5111_4f10_1011,
        }
    }
}

fn write_four_hop_fixture(path: &Path, kind: FourHopKind) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, kind.fixture_kind(), FOUR_HOP_RECORDS)?;

    let mut rng_state: u64 = kind.rng_seed();
    for _ in 0..FOUR_HOP_RECORDS {
        rng_state = lcg_step(rng_state);
        let world_seed = rng_state;
        rng_state = lcg_step(rng_state);
        let continent_salt = rng_state | 1;
        rng_state = lcg_step(rng_state);
        let snow_salt = rng_state | 1;
        rng_state = lcg_step(rng_state);
        let biome_salt = rng_state | 1;
        rng_state = lcg_step(rng_state);
        let child_salt = rng_state | 1;
        rng_state = lcg_step(rng_state);
        let w = ((rng_state & 0xf) as u32) + 2;
        let h = ((rng_state >> 8) & 0xf) as u32 + 2;
        rng_state = lcg_step(rng_state);
        let x = (rng_state as i32) % 64;
        rng_state = lcg_step(rng_state);
        let z = (rng_state as i32) % 64;
        let rec = four_hop_record(
            kind,
            world_seed,
            continent_salt,
            snow_salt,
            biome_salt,
            child_salt,
            x,
            z,
            w,
            h,
        );
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

fn four_hop_record(
    kind: FourHopKind,
    world_seed: u64,
    continent_salt: u64,
    snow_salt: u64,
    biome_salt: u64,
    child_salt: u64,
    x: i32,
    z: i32,
    w: u32,
    h: u32,
) -> FourHopRecord {
    // 4-hop chain needs (w+6, h+6) cells at most (continent reads
    // (w+6) for snow which reads (w+4) for biome which reads (w, h)
    // for the child).
    let p_cells = ((w + 8) * (h + 8)) as usize;
    let mut out: Vec<i32> = vec![0; p_cells];
    unsafe {
        let dispatch = match kind {
            FourHopKind::Noise => ffi::cubiomes_call_map_noise,
            FourHopKind::Bamboo => ffi::cubiomes_call_map_bamboo,
            FourHopKind::SwampRiver => ffi::cubiomes_call_map_swamp_river,
            FourHopKind::Sunflower => ffi::cubiomes_call_map_sunflower,
        };
        dispatch(
            world_seed,
            MC_1_7,
            continent_salt,
            snow_salt,
            biome_salt,
            child_salt,
            out.as_mut_ptr(),
            x,
            z,
            w as c_int,
            h as c_int,
        );
    }
    let cells = (w * h) as usize;
    let digest = digest_i32_slice(&out[..cells]);
    FourHopRecord {
        world_seed,
        continent_salt,
        snow_salt,
        biome_salt,
        child_salt,
        x,
        z,
        w,
        h,
        digest,
        pad: 0,
    }
}

/// Generic record for layers reading 1 or 2 `mapContinent` parents:
/// `mapRiver` (kind = 30), `mapSmooth` (kind = 31), `mapRiverMix`
/// (kind = 32). For river / smooth the `secondary_salt` field is
/// unused (set to zero in the writer).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct PostBiomeRecord {
    pub world_seed: u64,
    pub primary_salt: u64,
    pub secondary_salt: u64,
    pub target_salt: u64,
    pub x: i32,
    pub z: i32,
    pub w: u32,
    pub h: u32,
    pub digest: u32,
    pub pad: u32,
}

const POST_BIOME_RECORDS: u64 = 4096;

#[derive(Copy, Clone)]
enum PostBiomeKind {
    River,
    Smooth,
    RiverMix,
}

impl PostBiomeKind {
    const fn fixture_kind(self) -> u16 {
        match self {
            Self::River => 30,
            Self::Smooth => 31,
            Self::RiverMix => 32,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::River => "river",
            Self::Smooth => "smooth",
            Self::RiverMix => "river_mix",
        }
    }

    const fn rng_seed(self) -> u64 {
        match self {
            Self::River => 0x1131_e1ec_0001,
            Self::Smooth => 0x5300_0073_4001,
            Self::RiverMix => 0x113c_e1ec_4117,
        }
    }
}

fn write_post_biome_fixture(path: &Path, kind: PostBiomeKind) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, kind.fixture_kind(), POST_BIOME_RECORDS)?;

    let mut rng_state: u64 = kind.rng_seed();
    for _ in 0..POST_BIOME_RECORDS {
        rng_state = lcg_step(rng_state);
        let world_seed = rng_state;
        rng_state = lcg_step(rng_state);
        let primary_salt = rng_state | 1;
        rng_state = lcg_step(rng_state);
        let secondary_salt = rng_state | 1;
        rng_state = lcg_step(rng_state);
        let target_salt = rng_state | 1;
        rng_state = lcg_step(rng_state);
        let w = ((rng_state & 0x1f) as u32) + 2;
        let h = ((rng_state >> 8) & 0x1f) as u32 + 2;
        rng_state = lcg_step(rng_state);
        let x = (rng_state as i32) % 64;
        rng_state = lcg_step(rng_state);
        let z = (rng_state as i32) % 64;
        let rec = post_biome_record(
            kind,
            world_seed,
            primary_salt,
            secondary_salt,
            target_salt,
            x,
            z,
            w,
            h,
        );
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

fn post_biome_record(
    kind: PostBiomeKind,
    world_seed: u64,
    primary_salt: u64,
    secondary_salt: u64,
    target_salt: u64,
    x: i32,
    z: i32,
    w: u32,
    h: u32,
) -> PostBiomeRecord {
    let p_cells = ((w + 2) * (h + 2)) as usize;
    let mut out: Vec<i32> = vec![0; p_cells * 3];
    unsafe {
        match kind {
            PostBiomeKind::River => ffi::cubiomes_call_map_river(
                world_seed,
                MC_1_18_C,
                primary_salt,
                target_salt,
                out.as_mut_ptr(),
                x,
                z,
                w as c_int,
                h as c_int,
            ),
            PostBiomeKind::Smooth => ffi::cubiomes_call_map_smooth(
                world_seed,
                MC_1_18_C,
                primary_salt,
                target_salt,
                out.as_mut_ptr(),
                x,
                z,
                w as c_int,
                h as c_int,
            ),
            PostBiomeKind::RiverMix => ffi::cubiomes_call_map_river_mix(
                world_seed,
                MC_1_18_C,
                primary_salt,
                secondary_salt,
                target_salt,
                out.as_mut_ptr(),
                x,
                z,
                w as c_int,
                h as c_int,
            ),
        }
    }
    let digest = digest_i32_slice(&out[..(w * h) as usize]);
    PostBiomeRecord {
        world_seed,
        primary_salt,
        secondary_salt,
        target_salt,
        x,
        z,
        w,
        h,
        digest,
        pad: 0,
    }
}

/// `genArea` at `entry_1` (Voronoi) parity record (kind = 39). One
/// record per `(mc, world_seed)` tuple — runs cubiomes' `genArea`
/// against the per-version Voronoi entry, which exercises the entire
/// DAG end-to-end for that MC. `(x, z, w, h)` is aligned to 4 so
/// cubiomes writes every output cell (avoiding the stale-parent
/// scratch quirk that bites Voronoi at unaligned origins).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GenAreaEntry1Record {
    pub mc: u32,
    pub large_biomes: u32,
    pub world_seed: u64,
    pub x: i32,
    pub z: i32,
    pub w: u32,
    pub h: u32,
    pub digest: u32,
    pub pad: u32,
}

const GEN_AREA_ENTRY1_RECORDS: u64 = 384;

fn write_gen_area_entry1_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 39, GEN_AREA_ENTRY1_RECORDS)?;

    // MC ordinals that exercise distinct DAG branches:
    // B1.8 (2), 1.0 (3), 1.1 (4), 1.6 (9), 1.7 (10), 1.12 (15),
    // 1.13 (16), 1.14 (17), 1.18 (22), 1.20 (25). The pre-1.13
    // branches all share the river+smooth tail with a Voronoi114
    // capstone; 1.13+ adds the ocean variants. 1.14 toggles Bamboo
    // on, 1.15+ switches to SHA-driven Voronoi.
    let mc_versions: [i32; 10] = [2, 3, 4, 9, 10, 15, 16, 17, 22, 25];

    let mut rng_state: u64 = 0x070_e1a_1e1_777;
    for _ in 0..GEN_AREA_ENTRY1_RECORDS {
        rng_state = lcg_step(rng_state);
        let world_seed = rng_state;
        rng_state = lcg_step(rng_state);
        let mc = mc_versions[(rng_state as usize) % mc_versions.len()];
        rng_state = lcg_step(rng_state);
        let large_biomes = (rng_state & 1) as c_int;
        rng_state = lcg_step(rng_state);
        // Align to 4 (Voronoi-friendly) and keep dimensions modest so
        // chains down to 1:4096 still resolve to non-trivial output.
        let w = (((rng_state & 0xf) as u32) + 4) & !3;
        let h = (((rng_state >> 8) & 0xf) as u32 + 4) & !3;
        rng_state = lcg_step(rng_state);
        let x = ((rng_state as i32) % 32) & !3;
        rng_state = lcg_step(rng_state);
        let z = ((rng_state as i32) % 32) & !3;

        let cells = (w as usize) * (h as usize) + ((w as usize + 32) * (h as usize + 32)) * 2;
        let mut out: Vec<i32> = vec![0; cells];
        unsafe {
            ffi::cubiomes_call_gen_area_at_entry1(
                mc,
                large_biomes,
                world_seed,
                out.as_mut_ptr(),
                x,
                z,
                w as c_int,
                h as c_int,
            );
        }
        let digest = digest_i32_slice(&out[..(w * h) as usize]);
        let rec = GenAreaEntry1Record {
            mc: mc as u32,
            large_biomes: large_biomes as u32,
            world_seed,
            x,
            z,
            w,
            h,
            digest,
            pad: 0,
        };
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

/// `genArea` parity record (kind = 38). Runs cubiomes' `genArea` on a
/// freshly built `LayerStack` for an arbitrary (`mc`, `world_seed`,
/// `layer_id`) tuple and stores the digest. Padded so `Pod` is happy.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GenAreaRecord {
    pub mc: u32,
    pub large_biomes: u32,
    pub world_seed: u64,
    pub layer_id: u32,
    pub x: i32,
    pub z: i32,
    pub w: u32,
    pub h: u32,
    pub digest: u32,
}

const GEN_AREA_RECORDS: u64 = 512;

fn write_gen_area_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 38, GEN_AREA_RECORDS)?;

    // Layer IDs picked to exercise each LayerOp dispatch arm at least
    // once in the 1.18 DAG: Continent (0), ZoomFuzzy (3), Land (4),
    // Snow (10), Cool (12), Special (14), Mushroom (19), DeepOcean
    // (20), Biome (21), Bamboo (22), BiomeEdge (25), Noise (26),
    // Hills (29), Sunflower (30), Shore (34), Smooth (38), River (45),
    // RiverMix (47), OceanMix (55), Voronoi1 (56).
    let layer_ids: [i32; 20] = [
        0, 3, 4, 10, 12, 14, 19, 20, 21, 22, 25, 26, 29, 30, 34, 38, 45, 47, 55, 56,
    ];

    let mut rng_state: u64 = 0x6_ea7_ea7_777;
    for _ in 0..GEN_AREA_RECORDS {
        rng_state = lcg_step(rng_state);
        let world_seed = rng_state;
        rng_state = lcg_step(rng_state);
        let mc: i32 = 22; // 1.18 — broadest coverage; other MCs land in follow-ups.
        let large_biomes = 0;
        rng_state = lcg_step(rng_state);
        let layer_id = layer_ids[(rng_state as usize) % layer_ids.len()];
        let (x, z, w, h) = sample_dims_for_layer(layer_id, &mut rng_state);

        // Allocate generously — cubiomes' genArea reads its own
        // scratch beyond w*h for some layers.
        let cells = (w as usize) * (h as usize) + ((w as usize + 32) * (h as usize + 32)) * 2;
        let mut out: Vec<i32> = vec![0; cells];
        unsafe {
            ffi::cubiomes_call_gen_area_at(
                mc,
                large_biomes,
                world_seed,
                layer_id,
                out.as_mut_ptr(),
                x,
                z,
                w as c_int,
                h as c_int,
            );
        }
        let digest = digest_i32_slice(&out[..(w * h) as usize]);
        let rec = GenAreaRecord {
            mc: mc as u32,
            large_biomes: large_biomes as u32,
            world_seed,
            layer_id: layer_id as u32,
            x,
            z,
            w,
            h,
            digest,
        };
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

fn sample_dims_for_layer(layer_id: i32, rng_state: &mut u64) -> (i32, i32, u32, u32) {
    // For Voronoi (1:1 output reading a 1:4 parent), cubiomes leaves
    // cells outside the 4-block-aligned grid as stale parent
    // scratch. Align x/z/w/h to multiples of 4 so every output cell
    // is actually written — otherwise the digest would compare
    // garbage.
    if layer_id == 56 {
        *rng_state = lcg_step(*rng_state);
        let w = (((*rng_state & 0xf) as u32) + 4) & !3;
        let h = (((*rng_state >> 8) & 0xf) as u32 + 4) & !3;
        *rng_state = lcg_step(*rng_state);
        let x = ((*rng_state as i32) % 32) & !3;
        *rng_state = lcg_step(*rng_state);
        let z = ((*rng_state as i32) % 32) & !3;
        (x, z, w, h)
    } else {
        *rng_state = lcg_step(*rng_state);
        let w = ((*rng_state & 0xf) as u32) + 4;
        let h = ((*rng_state >> 8) & 0xf) as u32 + 4;
        *rng_state = lcg_step(*rng_state);
        let x = (*rng_state as i32) % 64;
        *rng_state = lcg_step(*rng_state);
        let z = (*rng_state as i32) % 64;
        (x, z, w, h)
    }
}

/// `setupLayerStack` + `setLayerSeed` parity record (kind = 37).
/// Header of `(mc, large_biomes, world_seed)` is followed inline by
/// `L_NUM_C * 3` `u64`s — `(layer_salt, start_salt, start_seed)` for
/// each layer slot in cubiomes' index order. `L_NUM_C = 61` as of the
/// upstream snapshot.
const L_NUM_C: usize = 61;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LayerStackHeader {
    pub mc: u32,
    pub large_biomes: u32,
    pub world_seed: u64,
}

const LAYER_STACK_RECORDS: u64 = 192;

fn write_layer_stack_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 37, LAYER_STACK_RECORDS)?;

    let mc_versions: [i32; 12] = [
        2,  // B1_8
        3,  // 1.0
        4,  // 1.1
        5,  // 1.2
        6,  // 1.3
        9,  // 1.6
        10, // 1.7
        15, // 1.12
        16, // 1.13
        17, // 1.14
        22, // 1.18
        25, // 1.20
    ];

    let mut rng_state: u64 = 0x1abc_75ac_4001;
    for _ in 0..LAYER_STACK_RECORDS {
        rng_state = lcg_step(rng_state);
        let world_seed = rng_state;
        rng_state = lcg_step(rng_state);
        let mc_idx = (rng_state as usize) % mc_versions.len();
        let mc = mc_versions[mc_idx];
        rng_state = lcg_step(rng_state);
        let large_biomes = (rng_state & 1) as u32;

        let mut buf: Vec<u64> = vec![0; L_NUM_C * 3];
        unsafe {
            ffi::cubiomes_call_dump_layer_stack(
                mc,
                large_biomes as c_int,
                world_seed,
                buf.as_mut_ptr(),
            );
        }

        let header = LayerStackHeader {
            mc: mc as u32,
            large_biomes,
            world_seed,
        };
        file.write_all(bytemuck::bytes_of(&header))?;
        file.write_all(bytemuck::cast_slice(&buf))?;
    }
    file.flush()
}

/// `mapVoronoi` (1.0-1.14) record (kind = 35). Layer chain is
/// `mapContinent` (with `biome_salt`) feeding `mapVoronoi` (SHA-driven
/// via `LAYER_INIT_SHA = ~0`). MC = 1.14 effectively (1.0-1.14 share
/// the same Voronoi function).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct VoronoiRecord {
    pub world_seed: u64,
    pub biome_salt: u64,
    pub x: i32,
    pub z: i32,
    pub w: u32,
    pub h: u32,
    pub digest: u32,
    pub pad: u32,
}

const VORONOI_RECORDS: u64 = 1024;

fn write_voronoi_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 35, VORONOI_RECORDS)?;

    let mut rng_state: u64 = 0xdeaf_b00b_77f1;
    for _ in 0..VORONOI_RECORDS {
        rng_state = lcg_step(rng_state);
        let world_seed = rng_state;
        rng_state = lcg_step(rng_state);
        let biome_salt = rng_state | 1;
        rng_state = lcg_step(rng_state);
        // cubiomes' `mapVoronoi` only writes cells in 4x4 parent
        // blocks whose `j4`/`i4` offsets land inside `[0, h)` /
        // `[0, w)`. Aligning `(x, z, w, h)` to 4 guarantees every
        // output cell is covered. We allow 0 as a "no-op" zero too.
        let w = (((rng_state & 0xf) as u32) + 4) & !3;
        let h = (((rng_state >> 8) & 0xf) as u32 + 4) & !3;
        rng_state = lcg_step(rng_state);
        let x = ((rng_state as i32) % 32) & !3;
        rng_state = lcg_step(rng_state);
        let z = ((rng_state as i32) % 32) & !3;
        let rec = voronoi_record(world_seed, biome_salt, x, z, w, h);
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

fn voronoi_record(
    world_seed: u64,
    biome_salt: u64,
    x: i32,
    z: i32,
    w: u32,
    h: u32,
) -> VoronoiRecord {
    // cubiomes uses `out + w*h` as scratch for the parent grid, which
    // is `pw * ph` int32 cells. Worst case pw ~ (w >> 2) + 3, similarly
    // for ph; pad generously.
    let cells = (w as usize * h as usize) + ((w as usize + 16) * (h as usize + 16));
    let mut out: Vec<i32> = vec![0; cells];
    unsafe {
        ffi::cubiomes_call_map_voronoi(
            world_seed,
            biome_salt,
            out.as_mut_ptr(),
            x,
            z,
            w as c_int,
            h as c_int,
        );
    }
    let digest = digest_i32_slice(&out[..(w * h) as usize]);
    VoronoiRecord {
        world_seed,
        biome_salt,
        x,
        z,
        w,
        h,
        digest,
        pad: 0,
    }
}

/// `voronoiAccess3D` record (kind = 36). For each `(world_seed, x, y,
/// z)` records the cubiomes `(x4, y4, z4)` output.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct VoronoiAccessRecord {
    pub world_seed: u64,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub x4: i32,
    pub y4: i32,
    pub z4: i32,
    pub pad: u64,
}

const VORONOI_ACCESS_RECORDS: u64 = 4096;

fn write_voronoi_access_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 36, VORONOI_ACCESS_RECORDS)?;

    let mut rng_state: u64 = 0x0_acce5_5dead;
    for _ in 0..VORONOI_ACCESS_RECORDS {
        rng_state = lcg_step(rng_state);
        let world_seed = rng_state;
        rng_state = lcg_step(rng_state);
        let x = (rng_state as i32) % 1024;
        rng_state = lcg_step(rng_state);
        let y = (rng_state as i32) % 256;
        rng_state = lcg_step(rng_state);
        let z = (rng_state as i32) % 1024;
        let mut x4: c_int = 0;
        let mut y4: c_int = 0;
        let mut z4: c_int = 0;
        unsafe {
            ffi::cubiomes_call_voronoi_access_3d(
                world_seed,
                x,
                y,
                z,
                std::ptr::from_mut(&mut x4),
                std::ptr::from_mut(&mut y4),
                std::ptr::from_mut(&mut z4),
            );
        }
        let rec = VoronoiAccessRecord {
            world_seed,
            x,
            y,
            z,
            x4,
            y4,
            z4,
            pad: 0u64,
        };
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

/// `getVoronoiSHA` record (kind = 34). Pairs a 64-bit world seed with
/// the truncated SHA-256 digest cubiomes returns from `getVoronoiSHA`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct VoronoiShaRecord {
    pub seed: u64,
    pub digest: u64,
}

const VORONOI_SHA_RECORDS: u64 = 4096;

fn write_voronoi_sha_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 34, VORONOI_SHA_RECORDS)?;

    // Mix a small set of edge-case seeds (0, 1, u64::MAX, ...) ahead of
    // the deterministic LCG stream so the fixture exercises both the
    // boundary inputs and a broad random distribution.
    let edge_cases: [u64; 8] = [
        0,
        1,
        2,
        0xffff_ffff,
        0xffff_ffff_ffff_ffff,
        0x8000_0000_0000_0000,
        0x0000_0000_0000_0001,
        0xdead_beef_cafe_babe,
    ];
    for seed in edge_cases {
        let digest = unsafe { ffi::getVoronoiSHA(seed) };
        let rec = VoronoiShaRecord { seed, digest };
        file.write_all(bytemuck::bytes_of(&rec))?;
    }

    let mut rng_state: u64 = 0x5ec0_0a11_4519;
    for _ in 0..(VORONOI_SHA_RECORDS - edge_cases.len() as u64) {
        rng_state = lcg_step(rng_state);
        let seed = rng_state;
        let digest = unsafe { ffi::getVoronoiSHA(seed) };
        let rec = VoronoiShaRecord { seed, digest };
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

/// `mapOceanMix` record (kind = 33). The biome parent is a
/// `mapContinent` chain; the ocean parent is `mapOceanTemp` driven by
/// `PerlinNoise` initialized from `world_seed`. MC = 1.18.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct OceanMixRecord {
    pub world_seed: u64,
    pub biome_salt: u64,
    pub x: i32,
    pub z: i32,
    pub w: u32,
    pub h: u32,
    pub digest: u32,
    pub pad: u32,
}

const OCEAN_MIX_RECORDS: u64 = 2048;

fn write_ocean_mix_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 33, OCEAN_MIX_RECORDS)?;

    let mut rng_state: u64 = 0x0cea_4011_1334;
    for _ in 0..OCEAN_MIX_RECORDS {
        rng_state = lcg_step(rng_state);
        let world_seed = rng_state;
        rng_state = lcg_step(rng_state);
        let biome_salt = rng_state | 1;
        rng_state = lcg_step(rng_state);
        let w = ((rng_state & 0xf) as u32) + 4;
        let h = ((rng_state >> 8) & 0xf) as u32 + 4;
        rng_state = lcg_step(rng_state);
        let x = (rng_state as i32) % 64;
        rng_state = lcg_step(rng_state);
        let z = (rng_state as i32) % 64;
        let rec = ocean_mix_record(world_seed, biome_salt, x, z, w, h);
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

fn ocean_mix_record(
    world_seed: u64,
    biome_salt: u64,
    x: i32,
    z: i32,
    w: u32,
    h: u32,
) -> OceanMixRecord {
    // cubiomes reuses `out` as scratch: ocean chain occupies w*h cells,
    // biome chain (at x-8, z-8, size (w+17)*(h+17)) is written after.
    let cells = (w * h) as usize + ((w + 17) * (h + 17)) as usize;
    let mut out: Vec<i32> = vec![0; cells];
    unsafe {
        ffi::cubiomes_call_map_ocean_mix(
            world_seed,
            biome_salt,
            out.as_mut_ptr(),
            x,
            z,
            w as c_int,
            h as c_int,
        );
    }
    let digest = digest_i32_slice(&out[..(w * h) as usize]);
    OceanMixRecord {
        world_seed,
        biome_salt,
        x,
        z,
        w,
        h,
        digest,
        pad: 0,
    }
}

/// `mapHills` record (kind = 29). Two parent chains (each
/// `mapContinent`) feed `mapHills`. MC = 1.18.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct HillsRecord {
    pub world_seed: u64,
    pub biome_parent_salt: u64,
    pub river_parent_salt: u64,
    pub hills_salt: u64,
    pub x: i32,
    pub z: i32,
    pub w: u32,
    pub h: u32,
    pub digest: u32,
    pub pad: u32,
}

const HILLS_RECORDS: u64 = 4096;

fn write_hills_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 29, HILLS_RECORDS)?;

    let mut rng_state: u64 = 0x4111_11ee_5111;
    for _ in 0..HILLS_RECORDS {
        rng_state = lcg_step(rng_state);
        let world_seed = rng_state;
        rng_state = lcg_step(rng_state);
        let biome_parent_salt = rng_state | 1;
        rng_state = lcg_step(rng_state);
        let river_parent_salt = rng_state | 1;
        rng_state = lcg_step(rng_state);
        let hills_salt = rng_state | 1;
        rng_state = lcg_step(rng_state);
        let w = ((rng_state & 0x1f) as u32) + 2;
        let h = ((rng_state >> 8) & 0x1f) as u32 + 2;
        rng_state = lcg_step(rng_state);
        let x = (rng_state as i32) % 64;
        rng_state = lcg_step(rng_state);
        let z = (rng_state as i32) % 64;
        let rec = hills_record(
            world_seed,
            biome_parent_salt,
            river_parent_salt,
            hills_salt,
            x,
            z,
            w,
            h,
        );
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

fn hills_record(
    world_seed: u64,
    biome_parent_salt: u64,
    river_parent_salt: u64,
    hills_salt: u64,
    x: i32,
    z: i32,
    w: u32,
    h: u32,
) -> HillsRecord {
    // mapHills uses out as scratch for both parents (each pW * pH cells).
    let p_cells = ((w + 2) * (h + 2)) as usize;
    let mut out: Vec<i32> = vec![0; p_cells * 3];
    unsafe {
        ffi::cubiomes_call_map_hills(
            world_seed,
            MC_1_18_C,
            biome_parent_salt,
            river_parent_salt,
            hills_salt,
            out.as_mut_ptr(),
            x,
            z,
            w as c_int,
            h as c_int,
        );
    }
    let digest = digest_i32_slice(&out[..(w * h) as usize]);
    HillsRecord {
        world_seed,
        biome_parent_salt,
        river_parent_salt,
        hills_salt,
        x,
        z,
        w,
        h,
        digest,
        pad: 0,
    }
}

/// `mapShore` record (kind = 28). Two-layer chain at `MC_1_18`: simple
/// `mapContinent` parent followed by `mapShore`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ShoreRecord {
    pub world_seed: u64,
    pub parent_salt: u64,
    pub shore_salt: u64,
    pub x: i32,
    pub z: i32,
    pub w: u32,
    pub h: u32,
    pub digest: u32,
    pub pad: u32,
}

const SHORE_RECORDS: u64 = 4096;
/// `MC_1_18` ordinal (matches `MCVersion::V1_18.ord()`).
const MC_1_18_C: c_int = 22;

fn write_shore_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 28, SHORE_RECORDS)?;

    let mut rng_state: u64 = 0x5403_0e10_1010;
    for _ in 0..SHORE_RECORDS {
        rng_state = lcg_step(rng_state);
        let world_seed = rng_state;
        rng_state = lcg_step(rng_state);
        let parent_salt = rng_state | 1;
        rng_state = lcg_step(rng_state);
        let shore_salt = rng_state | 1;
        rng_state = lcg_step(rng_state);
        let w = ((rng_state & 0x1f) as u32) + 2;
        let h = ((rng_state >> 8) & 0x1f) as u32 + 2;
        rng_state = lcg_step(rng_state);
        let x = (rng_state as i32) % 64;
        rng_state = lcg_step(rng_state);
        let z = (rng_state as i32) % 64;
        let rec = shore_record(world_seed, parent_salt, shore_salt, x, z, w, h);
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

fn shore_record(
    world_seed: u64,
    parent_salt: u64,
    shore_salt: u64,
    x: i32,
    z: i32,
    w: u32,
    h: u32,
) -> ShoreRecord {
    let p_cells = ((w + 2) * (h + 2)) as usize;
    let mut out: Vec<i32> = vec![0; p_cells];
    unsafe {
        ffi::cubiomes_call_map_shore(
            world_seed,
            MC_1_18_C,
            parent_salt,
            shore_salt,
            out.as_mut_ptr(),
            x,
            z,
            w as c_int,
            h as c_int,
        );
    }
    let digest = digest_i32_slice(&out[..(w * h) as usize]);
    ShoreRecord {
        world_seed,
        parent_salt,
        shore_salt,
        x,
        z,
        w,
        h,
        digest,
        pad: 0,
    }
}

/// `mapVoronoi114` record (kind = 27). Two-layer chain: simple
/// `mapContinent` parent at 1:4 scale, then `mapVoronoi114` at 1:1.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Voronoi114Record {
    pub world_seed: u64,
    pub parent_salt: u64,
    pub voronoi_salt: u64,
    pub x: i32,
    pub z: i32,
    pub w: u32,
    pub h: u32,
    pub digest: u32,
    pub pad: u32,
}

const VORONOI114_RECORDS: u64 = 2048;

fn write_voronoi114_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 27, VORONOI114_RECORDS)?;

    let mut rng_state: u64 = 0x0fea_4011_1100;
    for _ in 0..VORONOI114_RECORDS {
        rng_state = lcg_step(rng_state);
        let world_seed = rng_state;
        rng_state = lcg_step(rng_state);
        let parent_salt = rng_state | 1;
        rng_state = lcg_step(rng_state);
        let voronoi_salt = rng_state | 1;
        rng_state = lcg_step(rng_state);
        // Voronoi at 1:1 needs 1:4 parent + a few sub-cell slack.
        let w = ((rng_state & 0xf) as u32) + 4;
        let h = ((rng_state >> 8) & 0xf) as u32 + 4;
        rng_state = lcg_step(rng_state);
        let x = (rng_state as i32) % 64;
        rng_state = lcg_step(rng_state);
        let z = (rng_state as i32) % 64;
        let rec = voronoi114_record(world_seed, parent_salt, voronoi_salt, x, z, w, h);
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

fn voronoi114_record(
    world_seed: u64,
    parent_salt: u64,
    voronoi_salt: u64,
    x: i32,
    z: i32,
    w: u32,
    h: u32,
) -> Voronoi114Record {
    // cubiomes' mapVoronoi114 reuses `out` as scratch for the 1:4
    // parent followed by the 1:1 buffer; allocate generously.
    let cells = ((w + 16) * (h + 16)) as usize;
    let mut out: Vec<i32> = vec![0; cells];
    unsafe {
        ffi::cubiomes_call_map_voronoi114(
            world_seed,
            parent_salt,
            voronoi_salt,
            out.as_mut_ptr(),
            x,
            z,
            w as c_int,
            h as c_int,
        );
    }
    let digest = digest_i32_slice(&out[..(w * h) as usize]);
    Voronoi114Record {
        world_seed,
        parent_salt,
        voronoi_salt,
        x,
        z,
        w,
        h,
        digest,
        pad: 0,
    }
}

/// `mapOceanTemp` record (kind = 26). Input is just a world seed +
/// sample rectangle; the layer derives a `PerlinNoise` from the seed
/// internally.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct OceanTempRecord {
    pub world_seed: u64,
    pub x: i32,
    pub z: i32,
    pub w: u32,
    pub h: u32,
    pub digest: u32,
    pub pad: u32,
}

const OCEAN_TEMP_RECORDS: u64 = 4096;

fn write_ocean_temp_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 26, OCEAN_TEMP_RECORDS)?;

    let mut rng_state: u64 = 0x0cea_4e10_1334;
    for _ in 0..OCEAN_TEMP_RECORDS {
        rng_state = lcg_step(rng_state);
        let world_seed = rng_state;
        rng_state = lcg_step(rng_state);
        let w = ((rng_state & 0x1f) as u32) + 2;
        let h = ((rng_state >> 8) & 0x1f) as u32 + 2;
        rng_state = lcg_step(rng_state);
        let x = (rng_state as i32) % 64;
        rng_state = lcg_step(rng_state);
        let z = (rng_state as i32) % 64;
        let rec = ocean_temp_record(world_seed, x, z, w, h);
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

fn ocean_temp_record(world_seed: u64, x: i32, z: i32, w: u32, h: u32) -> OceanTempRecord {
    let cells = (w * h) as usize;
    let mut out: Vec<i32> = vec![0; cells];
    unsafe {
        ffi::cubiomes_call_map_ocean_temp(
            world_seed,
            out.as_mut_ptr(),
            x,
            z,
            w as c_int,
            h as c_int,
        );
    }
    let digest = digest_i32_slice(&out);
    OceanTempRecord {
        world_seed,
        x,
        z,
        w,
        h,
        digest,
        pad: 0,
    }
}

/// Octave noise record (kind = 5). Uses fixed omin = -3, len = 4 for both
/// the Java and Xoroshiro initialisers (amplitudes = [1, 1, 1, 1]).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct OctaveRecord {
    pub seed: u64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub java_sample_bits: u64,
    pub xoroshiro_sample_bits: u64,
}

/// Double-Perlin record (kind = 6). Same setup as `OctaveRecord`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct DoublePerlinRecord {
    pub seed: u64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub java_sample_bits: u64,
    pub xoroshiro_sample_bits: u64,
}

const OCT_OMIN: i32 = -3;
const OCT_LEN: i32 = 4;
const OCT_AMPS: [f64; 4] = [1.0, 1.0, 1.0, 1.0];

fn write_octave_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 5, RECORD_COUNT)?;

    let mut rng_state: u64 = 0x0bad_f00d_d00d;
    for i in 0..RECORD_COUNT {
        rng_state = lcg_step(rng_state);
        let seed = rng_state ^ i;
        rng_state = lcg_step(rng_state);
        let x = u64_to_double_signed(rng_state) * 100.0;
        rng_state = lcg_step(rng_state);
        let y = u64_to_double_signed(rng_state) * 16.0;
        rng_state = lcg_step(rng_state);
        let z = u64_to_double_signed(rng_state) * 100.0;
        let rec = octave_record(seed, x, y, z);
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

fn octave_record(seed: u64, x: f64, y: f64, z: f64) -> OctaveRecord {
    let java_sample_bits = unsafe {
        let mut octaves = std::mem::zeroed::<[ffi::CPerlinNoise; OCT_LEN as usize]>();
        let mut oct = std::mem::zeroed::<ffi::COctaveNoise>();
        let mut s = ffi::cubiomes_set_seed(seed);
        ffi::octaveInit(
            &raw mut oct,
            &raw mut s,
            octaves.as_mut_ptr(),
            OCT_OMIN,
            OCT_LEN,
        );
        ffi::sampleOctave(&raw const oct, x, y, z).to_bits()
    };
    let xoroshiro_sample_bits = unsafe {
        let mut octaves = std::mem::zeroed::<[ffi::CPerlinNoise; OCT_LEN as usize]>();
        let mut oct = std::mem::zeroed::<ffi::COctaveNoise>();
        let mut xr = ffi::Xoroshiro { lo: 0, hi: 0 };
        ffi::cubiomes_x_set_seed(&raw mut xr, seed);
        ffi::xOctaveInit(
            &raw mut oct,
            &raw mut xr,
            octaves.as_mut_ptr(),
            OCT_AMPS.as_ptr(),
            OCT_OMIN,
            OCT_LEN,
            -1,
        );
        ffi::sampleOctave(&raw const oct, x, y, z).to_bits()
    };
    OctaveRecord {
        seed,
        x,
        y,
        z,
        java_sample_bits,
        xoroshiro_sample_bits,
    }
}

fn write_double_perlin_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 6, RECORD_COUNT)?;

    let mut rng_state: u64 = 0x1357_9bdf_2468;
    for i in 0..RECORD_COUNT {
        rng_state = lcg_step(rng_state);
        let seed = rng_state ^ i;
        rng_state = lcg_step(rng_state);
        let x = u64_to_double_signed(rng_state) * 100.0;
        rng_state = lcg_step(rng_state);
        let y = u64_to_double_signed(rng_state) * 16.0;
        rng_state = lcg_step(rng_state);
        let z = u64_to_double_signed(rng_state) * 100.0;
        let rec = double_perlin_record(seed, x, y, z);
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

fn double_perlin_record(seed: u64, x: f64, y: f64, z: f64) -> DoublePerlinRecord {
    let java_sample_bits = unsafe {
        let mut octaves_a = std::mem::zeroed::<[ffi::CPerlinNoise; OCT_LEN as usize]>();
        let mut octaves_b = std::mem::zeroed::<[ffi::CPerlinNoise; OCT_LEN as usize]>();
        let mut dp = std::mem::zeroed::<ffi::CDoublePerlinNoise>();
        let mut s = ffi::cubiomes_set_seed(seed);
        ffi::doublePerlinInit(
            &raw mut dp,
            &raw mut s,
            octaves_a.as_mut_ptr(),
            octaves_b.as_mut_ptr(),
            OCT_OMIN,
            OCT_LEN,
        );
        ffi::sampleDoublePerlin(&raw const dp, x, y, z).to_bits()
    };
    let xoroshiro_sample_bits = unsafe {
        // xDoublePerlinInit takes a single octaves buffer of size 2 * len.
        let mut octaves = std::mem::zeroed::<[ffi::CPerlinNoise; (2 * OCT_LEN) as usize]>();
        let mut dp = std::mem::zeroed::<ffi::CDoublePerlinNoise>();
        let mut xr = ffi::Xoroshiro { lo: 0, hi: 0 };
        ffi::cubiomes_x_set_seed(&raw mut xr, seed);
        ffi::xDoublePerlinInit(
            &raw mut dp,
            &raw mut xr,
            octaves.as_mut_ptr(),
            OCT_AMPS.as_ptr(),
            OCT_OMIN,
            OCT_LEN,
            -1,
        );
        ffi::sampleDoublePerlin(&raw const dp, x, y, z).to_bits()
    };
    DoublePerlinRecord {
        seed,
        x,
        y,
        z,
        java_sample_bits,
        xoroshiro_sample_bits,
    }
}

fn mc_seed_record(seed: u64, salt: u64) -> McSeedRecord {
    let mut rec = McSeedRecord::zeroed();
    rec.seed = seed;
    rec.salt = salt;
    rec.step_seed = unsafe { ffi::cubiomes_mc_step_seed(seed, salt) };
    rec.first_int_24 = unsafe { ffi::cubiomes_mc_first_int(seed, 24) };
    rec.layer_salt = unsafe { ffi::cubiomes_get_layer_salt(salt) };
    rec.start_salt = unsafe { ffi::cubiomes_get_start_salt(seed, salt) };
    rec.start_seed = unsafe { ffi::cubiomes_get_start_seed(seed, salt) };
    let x = (salt as i32).wrapping_neg();
    let z = salt as i32;
    rec.chunk_seed = unsafe { ffi::cubiomes_get_chunk_seed(seed, x, z) };
    // Provide a coprime pair so mulInv has a real result; if the seed
    // happens to share a factor with `salt | 7` we accept whatever
    // cubiomes returns and exercise the cooperating Rust port the same way.
    let mi_x = seed | 1;
    let mi_m = (salt | 7).max(2);
    rec.mul_inv = unsafe { ffi::cubiomes_mul_inv(mi_x, mi_m) };
    rec
}

mod ffi {
    use std::ffi::{c_char, c_int};

    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct Xoroshiro {
        pub lo: u64,
        pub hi: u64,
    }

    /// C-layout `PerlinNoise` from `cubiomes/noise.h`.
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct CPerlinNoise {
        pub d: [u8; 257],
        pub h2: u8,
        pub a: f64,
        pub b: f64,
        pub c: f64,
        pub amplitude: f64,
        pub lacunarity: f64,
        pub d2: f64,
        pub t2: f64,
    }

    /// C-layout `OctaveNoise`.
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct COctaveNoise {
        pub octcnt: c_int,
        pub octaves: *mut CPerlinNoise,
    }

    /// C-layout `DoublePerlinNoise`.
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct CDoublePerlinNoise {
        pub amplitude: f64,
        pub oct_a: COctaveNoise,
        pub oct_b: COctaveNoise,
    }

    unsafe extern "C" {
        pub fn mc2str(mc: c_int) -> *const c_char;

        pub fn cubiomes_set_seed(value: u64) -> u64;
        pub fn cubiomes_next(seed: *mut u64, bits: c_int) -> c_int;
        pub fn cubiomes_next_long(seed: *mut u64) -> u64;
        pub fn cubiomes_next_float(seed: *mut u64) -> f32;
        pub fn cubiomes_next_double(seed: *mut u64) -> f64;
        pub fn cubiomes_skip_next_n(seed: *mut u64, n: u64);
        pub fn cubiomes_next_int_24(seed: *mut u64) -> c_int;

        pub fn cubiomes_x_set_seed(xr: *mut Xoroshiro, value: u64);
        pub fn cubiomes_x_next_long(xr: *mut Xoroshiro) -> u64;
        pub fn cubiomes_x_next_int(xr: *mut Xoroshiro, n: u32) -> c_int;
        pub fn cubiomes_x_next_double(xr: *mut Xoroshiro) -> f64;
        pub fn cubiomes_x_next_float(xr: *mut Xoroshiro) -> f32;
        pub fn cubiomes_x_skip_n(xr: *mut Xoroshiro, count: c_int);
        pub fn cubiomes_x_next_long_j(xr: *mut Xoroshiro) -> u64;
        pub fn cubiomes_x_next_int_j(xr: *mut Xoroshiro, n: u32) -> c_int;

        pub fn cubiomes_mc_step_seed(s: u64, salt: u64) -> u64;
        pub fn cubiomes_mc_first_int(s: u64, m: c_int) -> c_int;
        pub fn cubiomes_get_chunk_seed(ss: u64, x: c_int, z: c_int) -> u64;
        pub fn cubiomes_get_layer_salt(salt: u64) -> u64;
        pub fn cubiomes_get_start_salt(ws: u64, ls: u64) -> u64;
        pub fn cubiomes_get_start_seed(ws: u64, ls: u64) -> u64;
        pub fn cubiomes_mul_inv(x: u64, m: u64) -> u64;

        // Noise (defined directly in cubiomes/noise.c, no wrapper needed).
        #[allow(non_snake_case)]
        pub fn perlinInit(noise: *mut CPerlinNoise, seed: *mut u64);
        #[allow(non_snake_case)]
        pub fn xPerlinInit(noise: *mut CPerlinNoise, xr: *mut Xoroshiro);
        #[allow(non_snake_case)]
        pub fn samplePerlin(
            noise: *const CPerlinNoise,
            x: f64,
            y: f64,
            z: f64,
            yamp: f64,
            ymin: f64,
        ) -> f64;
        #[allow(non_snake_case)]
        pub fn sampleSimplex2D(noise: *const CPerlinNoise, x: f64, y: f64) -> f64;

        #[allow(non_snake_case)]
        pub fn octaveInit(
            noise: *mut COctaveNoise,
            seed: *mut u64,
            octaves: *mut CPerlinNoise,
            omin: c_int,
            len: c_int,
        );
        #[allow(non_snake_case)]
        pub fn xOctaveInit(
            noise: *mut COctaveNoise,
            xr: *mut Xoroshiro,
            octaves: *mut CPerlinNoise,
            amplitudes: *const f64,
            omin: c_int,
            len: c_int,
            nmax: c_int,
        ) -> c_int;
        #[allow(non_snake_case)]
        pub fn sampleOctave(noise: *const COctaveNoise, x: f64, y: f64, z: f64) -> f64;

        #[allow(non_snake_case)]
        pub fn doublePerlinInit(
            noise: *mut CDoublePerlinNoise,
            seed: *mut u64,
            octaves_a: *mut CPerlinNoise,
            octaves_b: *mut CPerlinNoise,
            omin: c_int,
            len: c_int,
        );
        #[allow(non_snake_case)]
        pub fn xDoublePerlinInit(
            noise: *mut CDoublePerlinNoise,
            xr: *mut Xoroshiro,
            octaves: *mut CPerlinNoise,
            amplitudes: *const f64,
            omin: c_int,
            len: c_int,
            nmax: c_int,
        ) -> c_int;
        #[allow(non_snake_case)]
        pub fn sampleDoublePerlin(noise: *const CDoublePerlinNoise, x: f64, y: f64, z: f64) -> f64;

        // Layer-map wrappers (see cubiomes_layers_ffi.c).
        pub fn cubiomes_call_map_continent(
            start_seed: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_map_zoom_fuzzy(
            world_seed: u64,
            parent_layer_salt: u64,
            zoom_layer_salt: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_map_zoom(
            world_seed: u64,
            parent_layer_salt: u64,
            zoom_layer_salt: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_map_land(
            world_seed: u64,
            parent_layer_salt: u64,
            land_layer_salt: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_map_land16(
            world_seed: u64,
            parent_layer_salt: u64,
            land_layer_salt: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_map_land_b18(
            world_seed: u64,
            parent_layer_salt: u64,
            land_layer_salt: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_map_island(
            world_seed: u64,
            parent_salt: u64,
            child_salt: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_map_snow16(
            world_seed: u64,
            parent_salt: u64,
            child_salt: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_map_snow(
            world_seed: u64,
            parent_salt: u64,
            child_salt: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_map_special(
            world_seed: u64,
            parent_salt: u64,
            child_salt: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_map_mushroom(
            world_seed: u64,
            parent_salt: u64,
            child_salt: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_map_deep_ocean(
            world_seed: u64,
            parent_salt: u64,
            child_salt: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_map_cool(
            world_seed: u64,
            continent_salt: u64,
            snow_salt: u64,
            cool_salt: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_map_heat(
            world_seed: u64,
            continent_salt: u64,
            snow_salt: u64,
            heat_salt: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_map_ocean_temp(
            world_seed: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_map_ocean_mix(
            world_seed: u64,
            biome_salt: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_map_voronoi(
            world_seed: u64,
            biome_salt: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_voronoi_access_3d(
            world_seed: u64,
            x: c_int,
            y: c_int,
            z: c_int,
            x4: *mut c_int,
            y4: *mut c_int,
            z4: *mut c_int,
        );
        pub fn getVoronoiSHA(seed: u64) -> u64;
        pub fn cubiomes_call_dump_layer_stack(
            mc: c_int,
            large_biomes: c_int,
            world_seed: u64,
            out: *mut u64,
        );
        pub fn cubiomes_call_gen_area_at(
            mc: c_int,
            large_biomes: c_int,
            world_seed: u64,
            layer_id_ord: c_int,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        ) -> c_int;
        pub fn cubiomes_call_gen_area_at_entry1(
            mc: c_int,
            large_biomes: c_int,
            world_seed: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        ) -> c_int;
        pub fn cubiomes_call_map_river(
            world_seed: u64,
            mc: c_int,
            parent_salt: u64,
            river_salt: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_map_smooth(
            world_seed: u64,
            mc: c_int,
            parent_salt: u64,
            smooth_salt: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_map_river_mix(
            world_seed: u64,
            mc: c_int,
            biome_salt: u64,
            river_salt: u64,
            mix_salt: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_map_hills(
            world_seed: u64,
            mc: c_int,
            biome_parent_salt: u64,
            river_parent_salt: u64,
            hills_salt: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_map_shore(
            world_seed: u64,
            mc: c_int,
            parent_salt: u64,
            shore_salt: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_map_voronoi114(
            world_seed: u64,
            parent_salt: u64,
            voronoi_salt: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_map_biome(
            world_seed: u64,
            mc: c_int,
            continent_salt: u64,
            snow_salt: u64,
            biome_salt: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_map_noise(
            world_seed: u64,
            mc: c_int,
            continent_salt: u64,
            snow_salt: u64,
            biome_salt: u64,
            child_salt: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_map_bamboo(
            world_seed: u64,
            mc: c_int,
            continent_salt: u64,
            snow_salt: u64,
            biome_salt: u64,
            child_salt: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_map_swamp_river(
            world_seed: u64,
            mc: c_int,
            continent_salt: u64,
            snow_salt: u64,
            biome_salt: u64,
            child_salt: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_map_sunflower(
            world_seed: u64,
            mc: c_int,
            continent_salt: u64,
            snow_salt: u64,
            biome_salt: u64,
            child_salt: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
    }
}
