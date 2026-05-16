//! `get_biome_centers` parity (1.18+) vs cubiomes' `getBiomeCenters`.

#![allow(
    clippy::missing_panics_doc,
    clippy::items_after_statements,
    clippy::many_single_char_names,
    clippy::needless_range_loop
)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use cubioxides::finder::Pos;
use cubioxides::finder::biome_centers::get_biome_centers;
use cubioxides::generator::{Generator, Range};
use cubioxides::mc_version::{Dimension, MCVersion};

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 84;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("biome_centers.bin")
}

fn read_i32(b: &[u8], off: usize) -> i32 {
    i32::from_le_bytes(b[off..off + 4].try_into().unwrap())
}
fn read_u64(b: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(b[off..off + 8].try_into().unwrap())
}

fn mc_from_ord(o: i32) -> MCVersion {
    match o {
        17 => MCVersion::V1_14,
        19 => MCVersion::V1_16_1,
        22 => MCVersion::V1_18,
        28 => MCVersion::V1_21,
        _ => panic!("unsupported mc ord {o}"),
    }
}

#[test]
fn biome_centers_matches_cubiomes() {
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
    const MAX_N: usize = 32;
    // mc + 12 other i32s + 8 seed + 32*2 pos + 32 sizes
    // = 13*4 + 8 + 256 + 128 = 444
    const REC_LEN: usize = 4 * 13 + 8 + MAX_N * 2 * 4 + MAX_N * 4;

    for i in 0..count as usize {
        let r = &body[i * REC_LEN..(i + 1) * REC_LEN];
        let mc = mc_from_ord(read_i32(r, 0));
        let seed = read_u64(r, 4);
        let scale = read_i32(r, 12);
        let rx = read_i32(r, 16);
        let ry = read_i32(r, 20);
        let rz = read_i32(r, 24);
        let sx = read_i32(r, 28);
        let sy = read_i32(r, 32);
        let sz = read_i32(r, 36);
        let match_id = read_i32(r, 40);
        let minsiz = read_i32(r, 44);
        let tol = read_i32(r, 48);
        let nmax = read_i32(r, 52) as usize;
        let n = read_i32(r, 56) as usize;
        let pos_off = 60;
        let sizes_off = pos_off + MAX_N * 2 * 4;
        let mut expected_pos = Vec::with_capacity(n);
        let mut expected_sizes = Vec::with_capacity(n);
        for k in 0..n {
            let x = read_i32(r, pos_off + k * 8);
            let z = read_i32(r, pos_off + k * 8 + 4);
            expected_pos.push(Pos { x, z });
            expected_sizes.push(read_i32(r, sizes_off + k * 4));
        }

        let mut g = Generator::new(mc, 0);
        g.apply_seed(Dimension::Overworld, seed);
        let range = Range {
            scale,
            x: rx,
            z: rz,
            sx: sx as u32,
            sz: sz as u32,
            y: ry,
            sy: sy as u32,
        };
        let got =
            get_biome_centers(&mut g, range, match_id, minsiz, tol, nmax).expect("1.18+ supported");
        assert_eq!(
            got.pos.len(),
            n,
            "case {i} ({mc:?} seed={seed:#x} match={match_id} minsiz={minsiz} tol={tol}): \
             centre count rust={} vs cubiomes={n}",
            got.pos.len(),
        );
        assert_eq!(got.pos, expected_pos, "case {i}: pos mismatch");
        assert_eq!(got.sizes, expected_sizes, "case {i}: sizes mismatch");
    }
}
