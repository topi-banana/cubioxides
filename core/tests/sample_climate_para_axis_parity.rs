//! `BiomeNoise::sample_climate_para_axis` parity vs cubiomes'
//! `sampleClimatePara` for non-depth axes. Verifies both the raw
//! f64 sample (via `to_bits()`) and the quantised i64 side effect.

#![allow(
    clippy::missing_panics_doc,
    clippy::many_single_char_names,
    clippy::items_after_statements
)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use cubioxides::biomenoise::biome_noise::{
    BiomeNoise, NP_CONTINENTALNESS, NP_EROSION, NP_HUMIDITY, NP_MAX, NP_TEMPERATURE, NP_WEIRDNESS,
};
use cubioxides::mc_version::MCVersion;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 103;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("sample_climate_para_axis.bin")
}

fn read_i32(b: &[u8], off: usize) -> i32 {
    i32::from_le_bytes(b[off..off + 4].try_into().unwrap())
}
fn read_u64(b: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(b[off..off + 8].try_into().unwrap())
}
fn read_i64(b: &[u8], off: usize) -> i64 {
    i64::from_le_bytes(b[off..off + 8].try_into().unwrap())
}

fn mc_from_ord(o: i32) -> MCVersion {
    match o {
        22 => MCVersion::V1_18,
        28 => MCVersion::V1_21,
        _ => panic!("unsupported mc ord {o}"),
    }
}

fn nptype_from_i32(n: i32) -> usize {
    match n {
        0 => NP_TEMPERATURE,
        1 => NP_HUMIDITY,
        2 => NP_CONTINENTALNESS,
        3 => NP_EROSION,
        5 => NP_WEIRDNESS,
        _ => panic!("unsupported nptype {n} (NP_DEPTH not covered by this test)"),
    }
}

#[test]
fn sample_climate_para_axis_matches_cubiomes() {
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
    const REC_LEN: usize = 4 + 8 + 4 + 4 + 4 + 4 + 8 + 8;

    for i in 0..count as usize {
        let r = &body[i * REC_LEN..(i + 1) * REC_LEN];
        let mc = mc_from_ord(read_i32(r, 0));
        let seed = read_u64(r, 4);
        let large = read_i32(r, 12) != 0;
        let nptype = nptype_from_i32(read_i32(r, 16));
        let x = read_i32(r, 20);
        let z = read_i32(r, 24);
        let expected_bits = read_u64(r, 28);
        let expected_q = read_i64(r, 36);
        let expected = f64::from_bits(expected_bits);

        let bn = BiomeNoise::new(mc, seed, large);
        let mut np: [i64; NP_MAX] = [0; NP_MAX];
        let got = bn.sample_climate_para_axis(nptype, f64::from(x), f64::from(z), Some(&mut np));
        assert_eq!(
            got.to_bits(),
            expected.to_bits(),
            "case {i}: mc={mc:?} nptype={nptype} (x,z)=({x},{z}) \
             got={got} ({:#x}) expected={expected} ({:#x})",
            got.to_bits(),
            expected.to_bits(),
        );
        assert_eq!(
            np[nptype], expected_q,
            "case {i}: quantised got={} expected={expected_q}",
            np[nptype],
        );
    }
}
