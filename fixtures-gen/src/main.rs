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
    if let Err(err) = write_surface_noise_fixture(&fixtures_dir.join("surface_noise.bin")) {
        eprintln!("surface_noise fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_nether_fixture(&fixtures_dir.join("nether.bin")) {
        eprintln!("nether fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_end_fixture(&fixtures_dir.join("end.bin")) {
        eprintln!("end fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_climate_fixture(&fixtures_dir.join("climate.bin")) {
        eprintln!("climate fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_biome_noise_fixture(&fixtures_dir.join("biome_noise.bin")) {
        eprintln!("biome_noise fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_biome_noise_beta_fixture(&fixtures_dir.join("biome_noise_beta.bin")) {
        eprintln!("biome_noise_beta fixture failed: {err}");
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
    if let Err(err) = write_generator_biome_fixture(&fixtures_dir.join("generator_biome.bin")) {
        eprintln!("generator_biome fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_gen_biomes_fixture(&fixtures_dir.join("gen_biomes_range.bin")) {
        eprintln!("gen_biomes_range fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_structure_pos_fixture(&fixtures_dir.join("structure_pos.bin")) {
        eprintln!("structure_pos fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_slime_fixture(&fixtures_dir.join("slime_chunks.bin")) {
        eprintln!("slime fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_quadbase_fixture(&fixtures_dir.join("quadbase.bin")) {
        eprintln!("quadbase fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_stronghold_fixture(&fixtures_dir.join("stronghold_init.bin")) {
        eprintln!("stronghold fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_mineshaft_fixture(&fixtures_dir.join("mineshaft.bin")) {
        eprintln!("mineshaft fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_biome_predicates_fixture(&fixtures_dir.join("biome_predicates.bin")) {
        eprintln!("biome_predicates fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_stronghold_full_fixture(&fixtures_dir.join("stronghold_full.bin")) {
        eprintln!("stronghold_full fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_spawn_fixture(&fixtures_dir.join("estimate_spawn.bin")) {
        eprintln!("estimate_spawn fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_population_seed_fixture(&fixtures_dir.join("population_seed.bin")) {
        eprintln!("population_seed fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_end_islands_fixture(&fixtures_dir.join("end_islands.bin")) {
        eprintln!("end_islands fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_end_island_height_fixture(&fixtures_dir.join("end_island_height.bin")) {
        eprintln!("end_island_height fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_end_height_noise_fixture(&fixtures_dir.join("end_height_noise.bin")) {
        eprintln!("end_height_noise fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_end_surface_height_fixture(&fixtures_dir.join("end_surface_height.bin"))
    {
        eprintln!("end_surface_height fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_end_chunk_empty_fixture(&fixtures_dir.join("end_chunk_empty.bin")) {
        eprintln!("end_chunk_empty fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_biome_depth_scale_fixture(&fixtures_dir.join("biome_depth_scale.bin")) {
        eprintln!("biome_depth_scale fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_optimal_afk_fixture(&fixtures_dir.join("optimal_afk.bin")) {
        eprintln!("optimal_afk fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_map_approx_height_fixture(&fixtures_dir.join("map_approx_height.bin")) {
        eprintln!("map_approx_height fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_get_spawn_fixture(&fixtures_dir.join("get_spawn.bin")) {
        eprintln!("get_spawn fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) =
        write_viable_feature_biome_fixture(&fixtures_dir.join("viable_feature_biome.bin"))
    {
        eprintln!("viable_feature_biome fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) =
        write_viable_structure_pos_fixture(&fixtures_dir.join("viable_structure_pos.bin"))
    {
        eprintln!("viable_structure_pos fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_get_variant_fixture(&fixtures_dir.join("get_variant.bin")) {
        eprintln!("get_variant fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_fixed_end_gateways_fixture(&fixtures_dir.join("fixed_end_gateways.bin"))
    {
        eprintln!("fixed_end_gateways fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_linked_gateway_pos_fixture(&fixtures_dir.join("linked_gateway_pos.bin"))
    {
        eprintln!("linked_gateway_pos fixture failed: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = write_scan_for_quads_fixture(&fixtures_dir.join("scan_for_quads.bin")) {
        eprintln!("scan_for_quads fixture failed: {err}");
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

/// `Generator::gen_biomes` Range parity record (kind = 47). Captures
/// cubiomes' end-to-end `setupGenerator + applySeed + genBiomes`
/// flow over a small `(sx * sy * sz)` cuboid. Output stored as an
/// XOR-folded digest of the biome id grid.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GenBiomesRangeRecord {
    pub mc: u32,
    pub flags: u32,
    pub dim: i32,
    pub scale: i32,
    pub seed: u64,
    pub x: i32,
    pub z: i32,
    pub sx: u32,
    pub sz: u32,
    pub y: i32,
    pub sy: u32,
    pub digest: u32,
    pub pad: u32,
}

const GEN_BIOMES_RANGE_RECORDS: u64 = 256;

#[allow(clippy::many_single_char_names, clippy::too_many_lines)]
fn write_gen_biomes_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 47, GEN_BIOMES_RANGE_RECORDS)?;

    let mut rng_state: u64 = 0x0000_a14e_4d04_4044;
    for _ in 0..GEN_BIOMES_RANGE_RECORDS {
        rng_state = lcg_step(rng_state);
        let seed = rng_state;
        rng_state = lcg_step(rng_state);
        let mc_pool: [i32; 8] = [1, 3, 10, 15, 19, 22, 25, 28];
        let mc = mc_pool[(rng_state as usize) % mc_pool.len()];
        rng_state = lcg_step(rng_state);
        let dim_choice = rng_state % 3;
        let dim: i32 = match dim_choice {
            1 if mc >= 19 => -1,
            2 if mc >= 12 => 1,
            _ => 0,
        };
        rng_state = lcg_step(rng_state);
        // Restrict scales to the supported set:
        let scale = if dim == 0 && (10..=21).contains(&mc) {
            // Layered Overworld: 4, 16, 64, 256 (skip scale=1 which
            // would require the Voronoi 1:1 grid extension).
            let s: [i32; 4] = [4, 16, 64, 256];
            s[(rng_state as usize) % s.len()]
        } else if dim == 0 && mc >= 22 {
            // Modern: scale=4 only for now (>=4 supported).
            4
        } else if dim == 0 {
            // Beta — only return Overworld scale=4 records with
            // mc>=2 (B1_8+) since Beta gen_biomes isn't ported yet.
            // We skip Beta by picking another mc — simplest: clamp
            // to mc=15 (1.12, layered).
            4
        } else if dim == -1 {
            // Nether: 4, 16
            if rng_state.trailing_zeros() == 0 {
                16
            } else {
                4
            }
        } else {
            // End: 4 or 16
            if rng_state.trailing_zeros() == 0 {
                16
            } else {
                4
            }
        };

        // Beta has no gen_biomes implementation in our port yet —
        // skip those records by re-rolling mc to a layered version.
        let mc = if mc == 1 || mc == 3 {
            // 1.0 with dim=Overworld is layered, so it's fine —
            // only mc=1 (B1.7) needs replacement.
            if mc == 1 { 10 } else { mc }
        } else {
            mc
        };

        rng_state = lcg_step(rng_state);
        let flags: u32 = u32::from(mc >= 6 && rng_state.trailing_zeros() >= 2);
        rng_state = lcg_step(rng_state);
        let x = (rng_state as i32) % 256;
        rng_state = lcg_step(rng_state);
        let z = (rng_state as i32) % 256;
        rng_state = lcg_step(rng_state);
        // Keep grids small so the parity tests stay fast.
        let sx = ((rng_state & 0x7) as u32) + 1; // 1..=8
        let sz = ((rng_state >> 8) & 0x7) as u32 + 1;
        rng_state = lcg_step(rng_state);
        let y = (rng_state as i32) % 256;
        // sy: only Nether uses the sy axis meaningfully; for OW and
        // End it's a 2D layer expanded vertically. Keep sy small.
        let sy: u32 = if dim == -1 {
            ((rng_state >> 16) & 0x3) as u32 + 1 // 1..=4
        } else {
            1
        };

        let cells = (sx * sy * sz) as usize;
        // Allocate cubiomes' worst-case cache via allocCache-style
        // padding. For our parity test we only need to read the
        // first `cells` ints; oversize to be safe.
        let pad_cells = cells * 4 + 256;
        let mut out: Vec<i32> = vec![0; pad_cells];
        let err = unsafe {
            ffi::cubiomes_call_gen_biomes(
                mc,
                flags,
                dim,
                seed,
                scale,
                x,
                z,
                sx as c_int,
                sz as c_int,
                y,
                sy as c_int,
                out.as_mut_ptr(),
            )
        };
        let digest = if err == 0 {
            digest_i32_slice(&out[..cells])
        } else {
            // Record an error sentinel — the parity test will catch
            // the mismatch.
            0xffff_ffff
        };

        let rec = GenBiomesRangeRecord {
            mc: mc as u32,
            flags,
            dim,
            scale,
            seed,
            x,
            z,
            sx,
            sz,
            y,
            sy,
            digest,
            pad: 0,
        };
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

/// `Generator::biome_at` parity record (kind = 46). Captures
/// cubiomes' end-to-end `setupGenerator(mc, flags) +
/// applySeed(dim, seed) + getBiomeAt(scale, x, y, z)` flow.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GeneratorBiomeRecord {
    pub mc: u32,
    pub flags: u32,
    pub dim: i32,
    pub scale: i32,
    pub seed: u64,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub biome_id: i32,
}

const GENERATOR_BIOME_RECORDS: u64 = 1024;

/// `isSlimeChunk` parity record (kind = 49).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SlimeRecord {
    pub seed: u64,
    pub cx: i32,
    pub cz: i32,
    pub is_slime: i32,
    pub pad: i32,
}

const SLIME_RECORDS: u64 = 4096;

fn write_slime_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 49, SLIME_RECORDS)?;

    let mut rng_state: u64 = 0x0000_5117e_5117e;
    for _ in 0..SLIME_RECORDS {
        rng_state = lcg_step(rng_state);
        let seed = rng_state;
        rng_state = lcg_step(rng_state);
        let cx = (rng_state as i32) % 4096;
        rng_state = lcg_step(rng_state);
        let cz = (rng_state as i32) % 4096;
        let is_slime = unsafe { ffi::cubiomes_call_is_slime_chunk(seed, cx, cz) };
        let rec = SlimeRecord {
            seed,
            cx,
            cz,
            is_slime,
            pad: 0,
        };
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

/// Quad-base feature24 parity record (kind = 50). Records the
/// `isQuadBaseFeature24Classic` flag and the
/// `isQuadBaseFeature24(7+1, 7+1, 9+1)` enclosing-sphere radius
/// (`0.0` for non-quad-base seeds).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct QuadbaseRecord {
    pub seed: u64,
    pub structure_type: i32,
    pub mc: i32,
    pub classic_radius_bits: u32,
    pub feature24_radius_bits: u32,
    pub cst: i32,
    pub low20: u32,
}

const QUADBASE_RECORDS: u64 = 4096;

fn write_quadbase_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 50, QUADBASE_RECORDS)?;

    // Structures suitable for isQuadBaseFeature24: Swamp_Hut (3),
    // Desert_Pyramid (1), Jungle_Pyramid (2), Igloo (4), Village (5).
    let types: [i32; 5] = [3, 1, 2, 4, 5];
    let mc_pool: [i32; 3] = [15, 19, 22];

    let mut rng_state: u64 = 0x0000_9aba_5e90_0001;
    for _ in 0..QUADBASE_RECORDS {
        rng_state = lcg_step(rng_state);
        let seed = rng_state;
        rng_state = lcg_step(rng_state);
        let ty = types[(rng_state as usize) % types.len()];
        rng_state = lcg_step(rng_state);
        let mc = mc_pool[(rng_state as usize) % mc_pool.len()];

        let classic = unsafe { ffi::cubiomes_call_is_quad_base_feature_24_classic(ty, mc, seed) };
        let feat24 = unsafe { ffi::cubiomes_call_is_quad_base_feature_24(ty, mc, seed, 8, 8, 10) };

        // For non-witch-hut types `getQuadHutCst` isn't meaningful;
        // still record it so the fixture covers the function.
        let low20 = (seed & 0xfffff) as u32;
        let cst = unsafe { ffi::cubiomes_call_get_quad_hut_cst(low20 as u64) };

        let rec = QuadbaseRecord {
            seed,
            structure_type: ty,
            mc,
            classic_radius_bits: classic.to_bits(),
            feature24_radius_bits: feat24.to_bits(),
            cst,
            low20,
        };
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

/// Biome-checked stronghold iteration parity record (kind = 54).
/// Stores the (x, z) of the first 3 strongholds returned by
/// cubiomes' biome-aware `nextStronghold` for each (mc, seed) pair.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct StrongholdFullRecord {
    pub mc: i32,
    pub pad: i32,
    pub seed: u64,
    pub pos_xz: [i32; 6],
}

const STRONGHOLD_FULL_RECORDS: u64 = 128;

fn write_stronghold_full_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 54, STRONGHOLD_FULL_RECORDS)?;
    // Layered (10=1.7, 15=1.12) + Modern (22=1.18) + 1.19.4+
    // (28=1.21 WD) — covers the three biome-aware code paths.
    let mc_pool: [i32; 4] = [10, 15, 22, 28];
    let mut rng_state: u64 = 0x0000_5478_55a4_b1ed;
    for _ in 0..STRONGHOLD_FULL_RECORDS {
        rng_state = lcg_step(rng_state);
        let mc = mc_pool[(rng_state as usize) % mc_pool.len()];
        rng_state = lcg_step(rng_state);
        let seed = rng_state;
        let mut buf = [0_i32; 6];
        unsafe {
            ffi::cubiomes_call_nth_strongholds(mc, seed, 3, buf.as_mut_ptr());
        }
        let rec = StrongholdFullRecord {
            mc,
            pad: 0,
            seed,
            pos_xz: buf,
        };
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

/// `estimateSpawn` parity record (kind = 55). `(mc, seed) →
/// (spawn_x, spawn_z)`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct EstimateSpawnRecord {
    pub mc: i32,
    pub pad: i32,
    pub seed: u64,
    pub spawn_x: i32,
    pub spawn_z: i32,
}

fn write_spawn_fixture(path: &Path) -> std::io::Result<()> {
    let mc_pool: [i32; 5] = [3, 10, 15, 22, 28];
    // Modern findFittestPos is O(spiral) — keep the per-MC count
    // small so fixture generation stays under a few seconds.
    let modern_count: u64 = 32;
    let layered_count: u64 = 96;
    let beta_count: u64 = 32;
    // mc_pool = [3, 10, 15, 22, 28]: 1 beta (V1_0), 2 layered
    // (V1_7, V1_12), 2 modern (V1_18, V1_21 WD).
    let total = beta_count + layered_count * 2 + modern_count * 2;
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 55, total)?;
    let mut rng_state: u64 = 0x0000_55a1_7e51_5e91;
    for &mc in &mc_pool {
        let count = if mc <= 3 {
            beta_count
        } else if mc <= 21 {
            layered_count
        } else {
            modern_count
        };
        for _ in 0..count {
            rng_state = lcg_step(rng_state);
            let seed = rng_state;
            let mut px: c_int = 0;
            let mut pz: c_int = 0;
            unsafe {
                ffi::cubiomes_call_estimate_spawn(
                    mc,
                    seed,
                    std::ptr::from_mut(&mut px),
                    std::ptr::from_mut(&mut pz),
                );
            }
            let rec = EstimateSpawnRecord {
                mc,
                pad: 0,
                seed,
                spawn_x: px,
                spawn_z: pz,
            };
            file.write_all(bytemuck::bytes_of(&rec))?;
        }
    }
    file.flush()
}

/// `getPopulationSeed` parity record (kind = 56). Captures the 3-way
/// MC dispatch: pre-1.13 (`/2*2+1`), 1.13–1.17 (`|1`, Java RNG),
/// 1.18+ (`|1`, Xoroshiro). Pre-1.13 path is gated behind a separate
/// MC pool because cubiomes does not support `getEndIslands` there.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct PopulationSeedRecord {
    pub mc: i32,
    pub x: i32,
    pub z: i32,
    pub pad: i32,
    pub ws: u64,
    pub pop_seed: u64,
}

fn write_population_seed_fixture(path: &Path) -> std::io::Result<()> {
    // mc_pool spans every dispatch leg of getPopulationSeed:
    // - 8 (V1_12): pre-1.13 `/2*2+1` path, Java RNG
    // - 17 (V1_13): 1.13–1.17 `|1` path, Java RNG
    // - 21 (V1_17): same Java-RNG path
    // - 22 (V1_18): 1.18+ Xoroshiro path
    // - 28 (V1_21): same Xoroshiro path, modern WD btree
    let mc_pool: [i32; 5] = [8, 17, 21, 22, 28];
    let per_mc: u64 = 500;
    let total = mc_pool.len() as u64 * per_mc;
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 56, total)?;

    let mut rng_state: u64 = 0x0123_4567_89ab_cdef;
    for &mc in &mc_pool {
        for _ in 0..per_mc {
            rng_state = lcg_step(rng_state);
            let ws = rng_state;
            rng_state = lcg_step(rng_state);
            // Range chunk coordinates -32..32 (block coords -512..512).
            let x = ((rng_state >> 32) as i32) % 1024 - 512;
            rng_state = lcg_step(rng_state);
            let z = ((rng_state >> 32) as i32) % 1024 - 512;
            let pop_seed = unsafe { ffi::cubiomes_call_get_population_seed(mc, ws, x, z) };
            let rec = PopulationSeedRecord {
                mc,
                x,
                z,
                pad: 0,
                ws,
                pop_seed,
            };
            file.write_all(bytemuck::bytes_of(&rec))?;
        }
    }
    file.flush()
}

/// `getEndIslands` parity record (kind = 57). One record per
/// `(mc, seed, chunk_x, chunk_z)` capturing the (0, 1, or 2) island
/// centres and radii.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct EndIslandsRecord {
    pub mc: i32,
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub n: i32,
    pub seed: u64,
    /// `[x0, y0, z0, r0, x1, y1, z1, r1]`. Unused slots are zero.
    pub islands: [i32; 8],
}

/// `mapEndIslandHeight` parity record (kind = 58). Each record
/// reduces a `(w, h)` height grid to a `(min, max, hash32)` triple
/// — full grids would balloon the fixture, but the digest catches
/// any per-cell drift.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct EndIslandHeightRecord {
    pub mc: i32,
    pub x: i32,
    pub z: i32,
    pub w: i32,
    pub h: i32,
    pub scale: i32,
    pub seed: u64,
    pub y_min_bits: u32,
    pub y_max_bits: u32,
    pub digest: u32,
    pub pad: u32,
}

/// `scanForQuads` parity record (kind = 71). For each (s48, x, z,
/// w, h) tuple, captures the count + first N=8 quad-base hit
/// positions returned by cubiomes' Swamp_Hut + radius=128 scan.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ScanForQuadsRecord {
    pub x: i32,
    pub z: i32,
    pub w: i32,
    pub h: i32,
    pub cnt: i32,
    pub pad: i32,
    pub s48: u64,
    pub out_xz: [i32; 16],
}

fn write_scan_for_quads_fixture(path: &Path) -> std::io::Result<()> {
    // Swamp_Hut, radius=128, 1.18 (any modern MC works since Swamp_Hut
    // structure config is stable). Use LOW20_QUAD_IDEAL constellations.
    const STY_SWAMP_HUT: i32 = 3;
    let low_bits: [u64; 3] = [0x43f18, 0xc751a, 0xf520a];
    // `salt` is the per-structure salt mixed by cubiomes' `moveStructure`
    // arithmetic. For Swamp_Hut 1.13+ the config salt is 14357620.
    let salt: u64 = 14_357_620;
    let total: u64 = 64;
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 71, total)?;

    let mut rng_state: u64 = 0x12af_face_b00b_c0de;
    for _ in 0..total {
        rng_state = lcg_step(rng_state);
        let s48 = rng_state & ((1u64 << 48) - 1);
        rng_state = lcg_step(rng_state);
        let x = ((rng_state >> 32) as i32) % 64 - 32;
        rng_state = lcg_step(rng_state);
        let z = ((rng_state >> 32) as i32) % 64 - 32;
        let w = 64_i32;
        let h = 64_i32;
        let mut buf = [0_i32; 16];
        let cnt = unsafe {
            ffi::cubiomes_call_scan_for_quads(
                22, // V1_18
                STY_SWAMP_HUT,
                128,
                s48,
                low_bits.as_ptr(),
                low_bits.len() as c_int,
                salt,
                x,
                z,
                w,
                h,
                buf.as_mut_ptr(),
                8,
            )
        };
        let rec = ScanForQuadsRecord {
            x,
            z,
            w,
            h,
            cnt,
            pad: 0,
            s48,
            out_xz: buf,
        };
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

/// `getLinkedGatewayPos` parity record (kind = 70). Each record
/// captures the resolved gateway destination for a (mc, seed,
/// src) input. `src` is one of the 20 fixed gateway anchor
/// positions.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LinkedGatewayPosRecord {
    pub mc: i32,
    pub src_x: i32,
    pub src_z: i32,
    pub dst_x: i32,
    pub dst_z: i32,
    pub pad: i32,
    pub seed: u64,
}

fn write_linked_gateway_pos_fixture(path: &Path) -> std::io::Result<()> {
    // Cover the three significant code paths:
    //   - 1.13/1.14 (MC ≤ 1.16): full surface-height search
    //   - 1.17+ (MC > MC_1_16): trivial (15, 15) corner
    //   - 1.18+: same trivial path with the modern btree.
    // Each call does several `isEndChunkEmpty` + 33×33 surface-height
    // generations, so keep the record count small (slow fixture-gen).
    let mc_pool: [i32; 4] = [17, 20, 22, 28];
    // Pick 4 of the 20 fixed gateway anchors (covers a range of angles).
    let anchors: [(i32, i32); 4] = [(96, 0), (-1, 96), (-96, -1), (0, -96)];
    let per_seed: u64 = 8;
    let total = (mc_pool.len() * anchors.len()) as u64 * per_seed;
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 70, total)?;

    let mut rng_state: u64 = 0xface_b00b_face_5eed;
    for &mc in &mc_pool {
        for &(sx, sz) in &anchors {
            for _ in 0..per_seed {
                rng_state = lcg_step(rng_state);
                let seed = rng_state;
                let mut dx: c_int = 0;
                let mut dz: c_int = 0;
                unsafe {
                    ffi::cubiomes_call_get_linked_gateway_pos(
                        mc,
                        seed,
                        sx,
                        sz,
                        std::ptr::from_mut(&mut dx),
                        std::ptr::from_mut(&mut dz),
                    );
                }
                let rec = LinkedGatewayPosRecord {
                    mc,
                    src_x: sx,
                    src_z: sz,
                    dst_x: dx,
                    dst_z: dz,
                    pad: 0,
                    seed,
                };
                file.write_all(bytemuck::bytes_of(&rec))?;
            }
        }
    }
    file.flush()
}

/// `getFixedEndGateways` parity record (kind = 69). The 20-position
/// output is stored as two `[i32; 20]` halves (Pod's auto-impl tops
/// out at length 32).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct FixedEndGatewaysRecord {
    pub mc: i32,
    pub pad: i32,
    pub seed: u64,
    pub xs: [i32; 20],
    pub zs: [i32; 20],
}

fn write_fixed_end_gateways_fixture(path: &Path) -> std::io::Result<()> {
    // mc is ignored by cubiomes; pick a single representative.
    let mc_pool: [i32; 3] = [17, 22, 28];
    let per_mc: u64 = 64;
    let total = mc_pool.len() as u64 * per_mc;
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 69, total)?;

    let mut rng_state: u64 = 0xface_1234_5678_b00b;
    for &mc in &mc_pool {
        for _ in 0..per_mc {
            rng_state = lcg_step(rng_state);
            let seed = rng_state;
            let mut buf = [0_i32; 40];
            unsafe {
                ffi::cubiomes_call_get_fixed_end_gateways(mc, seed, buf.as_mut_ptr());
            }
            let mut xs = [0_i32; 20];
            let mut zs = [0_i32; 20];
            for i in 0..20 {
                xs[i] = buf[i * 2];
                zs[i] = buf[i * 2 + 1];
            }
            let rec = FixedEndGatewaysRecord {
                mc,
                pad: 0,
                seed,
                xs,
                zs,
            };
            file.write_all(bytemuck::bytes_of(&rec))?;
        }
    }
    file.flush()
}

/// `getVariant` parity record (kind = 68). Captures the 17
/// integer-encoded fields of `StructureVariant` plus the `getVariant`
/// return code. `pad2` rounds the struct out to a multiple of the
/// u64 alignment (8 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GetVariantRecord {
    pub structure_type: i32,
    pub mc: i32,
    pub biome_id: i32,
    pub rc: i32,
    pub seed: u64,
    pub x: i32,
    pub z: i32,
    /// Order: `abandoned, giant, underground, airpocket, basement,
    /// cracked, size, start, biome, rotation, mirror, x, y, z, sx,
    /// sy, sz` (17 entries).
    pub fields: [i32; 17],
    pub pad: i32,
}

fn write_get_variant_fixture(path: &Path) -> std::io::Result<()> {
    // (structure_type, mc, biome_id) probes Village + Bastion across
    // the supported biome variants. -1 sentinel means "unused" (Bastion).
    // V1_14 (ord 17) and V1_18 (ord 22) cover the legacy and modern
    // rotation formulas, plus V1_16_1 for the start/rotation swap.
    #[allow(clippy::type_complexity)]
    let probes: &[(i32, i32, i32)] = &[
        // Village: 5 biomes × 3 MC versions.
        (5, 17, 1),   // Village V1_14 plains
        (5, 17, 2),   // Village V1_14 desert
        (5, 17, 35),  // Village V1_14 savanna
        (5, 17, 5),   // Village V1_14 taiga
        (5, 17, 12),  // Village V1_14 snowy_tundra
        (5, 17, 177), // Village V1_14 meadow (falls through to plains; pre-1.18 rejects)
        (5, 22, 1),   // Village V1_18 plains
        (5, 22, 2),   // Village V1_18 desert
        (5, 22, 35),  // Village V1_18 savanna
        (5, 22, 5),   // Village V1_18 taiga
        (5, 22, 12),  // Village V1_18 snowy_tundra
        (5, 22, 177), // Village V1_18 meadow
        (5, 28, 1),   // Village V1_21 plains
        // Bastion: biome_id ignored.
        (19, 19, -1), // Bastion V1_16_1 (start/rotation swap)
        (19, 21, -1), // Bastion V1_17
        (19, 22, -1), // Bastion V1_18
        (19, 28, -1), // Bastion V1_21 WD
        // Ancient_City: biome_id ignored.
        (13, 23, -1), // Ancient_City V1_19_2
        (13, 28, -1), // Ancient_City V1_21 WD
        // Trial_Chambers: biome_id ignored.
        (24, 26, -1), // Trial_Chambers V1_21_1
        (24, 28, -1), // Trial_Chambers V1_21 WD
        // Monument: biome_id ignored, fixed bounding box.
        (8, 22, -1), // Monument V1_18
        // Desert_Pyramid / Jungle_Temple / Swamp_Hut: pre-1.20 vs 1.20+.
        (1, 19, -1), // Desert_Pyramid V1_16_1 (pre-1.20)
        (1, 25, -1), // Desert_Pyramid V1_20 (with rotation)
        (2, 25, -1), // Jungle_Temple V1_20
        (3, 25, -1), // Swamp_Hut V1_20
        (3, 28, -1), // Swamp_Hut V1_21 WD
        // Igloo: pre-1.13 (different seed) vs 1.13+.
        (4, 15, -1), // Igloo V1_12 (population-seed re-seed)
        (4, 22, -1), // Igloo V1_18
        (4, 28, -1), // Igloo V1_21 WD
        // Geode: 1.17 (JavaRng) vs 1.18+ (Xoroshiro). biome_id ignored.
        (17, 20, -1), // Geode V1_17 (JavaRng path)
        (17, 22, -1), // Geode V1_18 (Xoroshiro path)
        (17, 28, -1), // Geode V1_21 WD
        // Ruined_Portal: biome_id matters (categorizes to plains/desert/etc).
        (11, 22, 1),   // V1_18 plains
        (11, 22, 2),   // V1_18 desert
        (11, 22, 6),   // V1_18 swamp
        (11, 22, 21),  // V1_18 jungle
        (11, 22, 0),   // V1_18 ocean
        (11, 22, 3),   // V1_18 mountains (fallback)
        (11, 22, 184), // V1_18 mangrove_swamp → swamp fallback
        (11, 22, 177), // V1_18 meadow → no category → plains
        (11, 28, 1),   // V1_21 WD plains
        // Ruined_Portal_N: same logic, nether dim. biome must be in nether category.
        (12, 22, 8), // V1_18 nether_wastes
    ];
    let per_combo: u64 = 64;
    let total = probes.len() as u64 * per_combo;
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 68, total)?;

    let mut rng_state: u64 = 0xface_b00b_5eed_face;
    for &(sty, mc, biome) in probes {
        for _ in 0..per_combo {
            rng_state = lcg_step(rng_state);
            let seed = rng_state;
            rng_state = lcg_step(rng_state);
            let x = ((rng_state >> 32) as i32) % 2048 - 1024;
            rng_state = lcg_step(rng_state);
            let z = ((rng_state >> 32) as i32) % 2048 - 1024;
            let mut out = [0_i32; 17];
            let rc = unsafe {
                ffi::cubiomes_call_get_variant(sty, mc, seed, x, z, biome, out.as_mut_ptr())
            };
            let rec = GetVariantRecord {
                structure_type: sty,
                mc,
                biome_id: biome,
                rc,
                seed,
                x,
                z,
                fields: out,
                pad: 0,
            };
            file.write_all(bytemuck::bytes_of(&rec))?;
        }
    }
    file.flush()
}

/// `isViableStructurePos` parity record (kind = 67). Covers only
/// the Nether and End branches in this fixture; the Overworld
/// branch needs the `mapViableBiome` layer hook (follow-up).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ViableStructurePosRecord {
    pub mc: i32,
    pub dim: i32,
    pub structure_type: i32,
    pub viable: i32,
    pub seed: u64,
    pub x: i32,
    pub z: i32,
}

fn write_viable_structure_pos_fixture(path: &Path) -> std::io::Result<()> {
    // Nether: Fortress (1.0+), Bastion 1.16.1+ (with getVariant
    // path for 1.18+), Ruined_Portal_N (1.16.1+).
    // End: End_City (1.9+), End_Gateway (1.13+).
    // Overworld 1.18+: L_feature path + Village + always-viable.
    let combos: [(i32, i32, i32); 48] = [
        // (mc, dim, structure_type)
        // Nether
        (10, -1, 18), // V1_7 Nether Fortress (returns true)
        (17, -1, 18), // V1_14 Nether Fortress
        (19, -1, 18), // V1_16_1 Nether Fortress 1.18- (returns true)
        (19, -1, 19), // V1_16_1 Nether Bastion
        (21, -1, 19), // V1_17 Nether Bastion
        (22, -1, 19), // V1_18 Nether Bastion (getVariant path)
        (23, -1, 19), // V1_19_2 Nether Bastion (sampleY=33>>2)
        (28, -1, 19), // V1_21 Nether Bastion
        (22, -1, 18), // V1_18 Nether Fortress (bastion-exclusion check)
        (19, -1, 12), // V1_16_1 Nether Ruined_Portal_N
        // End
        (15, 1, 20), // V1_12 End EndCity
        (22, 1, 20), // V1_18 End EndCity
        // Overworld 1.18+
        (22, 0, 1),  // V1_18 OW Desert_Pyramid
        (22, 0, 2),  // V1_18 OW Jungle_Temple
        (22, 0, 3),  // V1_18 OW Swamp_Hut
        (22, 0, 4),  // V1_18 OW Igloo
        (22, 0, 5),  // V1_18 OW Village
        (22, 0, 6),  // V1_18 OW Ocean_Ruin
        (22, 0, 7),  // V1_18 OW Shipwreck
        (22, 0, 14), // V1_18 OW Treasure
        (22, 0, 9),  // V1_18 OW Mansion
        (22, 0, 11), // V1_18 OW Ruined_Portal (always viable)
        (22, 0, 17), // V1_18 OW Geode (always viable)
        (22, 0, 15), // V1_18 OW Mineshaft (always viable)
        // Overworld L_jigsaw (1.19_2+ Ancient_City, 1.21_1+ Trial_Chambers)
        (23, 0, 13), // V1_19_2 OW Ancient_City
        (28, 0, 13), // V1_21 WD OW Ancient_City
        (28, 0, 24), // V1_21 WD OW Trial_Chambers
        // Outpost 1.18+ (Village proximity check + variant centroid sample).
        (22, 0, 10), // V1_18 OW Outpost
        (28, 0, 10), // V1_21 WD OW Outpost
        // Monument 1.18+ (deep-ocean center + 29-block ocean radius).
        (22, 0, 8), // V1_18 OW Monument
        (28, 0, 8), // V1_21 WD OW Monument
        // Pre-1.18 Overworld L_feature path (V1_14 = ord 17 covers
        // 1.16-1.17 sample shape via L_RIVER_MIX_4; V1_12 = ord 15
        // covers the pre-1.16 L_VORONOI_1 shape).
        (15, 0, 1),  // V1_12 OW Desert_Pyramid (pre-1.16 voronoi sample)
        (15, 0, 3),  // V1_12 OW Swamp_Hut
        (17, 0, 1),  // V1_14 OW Desert_Pyramid (1.16-1.17 river-mix)
        (17, 0, 3),  // V1_14 OW Swamp_Hut
        (21, 0, 4),  // V1_17 OW Igloo
        (21, 0, 6),  // V1_17 OW Ocean_Ruin
        (21, 0, 14), // V1_17 OW Treasure
        // Pre-1.18 Village (V1_15 uses voronoi sample, others use river-mix;
        // pre-1.10 has the extra chunk-corner check).
        (15, 0, 5), // V1_12 OW Village
        (17, 0, 5), // V1_14 OW Village
        (18, 0, 5), // V1_15 OW Village (voronoi sample exclusively in 1.15)
        // Always-viable.
        (17, 0, 15), // V1_14 OW Mineshaft
        (21, 0, 11), // V1_17 OW Ruined_Portal (1.16.1+ always viable)
        (21, 0, 17), // V1_17 OW Geode (always viable)
        // Desert_Well pre-1.18.
        (17, 0, 16), // V1_14 OW Desert_Well
        (21, 0, 16), // V1_17 OW Desert_Well
        // Pre-1.18 Monument: implementation is in place but has a
        // residual `areBiomesViable` divergence vs cubiomes that
        // surfaces under specific seeds (likely Range-Y or sy=1
        // handling); fixtures land in a follow-up after the
        // diff is localised.
        // Pre-1.18 Mansion: V1_14 + V1_17.
        (17, 0, 9), // V1_14 OW Mansion
        (21, 0, 9), // V1_17 OW Mansion
    ];
    let per_combo: u64 = 64;
    let total = combos.len() as u64 * per_combo;
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 67, total)?;

    let mut rng_state: u64 = 0x55a1_5e91_b00b_face;
    for &(mc, dim, sty) in &combos {
        for _ in 0..per_combo {
            rng_state = lcg_step(rng_state);
            let seed = rng_state;
            rng_state = lcg_step(rng_state);
            let x = ((rng_state >> 32) as i32) % 4096 - 2048;
            rng_state = lcg_step(rng_state);
            let z = ((rng_state >> 32) as i32) % 4096 - 2048;
            let viable =
                unsafe { ffi::cubiomes_call_is_viable_structure_pos(mc, dim, sty, seed, x, z, 0) };
            let rec = ViableStructurePosRecord {
                mc,
                dim,
                structure_type: sty,
                viable,
                seed,
                x,
                z,
            };
            file.write_all(bytemuck::bytes_of(&rec))?;
        }
    }
    file.flush()
}

/// `isViableFeatureBiome` parity record (kind = 66). One record per
/// (mc, structure_type, biome_id) triple — cubiomes panics on
/// unsupported types so we skip Feature / EndIsland / Geode.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ViableFeatureBiomeRecord {
    pub mc: i32,
    pub structure_type: i32,
    pub biome_id: i32,
    pub viable: i32,
}

fn write_viable_feature_biome_fixture(path: &Path) -> std::io::Result<()> {
    // Cover every MC version with a meaningful step and every
    // structure type cubiomes' isViableFeatureBiome accepts.
    let mc_pool: [i32; 12] = [3, 5, 8, 11, 14, 16, 17, 19, 20, 22, 25, 28];
    // ords from cubiomes' enum StructureType, excluding Feature (0),
    // EndIsland (22), Geode (17) which trigger the fatal panic arm.
    let struct_types: [i32; 22] = [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 19, 20, 21, 23, 24,
    ];
    // 256 biome IDs cover every cubiomes biome (and reserve some for
    // negative tests against ids that no version uses).
    let total: u64 = mc_pool.len() as u64 * struct_types.len() as u64 * 256;
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 66, total)?;

    for &mc in &mc_pool {
        for &sty in &struct_types {
            for biome_id in 0..256_i32 {
                let viable =
                    unsafe { ffi::cubiomes_call_is_viable_feature_biome(mc, sty, biome_id) };
                let rec = ViableFeatureBiomeRecord {
                    mc,
                    structure_type: sty,
                    biome_id,
                    viable,
                };
                file.write_all(bytemuck::bytes_of(&rec))?;
            }
        }
    }
    file.flush()
}

/// `getSpawn` parity record (kind = 65). MC pool matches
/// `estimate_spawn` minus B1.7 (which short-circuits identically).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GetSpawnRecord {
    pub mc: i32,
    pub pad: i32,
    pub seed: u64,
    pub spawn_x: i32,
    pub spawn_z: i32,
}

fn write_get_spawn_fixture(path: &Path) -> std::io::Result<()> {
    // 1.0 (Beta returns estimate verbatim — interesting but trivial),
    // 1.7 (legacy 1.13-1.17 branch), 1.18 (modern spiral),
    // 1.21 WD (modern spiral, new btree).
    //
    // 1.12 path uses the slow 1000-iter random walk which can take
    // ~100ms per seed; skip it to keep fixture-gen quick.
    let mc_pool: [i32; 3] = [10, 22, 28];
    let per_mc: u64 = 16;
    let total = mc_pool.len() as u64 * per_mc;
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 65, total)?;

    let mut rng_state: u64 = 0x0000_5e94_b00b_5eed;
    for &mc in &mc_pool {
        for _ in 0..per_mc {
            rng_state = lcg_step(rng_state);
            let seed = rng_state;
            let mut px: c_int = 0;
            let mut pz: c_int = 0;
            unsafe {
                ffi::cubiomes_call_get_spawn(
                    mc,
                    seed,
                    std::ptr::from_mut(&mut px),
                    std::ptr::from_mut(&mut pz),
                );
            }
            let rec = GetSpawnRecord {
                mc,
                pad: 0,
                seed,
                spawn_x: px,
                spawn_z: pz,
            };
            file.write_all(bytemuck::bytes_of(&rec))?;
        }
    }
    file.flush()
}

/// `mapApproxHeight` parity record (kind = 64). Records the
/// reduced `(min, max, digest)` triple of the per-record height
/// grid and ids hash.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct MapApproxHeightRecord {
    pub mc: i32,
    pub dim: i32,
    pub x: i32,
    pub z: i32,
    pub w: i32,
    pub h: i32,
    pub rc: i32,
    pub pad: i32,
    pub seed: u64,
    pub y_min_bits: u32,
    pub y_max_bits: u32,
    pub y_digest: u32,
    pub ids_digest: u32,
}

fn write_map_approx_height_fixture(path: &Path) -> std::io::Result<()> {
    // (mc, dim) combinations covering each dispatch branch:
    //   - Overworld 1.7 (legacy layered 1.0-1.17 path)
    //   - Overworld 1.18 (1.18+ BiomeNoise path)
    //   - Overworld 1.21 WD (1.18+ BiomeNoise path)
    //   - End 1.13 (delegates to mapEndSurfaceHeight scale 4)
    //   - End 1.18 (same)
    //   - Nether 1.18 (returns 127, y unwritten)
    //
    // MC 1.16.1 (ord 19) is currently excluded — the legacy path
    // diverges there in a way 1.7 doesn't; a follow-up stage will
    // investigate (likely a per-version layer-stack edge case).
    let combos: [(i32, i32); 6] = [(10, 0), (22, 0), (28, 0), (17, 1), (22, 1), (22, -1)];
    let per_combo: u64 = 30;
    let (w, h) = (8_i32, 8_i32);
    let total = combos.len() as u64 * per_combo;
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 64, total)?;

    let mut rng_state: u64 = 0xa110_de_c0de_5eed;
    for &(mc, dim) in &combos {
        for _ in 0..per_combo {
            rng_state = lcg_step(rng_state);
            let seed = rng_state;
            rng_state = lcg_step(rng_state);
            // Sample coordinates at scale 4 — small range near origin.
            let x = ((rng_state >> 32) as i32) % 256 - 128;
            rng_state = lcg_step(rng_state);
            let z = ((rng_state >> 32) as i32) % 256 - 128;
            let mut y = vec![0.0_f32; (w * h) as usize];
            let mut ids = vec![0_i32; (w * h) as usize];
            let rc = unsafe {
                ffi::cubiomes_call_map_approx_height(
                    mc,
                    dim,
                    seed,
                    x,
                    z,
                    w,
                    h,
                    y.as_mut_ptr(),
                    ids.as_mut_ptr(),
                )
            };
            let mut y_min = f32::INFINITY;
            let mut y_max = f32::NEG_INFINITY;
            let mut y_digest: u32 = 0;
            for &v in &y {
                if v < y_min {
                    y_min = v;
                }
                if v > y_max {
                    y_max = v;
                }
                y_digest = hash32(y_digest.wrapping_add(v.to_bits()));
            }
            let mut ids_digest: u32 = 0;
            for &id in &ids {
                ids_digest = hash32(ids_digest.wrapping_add(id as u32));
            }
            let rec = MapApproxHeightRecord {
                mc,
                dim,
                x,
                z,
                w,
                h,
                rc,
                pad: 0,
                seed,
                y_min_bits: y_min.to_bits(),
                y_max_bits: y_max.to_bits(),
                y_digest,
                ids_digest,
            };
            file.write_all(bytemuck::bytes_of(&rec))?;
        }
    }
    file.flush()
}

/// `getOptimalAfk` parity record (kind = 63). Captures the optimal
/// AFK `(x, z)` and the achieved block-in-range count for a random
/// 4-witch-hut footprint.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct OptimalAfkRecord {
    pub p0x: i32,
    pub p0z: i32,
    pub p1x: i32,
    pub p1z: i32,
    pub p2x: i32,
    pub p2z: i32,
    pub p3x: i32,
    pub p3z: i32,
    pub ax: i32,
    pub ay: i32,
    pub az: i32,
    pub spcnt: i32,
    pub afk_x: i32,
    pub afk_z: i32,
}

fn write_optimal_afk_fixture(path: &Path) -> std::io::Result<()> {
    // ax/ay/az from cubiomes' witch hut footprint constants
    // (the most common quad-base use). Use 7x9x7 (witch hut).
    let total: u64 = 256;
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 63, total)?;

    let mut rng_state: u64 = 0x12af_b00b_0000_face;
    for _ in 0..total {
        // Place each point in its own quadrant so that no two share
        // an x or z. This avoids cubiomes' OOB-midpoint UB
        // (`(int)NaN = INT_MIN` on x86) which fires when two
        // anchor points share a coordinate, making a midpoint land
        // outside the `(ax/2)`-shifted bounding box.
        let quadrants = [
            (-40, -10, -40, -10),
            (10, 40, -40, -10),
            (-40, -10, 10, 40),
            (10, 40, 10, 40),
        ];
        let mut pts = [(0_i32, 0_i32); 4];
        for (i, q) in quadrants.iter().enumerate() {
            rng_state = lcg_step(rng_state);
            let span_x = q.1 - q.0;
            let x = q.0 + ((rng_state >> 32) as u32 % span_x as u32) as i32;
            rng_state = lcg_step(rng_state);
            let span_z = q.3 - q.2;
            let z = q.2 + ((rng_state >> 32) as u32 % span_z as u32) as i32;
            pts[i] = (x, z);
        }

        let ax = 7_i32;
        let ay = 9_i32;
        let az = 7_i32;
        let mut px: c_int = 0;
        let mut pz: c_int = 0;
        let mut spcnt: c_int = 0;
        unsafe {
            ffi::cubiomes_call_get_optimal_afk(
                std::ptr::from_mut(&mut px),
                std::ptr::from_mut(&mut pz),
                std::ptr::from_mut(&mut spcnt),
                pts[0].0,
                pts[0].1,
                pts[1].0,
                pts[1].1,
                pts[2].0,
                pts[2].1,
                pts[3].0,
                pts[3].1,
                ax,
                ay,
                az,
            );
        }
        let rec = OptimalAfkRecord {
            p0x: pts[0].0,
            p0z: pts[0].1,
            p1x: pts[1].0,
            p1z: pts[1].1,
            p2x: pts[2].0,
            p2z: pts[2].1,
            p3x: pts[3].0,
            p3z: pts[3].1,
            ax,
            ay,
            az,
            spcnt,
            afk_x: px,
            afk_z: pz,
        };
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

/// `getBiomeDepthAndScale` parity record (kind = 62). Exercises
/// each biome id 0..256.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BiomeDepthScaleRecord {
    pub id: i32,
    pub found: i32,
    pub grass: i32,
    pub pad: i32,
    pub depth_bits: u64,
    pub scale_bits: u64,
}

fn write_biome_depth_scale_fixture(path: &Path) -> std::io::Result<()> {
    let total: u64 = 256;
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 62, total)?;

    for id in 0..256_i32 {
        let mut depth = 0.0_f64;
        let mut scale = 0.0_f64;
        let mut grass: c_int = 0;
        let found = unsafe {
            ffi::cubiomes_call_get_biome_depth_and_scale(
                id,
                std::ptr::from_mut(&mut depth),
                std::ptr::from_mut(&mut scale),
                std::ptr::from_mut(&mut grass),
            )
        };
        let rec = BiomeDepthScaleRecord {
            id,
            found,
            grass,
            pad: 0,
            depth_bits: depth.to_bits(),
            scale_bits: scale.to_bits(),
        };
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

/// `isEndChunkEmpty` parity record (kind = 61).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct EndChunkEmptyRecord {
    pub mc: i32,
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub empty: i32,
    pub seed: u64,
}

fn write_end_chunk_empty_fixture(path: &Path) -> std::io::Result<()> {
    let mc_pool: [i32; 3] = [17, 22, 28];
    let per_mc: u64 = 80;
    let total = mc_pool.len() as u64 * per_mc;
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 61, total)?;

    let mut rng_state: u64 = 0xface_0fff_5eed_b00b;
    for &mc in &mc_pool {
        for _ in 0..per_mc {
            rng_state = lcg_step(rng_state);
            let seed = rng_state;
            rng_state = lcg_step(rng_state);
            // Range close to the center where small_end_islands /
            // small noise islands matter most; chunk coords ±256.
            let cx = ((rng_state >> 32) as i32) % 512 - 256;
            rng_state = lcg_step(rng_state);
            let cz = ((rng_state >> 32) as i32) % 512 - 256;
            let empty = unsafe { ffi::cubiomes_call_is_end_chunk_empty(mc, seed, cx, cz) };
            let rec = EndChunkEmptyRecord {
                mc,
                chunk_x: cx,
                chunk_z: cz,
                empty,
                seed,
            };
            file.write_all(bytemuck::bytes_of(&rec))?;
        }
    }
    file.flush()
}

/// `mapEndSurfaceHeight` parity record (kind = 60). One row per
/// `(mc, seed, x, z, w, h, scale, ymin)` with the digest+min/max
/// of the resulting f32 grid.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct EndSurfaceHeightRecord {
    pub mc: i32,
    pub x: i32,
    pub z: i32,
    pub w: i32,
    pub h: i32,
    pub scale: i32,
    pub ymin: i32,
    pub pad: i32,
    pub seed: u64,
    pub y_min_bits: u32,
    pub y_max_bits: u32,
    pub digest: u32,
    pub pad2: u32,
}

fn write_end_surface_height_fixture(path: &Path) -> std::io::Result<()> {
    let mc_pool: [i32; 3] = [17, 22, 28];
    // (scale, w, h) — exercise each supported scale; smaller grids
    // are fine since the column-sampling logic is per-cell.
    let scales: [(i32, i32, i32); 4] = [(1, 8, 8), (2, 8, 8), (4, 6, 6), (8, 4, 4)];
    let per_combo: u64 = 12;
    let total = mc_pool.len() as u64 * scales.len() as u64 * per_combo;
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 60, total)?;

    let mut rng_state: u64 = 0xc011_a87e_face_e711;
    for &mc in &mc_pool {
        for &(scale, w, h) in &scales {
            for _ in 0..per_combo {
                rng_state = lcg_step(rng_state);
                let seed = rng_state;
                rng_state = lcg_step(rng_state);
                let x = ((rng_state >> 32) as i32) % 256 - 128;
                rng_state = lcg_step(rng_state);
                let z = ((rng_state >> 32) as i32) % 256 - 128;
                rng_state = lcg_step(rng_state);
                let ymin = (rng_state as i32) & 0x3f;
                let mut y = vec![0.0_f32; (w * h) as usize];
                unsafe {
                    ffi::cubiomes_call_map_end_surface_height(
                        mc,
                        seed,
                        x,
                        z,
                        w,
                        h,
                        scale,
                        ymin,
                        y.as_mut_ptr(),
                    );
                }
                let mut y_min = f32::INFINITY;
                let mut y_max = f32::NEG_INFINITY;
                let mut digest: u32 = 0;
                for &v in &y {
                    if v < y_min {
                        y_min = v;
                    }
                    if v > y_max {
                        y_max = v;
                    }
                    digest = hash32(digest.wrapping_add(v.to_bits()));
                }
                let rec = EndSurfaceHeightRecord {
                    mc,
                    x,
                    z,
                    w,
                    h,
                    scale,
                    ymin,
                    pad: 0,
                    seed,
                    y_min_bits: y_min.to_bits(),
                    y_max_bits: y_max.to_bits(),
                    digest,
                    pad2: 0,
                };
                file.write_all(bytemuck::bytes_of(&rec))?;
            }
        }
    }
    file.flush()
}

/// `getEndHeightNoise` parity record (kind = 59). Single height
/// sample per `(mc, seed, x, z, range)`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct EndHeightNoiseRecord {
    pub mc: i32,
    pub x: i32,
    pub z: i32,
    pub range: i32,
    pub seed: u64,
    pub height_bits: u32,
    pub pad: u32,
}

fn write_end_height_noise_fixture(path: &Path) -> std::io::Result<()> {
    let mc_pool: [i32; 4] = [17, 20, 22, 28];
    let per_mc: u64 = 500;
    let total = mc_pool.len() as u64 * per_mc;
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 59, total)?;

    let mut rng_state: u64 = 0xa53e_face_b00b_5eed;
    for &mc in &mc_pool {
        for _ in 0..per_mc {
            rng_state = lcg_step(rng_state);
            let seed = rng_state;
            rng_state = lcg_step(rng_state);
            // Use 8-block-per-cell coordinates: range -256..256 → block coords -2048..2048.
            let x = ((rng_state >> 32) as i32) % 512 - 256;
            rng_state = lcg_step(rng_state);
            let z = ((rng_state >> 32) as i32) % 512 - 256;
            rng_state = lcg_step(rng_state);
            // range in {0, 4, 12, 16} — exercise both the default
            // (0 → 12) branch and explicit overrides.
            let range_pool = [0_i32, 4, 12, 16];
            let range = range_pool[(rng_state as usize) % range_pool.len()];
            let h = unsafe { ffi::cubiomes_call_end_height_noise(mc, seed, x, z, range) };
            let rec = EndHeightNoiseRecord {
                mc,
                x,
                z,
                range,
                seed,
                height_bits: h.to_bits(),
                pad: 0,
            };
            file.write_all(bytemuck::bytes_of(&rec))?;
        }
    }
    file.flush()
}

fn write_end_island_height_fixture(path: &Path) -> std::io::Result<()> {
    // 1.13+ — pre-1.13 is unsupported by cubiomes anyway. Pre-1.14
    // skips the outer-ring fallback in mapEndBiome but the island
    // search logic itself is unchanged.
    let mc_pool: [i32; 4] = [17, 20, 22, 28];
    // Probe `scale = 1, 4, 16` with a small grid each — full
    // 16-block grids would inflate the fixture per record.
    let scales: [(i32, i32, i32); 3] = [(1, 16, 16), (4, 8, 8), (16, 4, 4)];
    let per_combo: u64 = 30;
    let total = mc_pool.len() as u64 * scales.len() as u64 * per_combo;
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 58, total)?;

    let mut rng_state: u64 = 0xfeed_face_dead_c0de;
    for &mc in &mc_pool {
        for &(scale, w, h) in &scales {
            for _ in 0..per_combo {
                rng_state = lcg_step(rng_state);
                let seed = rng_state;
                rng_state = lcg_step(rng_state);
                // Block coords / scale. Pick from a chunk-ish range
                // near the End origin where small_end_islands are
                // most common.
                let x = ((rng_state >> 32) as i32) % 64 - 32;
                rng_state = lcg_step(rng_state);
                let z = ((rng_state >> 32) as i32) % 64 - 32;
                let mut y = vec![0.0_f32; (w * h) as usize];
                unsafe {
                    ffi::cubiomes_call_map_end_island_height(
                        mc,
                        seed,
                        x,
                        z,
                        w,
                        h,
                        scale,
                        y.as_mut_ptr(),
                    );
                }
                let mut y_min = f32::INFINITY;
                let mut y_max = f32::NEG_INFINITY;
                let mut digest: u32 = 0;
                for &v in &y {
                    if v < y_min {
                        y_min = v;
                    }
                    if v > y_max {
                        y_max = v;
                    }
                    digest = hash32(digest.wrapping_add(v.to_bits()));
                }
                let rec = EndIslandHeightRecord {
                    mc,
                    x,
                    z,
                    w,
                    h,
                    scale,
                    seed,
                    y_min_bits: y_min.to_bits(),
                    y_max_bits: y_max.to_bits(),
                    digest,
                    pad: 0,
                };
                file.write_all(bytemuck::bytes_of(&rec))?;
            }
        }
    }
    file.flush()
}

fn write_end_islands_fixture(path: &Path) -> std::io::Result<()> {
    // mc_pool covers each version dispatch of getEndIslands:
    // - 17 (V1_13): integer-rarity, Java RNG, 1-in-14 hit rate
    // - 20 (V1_16): same integer-rarity branch as V1_13
    // - 21 (V1_17): float-rarity 1/14, Java RNG, 2nd island via nextInt(4)==0
    // - 22 (V1_18): Xoroshiro, 2nd island via xNextIntJ(4)==3
    // - 28 (V1_21): same Xoroshiro path on the modern btree.
    let mc_pool: [i32; 5] = [17, 20, 21, 22, 28];
    let per_mc: u64 = 2000;
    let total = mc_pool.len() as u64 * per_mc;
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 57, total)?;

    let mut rng_state: u64 = 0x55a1_7e51_5e91_dead;
    for &mc in &mc_pool {
        for _ in 0..per_mc {
            rng_state = lcg_step(rng_state);
            let seed = rng_state;
            rng_state = lcg_step(rng_state);
            let cx = ((rng_state >> 32) as i32) % 256 - 128;
            rng_state = lcg_step(rng_state);
            let cz = ((rng_state >> 32) as i32) % 256 - 128;
            let mut buf = [0_i32; 8];
            let n =
                unsafe { ffi::cubiomes_call_get_end_islands(mc, seed, cx, cz, buf.as_mut_ptr()) };
            let rec = EndIslandsRecord {
                mc,
                chunk_x: cx,
                chunk_z: cz,
                n,
                seed,
                islands: buf,
            };
            file.write_all(bytemuck::bytes_of(&rec))?;
        }
    }
    file.flush()
}

/// Biome predicate parity record (kind = 53). For each `(mc, id)`
/// pair, stores cubiomes' `biomeExists`, `isOverworld`, and
/// `isStrongholdBiome` outputs.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BiomePredicateRecord {
    pub mc: i32,
    pub id: i32,
    pub exists: i32,
    pub is_overworld: i32,
    pub is_stronghold: i32,
    pub pad: i32,
}

fn write_biome_predicates_fixture(path: &Path) -> std::io::Result<()> {
    let mc_pool: [i32; 11] = [1, 2, 3, 4, 9, 10, 15, 16, 22, 25, 28];
    let total = mc_pool.len() as u64 * 256;
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 53, total)?;

    for &mc in &mc_pool {
        for id in 0..256_i32 {
            let exists = unsafe { ffi::cubiomes_call_biome_exists(mc, id) };
            let is_overworld = unsafe { ffi::cubiomes_call_is_overworld(mc, id) };
            let is_stronghold = unsafe { ffi::cubiomes_call_is_stronghold_biome(mc, id) };
            let rec = BiomePredicateRecord {
                mc,
                id,
                exists,
                is_overworld,
                is_stronghold,
                pad: 0,
            };
            file.write_all(bytemuck::bytes_of(&rec))?;
        }
    }
    file.flush()
}

/// `initFirstStronghold` parity record (kind = 51). `(mc, seed) →
/// (first_x, first_z)` of the first stronghold's approximate
/// position.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct StrongholdInitRecord {
    pub mc: i32,
    pub pad: i32,
    pub seed: u64,
    pub first_x: i32,
    pub first_z: i32,
}

const STRONGHOLD_INIT_RECORDS: u64 = 2048;

fn write_stronghold_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 51, STRONGHOLD_INIT_RECORDS)?;

    let mc_pool: [i32; 6] = [3, 10, 12, 15, 22, 28];
    let mut rng_state: u64 = 0x0000_5478_0000_0011;
    for _ in 0..STRONGHOLD_INIT_RECORDS {
        rng_state = lcg_step(rng_state);
        let mc = mc_pool[(rng_state as usize) % mc_pool.len()];
        rng_state = lcg_step(rng_state);
        let seed = rng_state;
        let mut px: c_int = 0;
        let mut pz: c_int = 0;
        unsafe {
            ffi::cubiomes_call_init_first_stronghold(
                mc,
                seed,
                std::ptr::from_mut(&mut px),
                std::ptr::from_mut(&mut pz),
            );
        }
        let rec = StrongholdInitRecord {
            mc,
            pad: 0,
            seed,
            first_x: px,
            first_z: pz,
        };
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

/// `getMineshafts` parity record (kind = 52). Records the total
/// count plus an XOR-folded digest of the (x, z) pair stream over
/// a small chunk rectangle.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct MineshaftRecord {
    pub mc: i32,
    pub cx0: i32,
    pub cz0: i32,
    pub cx1: i32,
    pub cz1: i32,
    pub count: i32,
    pub digest: u32,
    pub pad0: u32,
    pub seed: u64,
}

