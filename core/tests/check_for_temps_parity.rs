//! `check_for_temps` parity vs cubiomes' `checkForTemps`.

#![allow(
    clippy::missing_panics_doc,
    clippy::items_after_statements,
    clippy::needless_range_loop,
    clippy::many_single_char_names
)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use cubioxides::finder::check_for_temps::{TC_LEN, check_for_temps};
use cubioxides::layer::stack::{LayerStack, setup_layer_stack};
use cubioxides::mc_version::MCVersion;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 90;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("check_for_temps.bin")
}

fn read_i32(b: &[u8], off: usize) -> i32 {
    i32::from_le_bytes(b[off..off + 4].try_into().unwrap())
}
fn read_u64(b: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(b[off..off + 8].try_into().unwrap())
}

fn mc_from_ord(o: i32) -> MCVersion {
    match o {
        10 => MCVersion::V1_7,
        17 => MCVersion::V1_14,
        20 => MCVersion::V1_17,
        _ => panic!("unsupported mc ord {o}"),
    }
}

#[test]
fn check_for_temps_matches_cubiomes() {
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
    // 4 (mc) + 8 (seed) + 4 (x) + 4 (z) + 4 (w) + 4 (h) + 9*4 (tc) + 4 (result) = 68
    const REC_LEN: usize = 4 + 8 + 4 * 4 + 9 * 4 + 4;

    for i in 0..count as usize {
        let r = &body[i * REC_LEN..(i + 1) * REC_LEN];
        let mc = mc_from_ord(read_i32(r, 0));
        let seed = read_u64(r, 4);
        let x = read_i32(r, 12);
        let z = read_i32(r, 16);
        let w = read_i32(r, 20);
        let h = read_i32(r, 24);
        let mut tc = [0_i32; TC_LEN];
        for j in 0..TC_LEN {
            tc[j] = read_i32(r, 28 + j * 4);
        }
        let expected = read_i32(r, 28 + TC_LEN * 4) != 0;

        let mut stack = LayerStack::default();
        setup_layer_stack(&mut stack, mc, false);
        let got = check_for_temps(&mut stack, seed, x, z, w, h, &tc);
        assert_eq!(
            got, expected,
            "case {i} ({mc:?}, seed={seed:#x}, area=({x},{z})+({w},{h})): tc={tc:?} — rust {got} vs cubiomes {expected}",
        );
    }
}
