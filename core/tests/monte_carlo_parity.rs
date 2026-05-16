//! `monte_carlo_biomes` parity vs cubiomes' `monteCarloBiomes`.
//! Both implementations use the same eval predicate ("biome at
//! (scale, x, y, z) equals target_id") so the run is fully
//! deterministic given the same RNG state.

#![allow(
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    clippy::items_after_statements
)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use cubioxides::finder::monte_carlo::{MonteCarloEval, monte_carlo_biomes};
use cubioxides::generator::{Generator, Range};
use cubioxides::mc_version::{Dimension, MCVersion};
use cubioxides::rng::JavaRng;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 98;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("monte_carlo.bin")
}

fn read_i32(b: &[u8], off: usize) -> i32 {
    i32::from_le_bytes(b[off..off + 4].try_into().unwrap())
}
fn read_u64(b: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(b[off..off + 8].try_into().unwrap())
}
fn read_f64(b: &[u8], off: usize) -> f64 {
    f64::from_le_bytes(b[off..off + 8].try_into().unwrap())
}

fn mc_from_ord(o: i32) -> MCVersion {
    match o {
        22 => MCVersion::V1_18,
        28 => MCVersion::V1_21,
        _ => panic!("unsupported mc ord {o}"),
    }
}

fn dim_from_ord(d: i32) -> Dimension {
    match d {
        -1 => Dimension::Nether,
        0 => Dimension::Overworld,
        1 => Dimension::End,
        _ => panic!("unsupported dim {d}"),
    }
}

#[test]
fn monte_carlo_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    // Header is 32 bytes.
    let header = &bytes[..32];
    assert_eq!(&header[..4], &MAGIC);
    assert_eq!(
        u16::from_le_bytes(header[4..6].try_into().unwrap()),
        FORMAT_VERSION
    );
    assert_eq!(u16::from_le_bytes(header[6..8].try_into().unwrap()), KIND);
    let count = u64::from_le_bytes(header[8..16].try_into().unwrap());
    let body = &bytes[32..];

    // Body per record: 100 bytes.
    // Layout (offsets): mc 0, dim 4, seed 8, scale 16, rx 20, ry 24,
    // rz 28, sx 32, sy 36, sz 40, pad 44, rng_init 48, coverage 56,
    // confidence 64, target 72, result 76, samples 80, successes 84,
    // rng_after 88. Tail-pad to 100? Actually 88+8 = 96. No tail pad
    // needed but write_all wrote nothing extra, so REC_LEN = 96.
    const REC_LEN: usize = 96;
    for i in 0..count as usize {
        let r = &body[i * REC_LEN..(i + 1) * REC_LEN];
        let mc = mc_from_ord(read_i32(r, 0));
        let dim = dim_from_ord(read_i32(r, 4));
        let seed = read_u64(r, 8);
        let scale = read_i32(r, 16);
        let rx = read_i32(r, 20);
        let ry = read_i32(r, 24);
        let rz = read_i32(r, 28);
        let sx = read_i32(r, 32);
        let sy = read_i32(r, 36);
        let sz = read_i32(r, 40);
        // pad at offset 44..48
        let rng_initial = read_u64(r, 48);
        let coverage = read_f64(r, 56);
        let confidence = read_f64(r, 64);
        let target_id = read_i32(r, 72);
        let expected_result = read_i32(r, 76);
        let expected_samples = read_i32(r, 80);
        let expected_successes = read_i32(r, 84);
        // skip 4 bytes pad at offset 88..92? Actually field-after layout:
        // 88..96 = rng_after? Let me re-examine the writer.
        // Writer writes: result, samples, successes, then directly rng_after.
        // So offsets: 76 (result), 80 (samples), 84 (successes), 88 (rng_after).
        let expected_rng_after = read_u64(r, 88);
        // Pad to 116 from 96 — uh, that's only 96 bytes. Let me recount.
        // 0: mc 4, dim 4, seed 8 → 16. scale..sz = 10 i32 = 40 → 16+40 = 56? No.
        // scale@16, rx@20, ry@24, rz@28, sx@32, sy@36, sz@40, pad@44, rng_init@48.
        // coverage@56, confidence@64, target@72, result@76, samples@80,
        // successes@84, rng_after@88. Total = 88+8 = 96 bytes. Not 116.

        let mut g = Generator::new(mc, 0);
        g.apply_seed(dim, seed);
        let range = Range {
            scale,
            x: rx,
            z: rz,
            sx: sx as u32,
            sz: sz as u32,
            y: ry,
            sy: sy as u32,
        };
        let mut rng = JavaRng::from_raw(rng_initial);
        let mut samples_count = 0_i32;
        let mut successes_count = 0_i32;
        let result = monte_carlo_biomes(
            &g,
            range,
            &mut rng,
            coverage,
            confidence,
            |g, scale, x, y, z| {
                samples_count += 1;
                let id = g.biome_at(scale, x, y, z).0;
                if id < 0 {
                    MonteCarloEval::Skip
                } else if id == target_id {
                    successes_count += 1;
                    MonteCarloEval::Success
                } else {
                    MonteCarloEval::Fail
                }
            },
        );
        let got_result = i32::from(result);
        let rng_after = rng.raw_seed();
        assert_eq!(
            got_result, expected_result,
            "case {i}: result mismatch — rust {got_result} vs cubiomes {expected_result}",
        );
        assert_eq!(
            samples_count, expected_samples,
            "case {i}: samples mismatch — rust {samples_count} vs cubiomes {expected_samples}",
        );
        assert_eq!(
            successes_count, expected_successes,
            "case {i}: successes mismatch — rust {successes_count} vs cubiomes {expected_successes}",
        );
        assert_eq!(
            rng_after, expected_rng_after,
            "case {i}: post-RNG mismatch — rust {rng_after:#x} vs cubiomes {expected_rng_after:#x}",
        );
    }
}
