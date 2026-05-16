//! `get_para_descent` parity vs cubiomes' `getParaDescent`.
//! Tests the no-callback path; the result is compared bit-exactly
//! via `f64::to_bits()`.

#![allow(
    clippy::missing_panics_doc,
    clippy::items_after_statements,
    clippy::many_single_char_names
)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use cubioxides::finder::para_descent::get_para_descent;
use cubioxides::generator::Generator;
use cubioxides::mc_version::{Dimension, MCVersion};

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 88;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("para_descent.bin")
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

#[test]
fn para_descent_matches_cubiomes() {
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
    // 4(mc) + 8(seed) + 4(npara) + 7*4(x,z,w,h,i0,j0,maxrad) + 4(maxiter)
    // + 8(factor) + 8(alpha) + 8(result) = 4+8+4+28+4+8+8+8 = 72
    const REC_LEN: usize = 4 + 8 + 4 + 4 * 7 + 4 + 8 + 8 + 8;

    for i in 0..count as usize {
        let r = &body[i * REC_LEN..(i + 1) * REC_LEN];
        let mc = mc_from_ord(read_i32(r, 0));
        let seed = read_u64(r, 4);
        let npara = read_i32(r, 12) as usize;
        let x = read_i32(r, 16);
        let z = read_i32(r, 20);
        let w = read_i32(r, 24);
        let h = read_i32(r, 28);
        let i0 = read_i32(r, 32);
        let j0 = read_i32(r, 36);
        let maxrad = read_i32(r, 40);
        let maxiter = read_i32(r, 44);
        let factor = read_f64(r, 48);
        let alpha = read_f64(r, 56);
        let expected_bits = read_u64(r, 64);
        let expected = f64::from_bits(expected_bits);

        let mut g = Generator::new(mc, 0);
        g.apply_seed(Dimension::Overworld, seed);
        let bn = g
            .biome_noise
            .as_ref()
            .expect("BiomeNoise must be set for 1.18+ overworld");
        let para = &bn.climate[npara];
        let got = get_para_descent::<fn(i32, i32, f64) -> bool>(
            para, factor, x, z, w, h, i0, j0, maxrad, maxiter, alpha, None,
        );
        assert_eq!(
            got.to_bits(),
            expected_bits,
            "case {i} ({mc:?}, seed={seed:#x}, npara={npara}, area=({x},{z})+({w},{h}), start=({i0},{j0})): \
             rust {got} ({:#x}) vs cubiomes {expected} ({expected_bits:#x})",
            got.to_bits(),
        );
    }
}