const MINESHAFT_RECORDS: u64 = 256;
const MINESHAFT_N_MAX: i32 = 4096;

#[allow(clippy::many_single_char_names)]
fn write_mineshaft_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 52, MINESHAFT_RECORDS)?;

    let mc_pool: [i32; 4] = [3, 15, 22, 28];
    let mut rng_state: u64 = 0x0000_8e8e_88a8_0000;
    for _ in 0..MINESHAFT_RECORDS {
        rng_state = lcg_step(rng_state);
        let mc = mc_pool[(rng_state as usize) % mc_pool.len()];
        rng_state = lcg_step(rng_state);
        let seed = rng_state;
        rng_state = lcg_step(rng_state);
        let cx0 = (rng_state as i32) % 256 - 128;
        rng_state = lcg_step(rng_state);
        let cz0 = (rng_state as i32) % 256 - 128;
        rng_state = lcg_step(rng_state);
        let width = ((rng_state & 0x3f) as i32) + 1;
        let height = ((rng_state >> 8) & 0x3f) as i32 + 1;
        let cx1 = cx0 + width;
        let cz1 = cz0 + height;

        let mut out_xz = vec![0_i32; (MINESHAFT_N_MAX * 2) as usize];
        let mut total: c_int = 0;
        unsafe {
            ffi::cubiomes_call_get_mineshafts(
                mc,
                seed,
                cx0,
                cz0,
                cx1,
                cz1,
                out_xz.as_mut_ptr(),
                MINESHAFT_N_MAX,
                std::ptr::from_mut(&mut total),
            );
        }
        let written = (total as usize).min(MINESHAFT_N_MAX as usize);
        let digest = digest_i32_slice(&out_xz[..written * 2]);

        let rec = MineshaftRecord {
            mc,
            cx0,
            cz0,
            cx1,
            cz1,
            count: total,
            digest,
            pad0: 0,
            seed,
        };
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

/// `getStructurePos` parity record (kind = 48). For each random
/// `(structure_type, mc, seed, reg_x, reg_z)` tuple, stores the
/// cubiomes-reported `valid` flag plus the attempt position when
/// `valid = 1`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct StructurePosRecord {
    pub structure_type: i32,
    pub mc: i32,
    pub seed: u64,
    pub reg_x: i32,
    pub reg_z: i32,
    pub pos_x: i32,
    pub pos_z: i32,
    pub valid: i32,
    pub pad: i32,
}

