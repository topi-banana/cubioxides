//! `BiomeNoise::re_seed_axis` parity vs cubiomes'
//! `setClimateParaSeed`. Each case initialises a `BiomeNoise` via
//! `setBiomeSeed(init_seed)`, then partially re-seeds the named
//! axis (or 3 depth-feeding axes for `NP_DEPTH`) with `para_seed`,
//! then samples the resulting `DoublePerlinNoise` at `(x, 0, z)`
//! and compares against the cubiomes-side f64 bits.

#![allow(
    clippy::missing_panics_doc,
    clippy::items_after_statements,
    clippy::many_single_char_names
)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use cubioxides::biomenoise::biome_noise::{
    BiomeNoise, NP_CONTINENTALNESS, NP_DEPTH, NP_EROSION, NP_HUMIDITY, NP_TEMPERATURE, NP_WEIRDNESS,
};
use cubioxides::mc_version::MCVersion;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 100;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("set_climate_para_seed.bin")
}

fn read_i32(b: &[u8], off: usize) -> i32 {
    i32::from_le_bytes(b[off..off + 4].try_into().unwrap())
}
fn read_u64(b: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(b[off..off + 8].try_into().unwrap())
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
        4 => NP_DEPTH,
        5 => NP_WEIRDNESS,
        _ => panic!("unsupported nptype {n}"),
    }
}

#[test]
fn set_climate_para_seed_matches_cubiomes() {
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
    const REC_LEN: usize = 4 + 8 + 8 + 4 + 4 + 4 + 4 + 8;

    for i in 0..count as usize {
        let r = &body[i * REC_LEN..(i + 1) * REC_LEN];
        let mc = mc_from_ord(read_i32(r, 0));
        let init_seed = read_u64(r, 4);
        let para_seed = read_u64(r, 12);
        let large = read_i32(r, 20) != 0;
        let nptype = nptype_from_i32(read_i32(r, 24));
        let x = read_i32(r, 28);
        let z = read_i32(r, 32);
        let expected_bits = read_u64(r, 36);
        let expected = f64::from_bits(expected_bits);

        let mut bn = BiomeNoise::new(mc, init_seed, large);
        bn.re_seed_axis(para_seed, large, nptype);
        let sample_axis = if nptype == NP_DEPTH {
            NP_CONTINENTALNESS
        } else {
            nptype
        };
        let got = bn.climate[sample_axis].sample(f64::from(x), 0.0, f64::from(z));
        assert_eq!(
            got.to_bits(),
            expected.to_bits(),
            "case {i}: mc={mc:?} nptype={nptype} (x,z)=({x},{z}) \
             got={got} ({:#x}) expected={expected} ({:#x})",
            got.to_bits(),
            expected.to_bits(),
        );
    }
}
