//! `Generator::min_cache_size` parity vs cubiomes'
//! `getMinCacheSize`. Covers Beta + Layered + Modern + Nether/End
//! voronoi paths.

#![allow(
    clippy::missing_panics_doc,
    clippy::many_single_char_names,
    clippy::items_after_statements
)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use cubioxides::generator::Generator;
use cubioxides::mc_version::{Dimension, MCVersion};

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 101;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("min_cache_size.bin")
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
        1 => MCVersion::B1_7,
        19 => MCVersion::V1_16_1,
        22 => MCVersion::V1_18,
        _ => panic!("unsupported mc ord {o}"),
    }
}

fn dim_from_i32(d: i32) -> Dimension {
    match d {
        0 => Dimension::Overworld,
        1 => Dimension::Nether,
        2 => Dimension::End,
        _ => panic!("unsupported dim {d}"),
    }
}

#[test]
fn min_cache_size_matches_cubiomes() {
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
    const REC_LEN: usize = 4 + 4 + 8 + 4 + 4 + 4 + 4 + 4 + 8;

    for i in 0..count as usize {
        let r = &body[i * REC_LEN..(i + 1) * REC_LEN];
        let mc = mc_from_ord(read_i32(r, 0));
        let dim = dim_from_i32(read_i32(r, 4));
        let seed = read_u64(r, 8);
        let flags = read_u32(r, 16);
        let scale = read_i32(r, 20);
        let sx = read_i32(r, 24);
        let sy = read_i32(r, 28);
        let sz = read_i32(r, 32);
        let expected = read_u64(r, 36) as usize;

        let mut g = Generator::new(mc, flags);
        g.apply_seed(dim, seed);
        let got = g.min_cache_size(scale, sx as u32, sy as u32, sz as u32);
        assert_eq!(
            got, expected,
            "case {i}: mc={mc:?} dim={dim:?} scale={scale} \
             sx={sx} sy={sy} sz={sz} → got {got}, expected {expected}",
        );
    }
}