const STRUCTURE_POS_RECORDS: u64 = 2048;

#[allow(clippy::many_single_char_names)]
fn write_structure_pos_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 48, STRUCTURE_POS_RECORDS)?;

    // (structure_type_ord, min_mc_ord) — sampled from cubiomes'
    // enum. Skip Mineshaft (15) since it isn't a region-grid
    // structure. Bastion (19) covers both 1.16+/1.17 (`getRegPos`)
    // and 1.18+ (`chunkGenerateRnd`) paths via the same min_mc=19.
    let types: [(i32, i32); 22] = [
        (0, 1),   // Feature (Beta-1.12 only)
        (1, 6),   // Desert_Pyramid (1.3+)
        (2, 6),   // Jungle_Temple (1.3+)
        (3, 7),   // Swamp_Hut (1.4+)
        (4, 12),  // Igloo (1.9+)
        (5, 2),   // Village (B1.8+)
        (6, 16),  // Ocean_Ruin (1.13+)
        (7, 16),  // Shipwreck (1.13+)
        (8, 11),  // Monument (1.8+)
        (9, 14),  // Mansion (1.11+)
        (10, 17), // Outpost (1.14+)
        (11, 19), // Ruined_Portal (1.16.1+)
        (12, 19), // Ruined_Portal_N (1.16.1+)
        (13, 23), // Ancient_City (1.19.2+)
        (14, 16), // Treasure (1.13+)
        (16, 16), // Desert_Well (1.13+ in cubiomes)
        (17, 21), // Geode (1.17+)
        (18, 3),  // Fortress (1.0+ all paths)
        (19, 19), // Bastion (1.16.1+; 1.18+ uses chunkGenerateRnd)
        (20, 12), // End_City (1.9+)
        (21, 16), // End_Gateway (1.13+)
        (22, 16), // End_Island (1.13+)
    ];

    let mc_pool: [i32; 8] = [2, 3, 10, 15, 19, 22, 25, 28];

    let mut rng_state: u64 = 0x0000_5e94_0d6f_a1e5;
    for _ in 0..STRUCTURE_POS_RECORDS {
        rng_state = lcg_step(rng_state);
        let (ty, min_mc) = types[(rng_state as usize) % types.len()];
        rng_state = lcg_step(rng_state);
        let valid_mcs: Vec<i32> = mc_pool.iter().copied().filter(|m| *m >= min_mc).collect();
        if valid_mcs.is_empty() {
            continue;
        }
        let mc = valid_mcs[(rng_state as usize) % valid_mcs.len()];
        // Feature only valid up to MC_1_12 (ord 15).
        let mc = if ty == 0 && mc > 15 { 15 } else { mc };
        rng_state = lcg_step(rng_state);
        let seed = rng_state;
        rng_state = lcg_step(rng_state);
        let reg_x = (rng_state as i32) % 256;
        rng_state = lcg_step(rng_state);
        let reg_z = (rng_state as i32) % 256;

        let mut pos_x: c_int = 0;
        let mut pos_z: c_int = 0;
        let valid = unsafe {
            ffi::cubiomes_call_get_structure_pos(
                ty,
                mc,
                seed,
                reg_x,
                reg_z,
                std::ptr::from_mut(&mut pos_x),
                std::ptr::from_mut(&mut pos_z),
            )
        };

        let rec = StructurePosRecord {
            structure_type: ty,
            mc,
            seed,
            reg_x,
            reg_z,
            pos_x,
            pos_z,
            valid,
            pad: 0,
        };
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

#[allow(clippy::many_single_char_names)]
fn write_generator_biome_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 46, GENERATOR_BIOME_RECORDS)?;

    // Random combinations of (mc, dim, scale) chosen at fixture-gen
    // time. Skip impossible matches (Nether < 1.16.1, End < 1.9).
    let mut rng_state: u64 = 0x0000_9e9e_a701_2025;
    for _ in 0..GENERATOR_BIOME_RECORDS {
        rng_state = lcg_step(rng_state);
        let seed = rng_state;
        rng_state = lcg_step(rng_state);
        // MC pool: a mix of Beta / 1.0 / 1.7 / 1.12 / 1.16 / 1.18 / 1.20 / 1.21.
        let mc_pool: [i32; 8] = [1, 3, 10, 15, 19, 22, 25, 28];
        let mc = mc_pool[(rng_state as usize) % mc_pool.len()];
        rng_state = lcg_step(rng_state);
        // Pick a compatible dim. cubiomes enum ordinals: B1_7=1,
        // 1.0=3, …, 1.7=10, 1.9=12, 1.12=15, 1.16.1=19, 1.18=22,
        // 1.20=25, 1.21 WD=28.
        let dim_choice = rng_state % 3;
        let dim: i32 = match dim_choice {
            1 if mc >= 19 => -1, // Nether — requires 1.16.1+
            2 if mc >= 12 => 1,  // End — requires 1.9+
            _ => 0,              // Overworld (and fallthrough)
        };
        rng_state = lcg_step(rng_state);
        // Pick a scale supported by the dim / mc combo:
        let scale = if dim == 0 && (10..=21).contains(&mc) {
            // Layered Overworld supports 1, 4, 16, 64, 256.
            let s: [i32; 5] = [1, 4, 16, 64, 256];
            s[(rng_state as usize) % s.len()]
        } else if dim == 0 {
            // Modern / Beta — scale 1 or 4 only (Beta has no Voronoi).
            if mc >= 22 {
                i32::from(rng_state.trailing_zeros() == 0) * 3 + 1
            } else {
                4
            }
        } else {
            // Nether / End — scale 1 or 4.
            i32::from(rng_state.trailing_zeros() == 0) * 3 + 1
        };
        rng_state = lcg_step(rng_state);
        let flags: u32 = u32::from(mc >= 6 && rng_state.trailing_zeros() >= 2);
        rng_state = lcg_step(rng_state);
        let x = (rng_state as i32) % 4096;
        rng_state = lcg_step(rng_state);
        let y = (rng_state as i32) % 256;
        rng_state = lcg_step(rng_state);
        let z = (rng_state as i32) % 4096;

        let biome_id =
            unsafe { ffi::cubiomes_call_get_biome_at(mc, flags, dim, seed, scale, x, y, z) };

        let rec = GeneratorBiomeRecord {
            mc: mc as u32,
            flags,
            dim,
            scale,
            seed,
            x,
            y,
            z,
            biome_id,
        };
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
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

/// `SurfaceNoise` record (kind = 40). Two outputs per record: the
/// plain `sampleSurfaceNoise` and the early-exit
/// `sampleSurfaceNoiseBetween`. `dim` ∈ {0 = Overworld, 1 = End}.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SurfaceNoiseRecord {
    pub dim: i32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub seed: u64,
    pub noise_min: f64,
    pub noise_max: f64,
    pub sample_bits: u64,
    pub between_bits: u64,
}

