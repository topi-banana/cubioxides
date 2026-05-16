//! `check_for_biomes` Layered (MC 1.7-1.17) path parity vs cubiomes'
//! `checkForBiomes`. The Rust port uses a simple "generate + bitmask"
//! approach; cubiomes uses an optimised swap-map chain that can
//! early-exit. Both produce the same Pass/Fail answer for filters
//! that only have one of {required, excluded, matchany} — the
//! cases this fixture exercises.

#![allow(
    clippy::missing_panics_doc,
    clippy::items_after_statements,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use cubioxides::finder::biome_filter::setup_biome_filter;
use cubioxides::finder::check_for_biomes::{CheckForBiomesResult, check_for_biomes};
use cubioxides::generator::{Generator, Range};
use cubioxides::mc_version::{Dimension, MCVersion};

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 106;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("check_for_biomes_layered.bin")
}

fn read_i32(b: &[u8], off: usize) -> i32 {
    i32::from_le_bytes(b[off..off + 4].try_into().unwrap())
}
fn read_u32(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(b[off..off + 4].try_into().unwrap())
}
fn read_u64(b: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(b[off..off + 8].try_into().unwrap())
}

fn mc_from_ord(o: i32) -> MCVersion {
    match o {
        12 => MCVersion::V1_7,
        16 => MCVersion::V1_13,
        17 => MCVersion::V1_14,
        19 => MCVersion::V1_16_1,
        21 => MCVersion::V1_17,
        _ => panic!("unsupported mc ord {o}"),
    }
}

#[test]
fn check_for_biomes_layered_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let header = &bytes[..32];
    assert_eq!(&header[..4], &MAGIC);
    assert_eq!(
        u16::from_le_bytes(header[4..6].try_into().unwrap()),
        FORMAT_VERSION
    );
    assert_eq!(u16::from_le_bytes(header[6..8].try_into().unwrap()), KIND);
    let count = u64::from_le_bytes(header[8..16].try_into().unwrap());
    let body = &bytes[32..];
    // 4(mc)+8(seed)+4(dim)+4(scale)+6*4(rx..sz)+4(flags)+3*4(lens)+3*8*4(arrays)+4(result)
    const REC_LEN: usize = 4 + 8 + 4 + 4 + 4 * 6 + 4 + 4 * 3 + 4 * 8 * 3 + 4;

    for i in 0..count as usize {
        let r = &body[i * REC_LEN..(i + 1) * REC_LEN];
        let mc = mc_from_ord(read_i32(r, 0));
        let seed = read_u64(r, 4);
        let _dim_ord = read_i32(r, 12); // always 0 (Overworld)
        let scale = read_i32(r, 16);
        let rx = read_i32(r, 20);
        let ry = read_i32(r, 24);
        let rz = read_i32(r, 28);
        let sx = read_i32(r, 32);
        let sy = read_i32(r, 36);
        let sz = read_i32(r, 40);
        let flags = read_u32(r, 44);
        let req_len = read_i32(r, 48) as usize;
        let exc_len = read_i32(r, 52) as usize;
        let any_len = read_i32(r, 56) as usize;
        let mut req = [0_i32; 8];
        let mut exc = [0_i32; 8];
        let mut any = [0_i32; 8];
        for j in 0..8 {
            req[j] = read_i32(r, 60 + j * 4);
            exc[j] = read_i32(r, 92 + j * 4);
            any[j] = read_i32(r, 124 + j * 4);
        }
        let expected_result = read_i32(r, 156);

        let filter =
            setup_biome_filter(mc, flags, &req[..req_len], &exc[..exc_len], &any[..any_len])
                .expect("filter");
        let mut g = Generator::new(mc, flags);
        let range = Range {
            scale,
            x: rx,
            z: rz,
            sx: sx as u32,
            sz: sz as u32,
            y: ry,
            sy: sy as u32,
        };
        let res = check_for_biomes(&mut g, range, Dimension::Overworld, seed, &filter);
        // Cubiomes returns 0/1/2; map 1 and 2 both to Pass. Our
        // simple-path implementation only ever returns Pass or Fail.
        let cubiomes_pass = expected_result != 0;
        let rust_pass = matches!(res, CheckForBiomesResult::Pass);
        assert_eq!(
            rust_pass, cubiomes_pass,
            "case {i} ({mc:?} seed={seed:#x} scale={scale} req={req:?}[..{req_len}] \
             exc={exc:?}[..{exc_len}] any={any:?}[..{any_len}]): \
             rust pass={rust_pass} vs cubiomes result={expected_result}",
        );
    }
}
