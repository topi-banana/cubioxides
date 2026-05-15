//! Parity tests: cubioxides RNG primitives against cubiomes via fixtures.
//!
//! Loads the binary records produced by `fixtures-gen regenerate-all` and
//! checks every Rust function call against the exact bytes cubiomes
//! produces. `f32` / `f64` outputs are compared by `to_bits`, so the
//! check is bit-exact regardless of platform float rounding.

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::rng::{
    JavaRng, Xoroshiro, get_chunk_seed, get_layer_salt, get_start_salt, get_start_seed,
    mc_first_int, mc_step_seed, mul_inv,
};

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
struct JavaRecord {
    seed: u64,
    next_32: i32,
    next_int_24: i32,
    next_long: u64,
    next_float_bits: u32,
    pad0: u32,
    next_double_bits: u64,
    skip_42_raw: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct XoroshiroRecord {
    seed: u64,
    state_lo: u64,
    state_hi: u64,
    next_long: u64,
    next_long_j: u64,
    next_int_24: i32,
    next_int_j_24: i32,
    next_double_bits: u64,
    next_float_bits: u32,
    pad0: u32,
    skip_42_lo: u64,
    skip_42_hi: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct McSeedRecord {
    seed: u64,
    salt: u64,
    step_seed: u64,
    first_int_24: i32,
    pad0: u32,
    layer_salt: u64,
    start_salt: u64,
    start_seed: u64,
    chunk_seed: u64,
    mul_inv: u64,
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("fixtures")
        .join("rng")
}

fn load_fixture<R: Pod>(name: &str, expected_kind: u16) -> Vec<R> {
    let path = fixture_dir().join(name);
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    assert!(
        bytes.len() >= std::mem::size_of::<Header>(),
        "fixture {} too short",
        path.display()
    );
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

#[test]
fn java_rng_matches_cubiomes() {
    let records: Vec<JavaRecord> = load_fixture("java.bin", 1);
    assert!(!records.is_empty());

    for (i, rec) in records.iter().enumerate() {
        let mut rng = JavaRng::new(rec.seed);
        assert_eq!(rng.next(32), rec.next_32, "next(32) at record {i}");

        let mut rng = JavaRng::new(rec.seed);
        assert_eq!(
            rng.next_int_24(),
            rec.next_int_24,
            "next_int_24 at record {i}"
        );

        let mut rng = JavaRng::new(rec.seed);
        assert_eq!(rng.next_long(), rec.next_long, "next_long at record {i}");

        let mut rng = JavaRng::new(rec.seed);
        assert_eq!(
            rng.next_float().to_bits(),
            rec.next_float_bits,
            "next_float at record {i}"
        );

        let mut rng = JavaRng::new(rec.seed);
        assert_eq!(
            rng.next_double().to_bits(),
            rec.next_double_bits,
            "next_double at record {i}"
        );

        let mut rng = JavaRng::new(rec.seed);
        rng.skip_n(42);
        assert_eq!(
            rng.raw_seed(),
            rec.skip_42_raw,
            "skip_n(42) raw seed at record {i}"
        );
    }
}

#[test]
fn xoroshiro_matches_cubiomes() {
    let records: Vec<XoroshiroRecord> = load_fixture("xoroshiro.bin", 2);
    assert!(!records.is_empty());

    for (i, rec) in records.iter().enumerate() {
        let xr = Xoroshiro::new(rec.seed);
        assert_eq!(
            (xr.lo, xr.hi),
            (rec.state_lo, rec.state_hi),
            "state after set_seed at record {i}"
        );

        let mut xr = Xoroshiro::new(rec.seed);
        assert_eq!(xr.next_long(), rec.next_long, "next_long at record {i}");

        let mut xr = Xoroshiro::new(rec.seed);
        assert_eq!(
            xr.next_long_j(),
            rec.next_long_j,
            "next_long_j at record {i}"
        );

        let mut xr = Xoroshiro::new(rec.seed);
        assert_eq!(
            xr.next_int(24),
            rec.next_int_24,
            "next_int(24) at record {i}"
        );

        let mut xr = Xoroshiro::new(rec.seed);
        assert_eq!(
            xr.next_int_j(24),
            rec.next_int_j_24,
            "next_int_j(24) at record {i}"
        );

        let mut xr = Xoroshiro::new(rec.seed);
        assert_eq!(
            xr.next_double().to_bits(),
            rec.next_double_bits,
            "next_double at record {i}"
        );

        let mut xr = Xoroshiro::new(rec.seed);
        assert_eq!(
            xr.next_float().to_bits(),
            rec.next_float_bits,
            "next_float at record {i}"
        );

        let mut xr = Xoroshiro::new(rec.seed);
        xr.skip_n(42);
        assert_eq!(
            (xr.lo, xr.hi),
            (rec.skip_42_lo, rec.skip_42_hi),
            "state after skip_n(42) at record {i}"
        );
    }
}

#[test]
fn mc_seed_matches_cubiomes() {
    let records: Vec<McSeedRecord> = load_fixture("mc_seed.bin", 3);
    assert!(!records.is_empty());

    for (i, rec) in records.iter().enumerate() {
        assert_eq!(
            mc_step_seed(rec.seed, rec.salt),
            rec.step_seed,
            "mc_step_seed at record {i}"
        );
        assert_eq!(
            mc_first_int(rec.seed, 24),
            rec.first_int_24,
            "mc_first_int at record {i}"
        );
        assert_eq!(
            get_layer_salt(rec.salt),
            rec.layer_salt,
            "get_layer_salt at record {i}"
        );
        assert_eq!(
            get_start_salt(rec.seed, rec.salt),
            rec.start_salt,
            "get_start_salt at record {i}"
        );
        assert_eq!(
            get_start_seed(rec.seed, rec.salt),
            rec.start_seed,
            "get_start_seed at record {i}"
        );
        let x = (rec.salt as i32).wrapping_neg();
        let z = rec.salt as i32;
        assert_eq!(
            get_chunk_seed(rec.seed, x, z),
            rec.chunk_seed,
            "get_chunk_seed at record {i}"
        );
        let mi_x = rec.seed | 1;
        let mi_m = (rec.salt | 7).max(2);
        assert_eq!(mul_inv(mi_x, mi_m), rec.mul_inv, "mul_inv at record {i}");
    }
}