const SURFACE_NOISE_RECORDS: u64 = 1024;

fn write_surface_noise_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 40, SURFACE_NOISE_RECORDS)?;

    let mut rng_state: u64 = 0x5_face_0001_3001;
    for _ in 0..SURFACE_NOISE_RECORDS {
        rng_state = lcg_step(rng_state);
        let seed = rng_state;
        rng_state = lcg_step(rng_state);
        // 75% Overworld, 25% End — Overworld exercises oct_surf /
        // oct_depth init; End exercises the shorter octave path.
        let dim: i32 = i32::from(rng_state.trailing_zeros() >= 2);
        rng_state = lcg_step(rng_state);
        let x = (rng_state as i32) % 4096;
        rng_state = lcg_step(rng_state);
        let y = (rng_state as i32) % 256;
        rng_state = lcg_step(rng_state);
        let z = (rng_state as i32) % 4096;
        rng_state = lcg_step(rng_state);
        let noise_min = u64_to_double_signed(rng_state) * 8.0;
        rng_state = lcg_step(rng_state);
        let noise_max = noise_min + u64_to_double_signed(rng_state).abs() * 8.0 + 0.5;

        let sample = unsafe { ffi::cubiomes_call_sample_surface_noise(dim, seed, x, y, z) };
        let between = unsafe {
            ffi::cubiomes_call_sample_surface_noise_between(
                dim, seed, x, y, z, noise_min, noise_max,
            )
        };
        let rec = SurfaceNoiseRecord {
            dim,
            x,
            y,
            z,
            seed,
            noise_min,
            noise_max,
            sample_bits: sample.to_bits(),
            between_bits: between.to_bits(),
        };
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

/// `NetherNoise` parity record (kind = 41). Pairs each input with
/// both a single-cell `getNetherBiome` result (biome + `ndel` bit
/// pattern) and the digest of a small `mapNether2D` grid keyed at
/// `(x, z)`. The single-cell result exercises the `f32` distance
/// arithmetic precisely; the grid digest catches `fillRad3D` /
/// confidence-radius regressions.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct NetherRecord {
    pub seed: u64,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub w: u32,
    pub h: u32,
    pub single_biome: i32,
    pub single_ndel_bits: u32,
    pub grid_digest: u32,
    pub pad: [u32; 2],
}

