//! `FORCE_OCEAN_VARIANTS` parity test. Verifies the Rust port's
//! `Generator::new(mc, FORCE_OCEAN_VARIANTS)` + `gen_biomes` output
//! matches cubiomes' `setupGenerator(g, mc, FORCE_OCEAN_VARIANTS)`
//! biome-by-biome. Exercises layered MC 1.13–1.17 at scales 16, 64,
//! and 256 (the only scales where cubiomes injects the
//! `mapOceanMixMod` wrapper).

#![allow(
    clippy::missing_panics_doc,
    clippy::items_after_statements,
    clippy::many_single_char_names,
    clippy::needless_range_loop
)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use cubioxides::biome::Biome;
use cubioxides::generator::{FORCE_OCEAN_VARIANTS, Generator, Range};
use cubioxides::mc_version::{Dimension, MCVersion};

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 105;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("force_ocean_variants.bin")
}

fn read_i32(b: &[u8], off: usize) -> i32 {
    i32::from_le_bytes(b[off..off + 4].try_into().unwrap())
}
fn read_u64(b: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(b[off..off + 8].try_into().unwrap())
}

fn mc_from_ord(o: i32) -> MCVersion {
    match o {
        16 => MCVersion::V1_13,
        17 => MCVersion::V1_14,
        19 => MCVersion::V1_16_1,
        21 => MCVersion::V1_17,
        _ => panic!("unsupported mc ord {o}"),
    }
}

#[test]
fn force_ocean_variants_matches_cubiomes() {
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
    let mut offset = 32_usize;

    for i in 0..count as usize {
        let mc = mc_from_ord(read_i32(&bytes, offset));
        let seed = read_u64(&bytes, offset + 4);
        let scale = read_i32(&bytes, offset + 12);
        let rx = read_i32(&bytes, offset + 16);
        let ry = read_i32(&bytes, offset + 20);
        let rz = read_i32(&bytes, offset + 24);
        let sx = read_i32(&bytes, offset + 28);
        let sy = read_i32(&bytes, offset + 32);
        let sz = read_i32(&bytes, offset + 36);
        let err = read_i32(&bytes, offset + 40);
        offset += 44;
        let n = (sx * sy * sz) as usize;
        let mut expected = vec![0_i32; n];
        for k in 0..n {
            expected[k] = read_i32(&bytes, offset + k * 4);
        }
        offset += n * 4;
        assert_eq!(err, 0, "case {i}: cubiomes err {err}");

        let mut g = Generator::new(mc, FORCE_OCEAN_VARIANTS);
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
        let mut cache = vec![Biome(0); n];
        g.gen_biomes(&mut cache, range);
        let got: Vec<i32> = cache[..n].iter().map(|b| b.0).collect();
        assert_eq!(
            got, expected,
            "case {i} (mc={mc:?}, scale={scale}, seed={seed:#x}, area=({rx},{ry},{rz})+({sx},{sy},{sz}))",
        );
    }
}
