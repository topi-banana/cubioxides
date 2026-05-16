//! `get_min_layer_cache_size` parity vs cubiomes' `getMinLayerCacheSize`.

#![allow(clippy::missing_panics_doc, clippy::items_after_statements)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use cubioxides::layer::cache::get_min_layer_cache_size;
use cubioxides::layer::stack::{LayerId, LayerStack, setup_layer_stack};
use cubioxides::mc_version::MCVersion;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 91;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("min_layer_cache_size.bin")
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

fn entry_from_ord(o: i32) -> LayerId {
    match o {
        0 => LayerId::Continent4096,
        21 => LayerId::Biome256,
        25 => LayerId::BiomeEdge64,
        29 => LayerId::Hills64,
        34 => LayerId::Shore16,
        47 => LayerId::RiverMix4,
        55 => LayerId::OceanMix4,
        56 => LayerId::Voronoi1,
        _ => panic!("unsupported entry {o}"),
    }
}

#[test]
fn min_layer_cache_size_matches_cubiomes() {
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
    const REC_LEN: usize = 4 + 4 + 4 + 4 + 8;

    for i in 0..count as usize {
        let r = &body[i * REC_LEN..(i + 1) * REC_LEN];
        let mc = mc_from_ord(read_i32(r, 0));
        let entry = entry_from_ord(read_i32(r, 4));
        let sx = read_i32(r, 8);
        let sz = read_i32(r, 12);
        let expected = read_u64(r, 16);
        let mut stack = LayerStack::default();
        setup_layer_stack(&mut stack, mc, false);
        let got = get_min_layer_cache_size(&stack, entry, sx, sz) as u64;
        assert_eq!(
            got, expected,
            "case {i} ({mc:?}, {entry:?}, {sx}×{sz}): rust {got} vs cubiomes {expected}",
        );
    }
}