const NETHER_RECORDS: u64 = 512;

#[allow(clippy::many_single_char_names)]
fn write_nether_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 41, NETHER_RECORDS)?;

    let mut rng_state: u64 = 0x9e7_4e7_770_001;
    for _ in 0..NETHER_RECORDS {
        rng_state = lcg_step(rng_state);
        let seed = rng_state;
        rng_state = lcg_step(rng_state);
        let x = (rng_state as i32) % 4096;
        rng_state = lcg_step(rng_state);
        let y = (rng_state as i32) % 128;
        rng_state = lcg_step(rng_state);
        let z = (rng_state as i32) % 4096;
        rng_state = lcg_step(rng_state);
        let w = ((rng_state & 0xf) as u32) + 4;
        let h = ((rng_state >> 8) & 0xf) as u32 + 4;

        let mut ndel: f32 = 0.0;
        let single_biome = unsafe {
            ffi::cubiomes_call_get_nether_biome(seed, x, y, z, std::ptr::from_mut(&mut ndel))
        };

        let mut out: Vec<i32> = vec![0; (w as usize) * (h as usize)];
        unsafe {
            ffi::cubiomes_call_map_nether_2d(seed, out.as_mut_ptr(), x, z, w as c_int, h as c_int);
        }
        let grid_digest = digest_i32_slice(&out);

        let rec = NetherRecord {
            seed,
            x,
            y,
            z,
            w,
            h,
            single_biome,
            single_ndel_bits: ndel.to_bits(),
            grid_digest,
            pad: [0; 2],
        };
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

/// `BiomeNoiseBeta::sample` parity record (kind = 45).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BiomeNoiseBetaRecord {
    pub seed: u64,
    pub t_bits: u64,
    pub h_bits: u64,
    pub x: i32,
    pub z: i32,
    pub biome_id: i32,
    pub pad: u32,
}

