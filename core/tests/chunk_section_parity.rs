//! `BiomeNoise::gen_chunk_section` parity vs cubiomes'
//! `genBiomeNoiseChunkSection`. 64 biome IDs per case, all compared
//! bit-exactly.

#![allow(
    clippy::missing_panics_doc,
    clippy::items_after_statements,
    clippy::needless_range_loop
)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use cubioxides::generator::Generator;
use cubioxides::mc_version::{Dimension, MCVersion};

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 85;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("chunk_section.bin")
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

#[test]
fn chunk_section_matches_cubiomes() {
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
    // mc(4) seed(8) cx(4) cy(4) cz(4) ids[64*4] = 24 + 256 = 280
    const REC_LEN: usize = 4 + 8 + 4 * 3 + 64 * 4;

    for i in 0..count as usize {
        let r = &body[i * REC_LEN..(i + 1) * REC_LEN];
        let mc = mc_from_ord(read_i32(r, 0));
        let seed = read_u64(r, 4);
        let cx = read_i32(r, 12);
        let cy = read_i32(r, 16);
        let cz = read_i32(r, 20);
        let mut expected = [0_i32; 64];
        for k in 0..64 {
            expected[k] = read_i32(r, 24 + k * 4);
        }

        let mut g = Generator::new(mc, 0);
        g.apply_seed(Dimension::Overworld, seed);
        let bn = g.biome_noise.as_ref().expect("BiomeNoise seeded");
        let mut out = [[[0_i32; 4]; 4]; 4];
        let mut dat = 0_u64;
        bn.gen_chunk_section(&mut out, cx, cy, cz, &mut dat);
        let mut got = [0_i32; 64];
        let mut n = 0;
        for ix in 0..4 {
            for jy in 0..4 {
                for kz in 0..4 {
                    got[n] = out[ix][jy][kz];
                    n += 1;
                }
            }
        }
        assert_eq!(
            got, expected,
            "case {i} ({mc:?}, seed={seed:#x}, ({cx},{cy},{cz}))"
        );
    }
}
