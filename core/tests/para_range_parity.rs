//! `get_para_range` parity vs cubiomes' `getParaRange`. Compares
//! the (pmin, pmax) pair bit-exactly via `f64::to_bits()`, plus the
//! error code (always 0 here since we don't pass a callback).

#![allow(
    clippy::missing_panics_doc,
    clippy::items_after_statements,
    clippy::many_single_char_names
)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use cubioxides::finder::para_range::get_para_range;
use cubioxides::generator::Generator;
use cubioxides::mc_version::{Dimension, MCVersion};

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 87;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("para_range.bin")
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
fn para_range_matches_cubiomes() {
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
    // 4(mc) + 8(seed) + 4(npara) + 4(pmin_en) + 4(pmax_en)
    // + 4(x) + 4(z) + 4(w) + 4(h) + 4(err) + 8(pmin) + 8(pmax) = 60
    const REC_LEN: usize = 4 + 8 + 4 + 4 + 4 + 4 * 4 + 4 + 8 + 8;

    for i in 0..count as usize {
        let r = &body[i * REC_LEN..(i + 1) * REC_LEN];
        let mc = mc_from_ord(read_i32(r, 0));
        let seed = read_u64(r, 4);
        let npara = read_i32(r, 12) as usize;
        let pmin_en = read_i32(r, 16) != 0;
        let pmax_en = read_i32(r, 20) != 0;
        let x = read_i32(r, 24);
        let z = read_i32(r, 28);
        let w = read_i32(r, 32);
        let h = read_i32(r, 36);
        let expected_err = read_i32(r, 40);
        let expected_pmin_bits = read_u64(r, 44);
        let expected_pmax_bits = read_u64(r, 52);

        let mut g = Generator::new(mc, 0);
        g.apply_seed(Dimension::Overworld, seed);
        let bn = g
            .biome_noise
            .as_ref()
            .expect("BiomeNoise must be seeded for 1.18+");
        let para = &bn.climate[npara];
        let result =
            get_para_range::<fn(i32, i32, f64) -> i32>(para, pmin_en, pmax_en, x, z, w, h, None);
        match result {
            Ok((pmin, pmax)) => {
                assert_eq!(
                    expected_err, 0,
                    "case {i}: cubiomes returned err {expected_err}"
                );
                assert_eq!(
                    pmin.to_bits(),
                    expected_pmin_bits,
                    "case {i}: pmin mismatch rust {:#x} vs cubiomes {expected_pmin_bits:#x}",
                    pmin.to_bits()
                );
                assert_eq!(
                    pmax.to_bits(),
                    expected_pmax_bits,
                    "case {i}: pmax mismatch rust {:#x} vs cubiomes {expected_pmax_bits:#x}",
                    pmax.to_bits()
                );
            }
            Err(code) => {
                assert_eq!(code, expected_err, "case {i}: err mismatch");
            }
        }
    }
}