const BIOME_NOISE_BETA_RECORDS: u64 = 1024;

fn write_biome_noise_beta_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 45, BIOME_NOISE_BETA_RECORDS)?;

    let mut rng_state: u64 = 0x0000_b17a_9999_5555;
    for _ in 0..BIOME_NOISE_BETA_RECORDS {
        rng_state = lcg_step(rng_state);
        let seed = rng_state;
        rng_state = lcg_step(rng_state);
        let x = (rng_state as i32) % 4096;
        rng_state = lcg_step(rng_state);
        let z = (rng_state as i32) % 4096;

        let mut t: f64 = 0.0;
        let mut h: f64 = 0.0;
        let biome_id = unsafe {
            ffi::cubiomes_call_sample_biome_noise_beta(
                seed,
                x,
                z,
                std::ptr::from_mut(&mut t),
                std::ptr::from_mut(&mut h),
            )
        };
        let rec = BiomeNoiseBetaRecord {
            seed,
            x,
            z,
            biome_id,
            t_bits: t.to_bits(),
            h_bits: h.to_bits(),
            pad: 0,
        };
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

/// `sampleBiomeNoise` parity record (kind = 44). For each input
/// `(mc, seed, large, x, y, z)` carries the chosen biome id and the
/// six-axis `np[6]` tuple cubiomes computes.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BiomeNoiseRecord {
    pub mc: u32,
    pub large: u32,
    pub seed: u64,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub biome_id: i32,
    pub np: [i64; 6],
    pub pad: [u32; 2],
}

