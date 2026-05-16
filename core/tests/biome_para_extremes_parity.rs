//! `get_biome_para_extremes` parity vs cubiomes' `getBiomeParaExtremes`.
//! Pure table lookup, so the comparison is exact integer equality.

#![allow(clippy::missing_panics_doc, clippy::items_after_statements)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use cubioxides::finder::biome_para::get_biome_para_extremes;
use cubioxides::mc_version::MCVersion;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 99;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("biome_para_extremes.bin")
}

fn read_i32(b: &[u8], off: usize) -> i32 {
    i32::from_le_bytes(b[off..off + 4].try_into().unwrap())
}

fn mc_from_ord(o: i32) -> MCVersion {
    match o {
        1 => MCVersion::B1_7,
        2 => MCVersion::B1_8,
        5 => MCVersion::V1_2,
        13 => MCVersion::V1_9,
        17 => MCVersion::V1_13,
        22 => MCVersion::V1_18,
        24 => MCVersion::V1_19,
        26 => MCVersion::V1_20,
        27 => MCVersion::V1_21_1,
        28 => MCVersion::V1_21,
        _ => panic!("unsupported mc ord {o}"),
    }
}

#[test]
fn biome_para_extremes_matches_cubiomes() {
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
    const REC_LEN: usize = 56; // 4 (mc) + 4 (has) + 12*4 (extremes)

    for i in 0..count as usize {
        let r = &body[i * REC_LEN..(i + 1) * REC_LEN];
        let mc = mc_from_ord(read_i32(r, 0));
        let has = read_i32(r, 4);
        let got = get_biome_para_extremes(mc);
        if has == 0 {
            assert!(
                got.is_none(),
                "case {i} ({mc:?}): rust returned Some, cubiomes returned NULL"
            );
        } else {
            let expected: [(i32, i32); 6] = [
                (read_i32(r, 8), read_i32(r, 12)),
                (read_i32(r, 16), read_i32(r, 20)),
                (read_i32(r, 24), read_i32(r, 28)),
                (read_i32(r, 32), read_i32(r, 36)),
                (read_i32(r, 40), read_i32(r, 44)),
                (read_i32(r, 48), read_i32(r, 52)),
            ];
            let got =
                got.unwrap_or_else(|| panic!("case {i} ({mc:?}): rust None, cubiomes had data"));
            assert_eq!(got, expected, "case {i} ({mc:?}): extremes mismatch");
        }
    }
}
