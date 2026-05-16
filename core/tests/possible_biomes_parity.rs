//! `get_possible_biomes_for_limits` parity vs cubiomes.

#![allow(clippy::missing_panics_doc, clippy::items_after_statements)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use cubioxides::finder::biome_para::get_possible_biomes_for_limits;
use cubioxides::mc_version::MCVersion;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 94;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("possible_biomes_for_limits.bin")
}

fn read_i32(b: &[u8], off: usize) -> i32 {
    i32::from_le_bytes(b[off..off + 4].try_into().unwrap())
}

fn mc_from_ord(o: i32) -> MCVersion {
    match o {
        22 => MCVersion::V1_18,
        23 => MCVersion::V1_19_2,
        24 => MCVersion::V1_19,
        25 => MCVersion::V1_20,
        26 => MCVersion::V1_21_1,
        27 => MCVersion::V1_21_3,
        28 => MCVersion::V1_21,
        _ => panic!("unsupported mc ord {o}"),
    }
}

#[test]
fn possible_biomes_matches_cubiomes() {
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
    const REC_LEN: usize = 4 + 12 * 4 + 256;

    for i in 0..count as usize {
        let r = &body[i * REC_LEN..(i + 1) * REC_LEN];
        let mc = mc_from_ord(read_i32(r, 0));
        let limits: [(i32, i32); 6] = [
            (read_i32(r, 4), read_i32(r, 8)),
            (read_i32(r, 12), read_i32(r, 16)),
            (read_i32(r, 20), read_i32(r, 24)),
            (read_i32(r, 28), read_i32(r, 32)),
            (read_i32(r, 36), read_i32(r, 40)),
            (read_i32(r, 44), read_i32(r, 48)),
        ];
        let expected: &[u8] = &r[52..52 + 256];
        let got = get_possible_biomes_for_limits(mc, &limits);
        for id in 0..256 {
            let expected_bit = expected[id] != 0;
            let got_bit = got[id];
            assert_eq!(
                got_bit, expected_bit,
                "case {i} ({mc:?}) id={id}: rust {got_bit} vs cubiomes {expected_bit}",
            );
        }
    }
}