const BIOME_NOISE_RECORDS: u64 = 512;

#[allow(clippy::many_single_char_names)]
fn write_biome_noise_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 44, BIOME_NOISE_RECORDS)?;

    let mc_versions: [i32; 5] = [22, 23, 24, 25, 28];
    let mut rng_state: u64 = 0x0000_b10e_0011_5e1d;
    for _ in 0..BIOME_NOISE_RECORDS {
        rng_state = lcg_step(rng_state);
        let seed = rng_state;
        rng_state = lcg_step(rng_state);
        let mc = mc_versions[(rng_state as usize) % mc_versions.len()];
        rng_state = lcg_step(rng_state);
        let large = i32::from(rng_state.trailing_zeros() >= 2);
        rng_state = lcg_step(rng_state);
        let x = (rng_state as i32) % 4096;
        rng_state = lcg_step(rng_state);
        let y = (rng_state as i32) % 256;
        rng_state = lcg_step(rng_state);
        let z = (rng_state as i32) % 4096;

        let mut np = [0i64; 6];
        let biome_id = unsafe {
            ffi::cubiomes_call_sample_biome_noise(mc, seed, large, x, y, z, 0, np.as_mut_ptr())
        };
        let rec = BiomeNoiseRecord {
            mc: mc as u32,
            large: large as u32,
            seed,
            x,
            y,
            z,
            biome_id,
            np,
            pad: [0, 0],
        };
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

/// `climateToBiome` parity record (kind = 43). One 6-axis climate
/// tuple per record, paired with cubiomes' returned biome id.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ClimateRecord {
    pub mc: u32,
    pub biome_id: i32,
    pub np: [u64; 6],
}

const CLIMATE_RECORDS: u64 = 2048;

fn write_climate_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 43, CLIMATE_RECORDS)?;

    // 1.18 (22), 1.19.2 (23), 1.19.4 (24), 1.20.6 (25), 1.21 WD (28)
    let mc_versions: [i32; 5] = [22, 23, 24, 25, 28];

    let mut rng_state: u64 = 0x0000_c11e_ea7e_5000;
    for _ in 0..CLIMATE_RECORDS {
        rng_state = lcg_step(rng_state);
        let mc = mc_versions[(rng_state as usize) % mc_versions.len()];
        let mut np = [0u64; 6];
        for slot in &mut np {
            rng_state = lcg_step(rng_state);
            // Climate values are typically in [-20000, 20000] (10000 *
            // noise sample). Mix in occasional out-of-range values to
            // exercise the wrap-around arithmetic on both sides of
            // cubiomes' `(int64_t)a > 0` test.
            let signed = (rng_state as i64) % 25000;
            *slot = signed as u64;
        }
        let biome_id = unsafe { ffi::cubiomes_call_climate_to_biome(mc, np.as_ptr()) };
        let rec = ClimateRecord {
            mc: mc as u32,
            biome_id,
            np,
        };
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
}

/// `EndNoise` parity record (kind = 42). Per-record digests of
/// `mapEndBiome` (1:16 scale, raw heightmap dispatch) and `mapEnd`
/// (1:4 wrapper) over a small grid.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct EndRecord {
    pub mc: u32,
    pub w: u32,
    pub h: u32,
    pub pad0: u32,
    pub seed: u64,
    pub x: i32,
    pub z: i32,
    pub biome_digest: u32,
    pub end_digest: u32,
}

const END_RECORDS: u64 = 256;

#[allow(clippy::many_single_char_names)]
fn write_end_fixture(path: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, 42, END_RECORDS)?;

    // MC ordinals chosen to exercise the pre/post 1.14 outer-ring
    // fallback: 14 (1.13), 17 (1.14), 22 (1.18), 25 (1.20).
    let mc_versions: [i32; 4] = [16, 17, 22, 25];

    let mut rng_state: u64 = 0x0000_00e0_de0d_5000;
    for _ in 0..END_RECORDS {
        rng_state = lcg_step(rng_state);
        let seed = rng_state;
        rng_state = lcg_step(rng_state);
        let mc = mc_versions[(rng_state as usize) % mc_versions.len()];
        rng_state = lcg_step(rng_state);
        let x = (rng_state as i32) % 4096;
        rng_state = lcg_step(rng_state);
        let z = (rng_state as i32) % 4096;
        rng_state = lcg_step(rng_state);
        let w = ((rng_state & 0xf) as u32) + 4;
        let h = ((rng_state >> 8) & 0xf) as u32 + 4;

        let mut out_biome: Vec<i32> = vec![0; (w as usize) * (h as usize)];
        let mut out_end: Vec<i32> = vec![0; (w as usize) * (h as usize)];
        unsafe {
            ffi::cubiomes_call_map_end_biome(
                mc,
                seed,
                out_biome.as_mut_ptr(),
                x,
                z,
                w as c_int,
                h as c_int,
            );
            ffi::cubiomes_call_map_end(
                mc,
                seed,
                out_end.as_mut_ptr(),
                x,
                z,
                w as c_int,
                h as c_int,
            );
        }
        let biome_digest = digest_i32_slice(&out_biome);
        let end_digest = digest_i32_slice(&out_end);

        let rec = EndRecord {
            mc: mc as u32,
            w,
            h,
            pad0: 0,
            seed,
            x,
            z,
            biome_digest,
            end_digest,
        };
        file.write_all(bytemuck::bytes_of(&rec))?;
    }
    file.flush()
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
        pub fn cubiomes_call_sample_surface_noise(
            dim: c_int,
            seed: u64,
            x: c_int,
            y: c_int,
            z: c_int,
        ) -> f64;
        pub fn cubiomes_call_sample_surface_noise_between(
            dim: c_int,
            seed: u64,
            x: c_int,
            y: c_int,
            z: c_int,
            nmin: f64,
            nmax: f64,
        ) -> f64;
        pub fn cubiomes_call_map_nether_2d(
            seed: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_get_nether_biome(
            seed: u64,
            x: c_int,
            y: c_int,
            z: c_int,
            ndel: *mut f32,
        ) -> c_int;
        pub fn cubiomes_call_map_end_biome(
            mc: c_int,
            seed: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_map_end(
            mc: c_int,
            seed: u64,
            out: *mut c_int,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
        );
        pub fn cubiomes_call_climate_to_biome(mc: c_int, np: *const u64) -> c_int;
        pub fn cubiomes_call_sample_biome_noise(
            mc: c_int,
            seed: u64,
            large_biomes: c_int,
            x: c_int,
            y: c_int,
            z: c_int,
            sample_flags: c_int,
            np_out: *mut i64,
        ) -> c_int;
        pub fn cubiomes_call_sample_biome_noise_beta(
            seed: u64,
            x: c_int,
            z: c_int,
            t_out: *mut f64,
            h_out: *mut f64,
        ) -> c_int;
        pub fn cubiomes_call_get_biome_at(
            mc: c_int,
            flags: u32,
            dim: c_int,
            seed: u64,
            scale: c_int,
            x: c_int,
            y: c_int,
            z: c_int,
        ) -> c_int;
        pub fn cubiomes_call_get_structure_pos(
            structure_type: c_int,
            mc: c_int,
            seed: u64,
            reg_x: c_int,
            reg_z: c_int,
            pos_x: *mut c_int,
            pos_z: *mut c_int,
        ) -> c_int;
        pub fn cubiomes_call_is_slime_chunk(seed: u64, cx: c_int, cz: c_int) -> c_int;
        pub fn cubiomes_call_is_quad_base_feature_24_classic(
            structure_type: c_int,
            mc: c_int,
            seed: u64,
        ) -> f32;
        pub fn cubiomes_call_is_quad_base_feature_24(
            structure_type: c_int,
            mc: c_int,
            seed: u64,
            ax: c_int,
            ay: c_int,
            az: c_int,
        ) -> f32;
        pub fn cubiomes_call_get_quad_hut_cst(low20: u64) -> c_int;
        pub fn cubiomes_call_is_stronghold_biome(mc: c_int, id: c_int) -> c_int;
        pub fn cubiomes_call_biome_exists(mc: c_int, id: c_int) -> c_int;
        pub fn cubiomes_call_is_overworld(mc: c_int, id: c_int) -> c_int;
        pub fn cubiomes_call_init_first_stronghold(
            mc: c_int,
            seed: u64,
            px: *mut c_int,
            pz: *mut c_int,
        );
        pub fn cubiomes_call_nth_strongholds(
            mc: c_int,
            seed: u64,
            n_steps: c_int,
            out_xz: *mut c_int,
        );
        pub fn cubiomes_call_estimate_spawn(mc: c_int, seed: u64, px: *mut c_int, pz: *mut c_int);
        pub fn cubiomes_call_get_population_seed(mc: c_int, ws: u64, x: c_int, z: c_int) -> u64;
        pub fn cubiomes_call_get_end_islands(
            mc: c_int,
            seed: u64,
            chunk_x: c_int,
            chunk_z: c_int,
            out_xyzr: *mut c_int,
        ) -> c_int;
        pub fn cubiomes_call_map_end_island_height(
            mc: c_int,
            seed: u64,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
            scale: c_int,
            y: *mut f32,
        ) -> c_int;
        pub fn cubiomes_call_end_height_noise(
            mc: c_int,
            seed: u64,
            x: c_int,
            z: c_int,
            range: c_int,
        ) -> f32;
        pub fn cubiomes_call_map_end_surface_height(
            mc: c_int,
            seed: u64,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
            scale: c_int,
            ymin: c_int,
            y: *mut f32,
        ) -> c_int;
        pub fn cubiomes_call_is_end_chunk_empty(
            mc: c_int,
            seed: u64,
            chunk_x: c_int,
            chunk_z: c_int,
        ) -> c_int;
        pub fn cubiomes_call_get_biome_depth_and_scale(
            id: c_int,
            depth: *mut f64,
            scale: *mut f64,
            grass: *mut c_int,
        ) -> c_int;
        pub fn cubiomes_call_map_approx_height(
            mc: c_int,
            dim: c_int,
            seed: u64,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
            y: *mut f32,
            ids: *mut c_int,
        ) -> c_int;
        pub fn cubiomes_call_get_spawn(mc: c_int, seed: u64, px: *mut c_int, pz: *mut c_int);
        pub fn cubiomes_call_is_viable_feature_biome(
            mc: c_int,
            structure_type: c_int,
            biome_id: c_int,
        ) -> c_int;
        pub fn cubiomes_call_is_viable_structure_pos(
            mc: c_int,
            dim: c_int,
            structure_type: c_int,
            seed: u64,
            x: c_int,
            z: c_int,
            flags: u32,
        ) -> c_int;
        pub fn cubiomes_call_get_variant(
            structure_type: c_int,
            mc: c_int,
            seed: u64,
            x: c_int,
            z: c_int,
            biome_id: c_int,
            out: *mut c_int,
        ) -> c_int;
        pub fn cubiomes_call_get_fixed_end_gateways(mc: c_int, seed: u64, out_xz: *mut c_int);
        pub fn cubiomes_call_scan_for_quads(
            mc: c_int,
            sty: c_int,
            radius: c_int,
            s48: u64,
            low_bits: *const u64,
            low_bit_count: c_int,
            salt: u64,
            x: c_int,
            z: c_int,
            w: c_int,
            h: c_int,
            out_xz: *mut c_int,
            n: c_int,
        ) -> c_int;
        pub fn cubiomes_call_get_linked_gateway_pos(
            mc: c_int,
            seed: u64,
            src_x: c_int,
            src_z: c_int,
            out_x: *mut c_int,
            out_z: *mut c_int,
        );
        pub fn cubiomes_call_get_optimal_afk(
            px: *mut c_int,
            pz: *mut c_int,
            spcnt: *mut c_int,
            p0x: c_int,
            p0z: c_int,
            p1x: c_int,
            p1z: c_int,
            p2x: c_int,
            p2z: c_int,
            p3x: c_int,
            p3z: c_int,
            ax: c_int,
            ay: c_int,
            az: c_int,
        );
        pub fn cubiomes_call_get_mineshafts(
            mc: c_int,
            seed: u64,
            cx0: c_int,
            cz0: c_int,
            cx1: c_int,
            cz1: c_int,
            out_xz: *mut c_int,
            n_max: c_int,
            total: *mut c_int,
        ) -> c_int;
        pub fn cubiomes_call_gen_biomes(
            mc: c_int,
            flags: u32,
            dim: c_int,
            seed: u64,
            scale: c_int,
            x: c_int,
            z: c_int,
            sx: c_int,
            sz: c_int,
            y: c_int,
            sy: c_int,
            out: *mut c_int,
        ) -> c_int;
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
