//! `gen_potential` parity vs cubiomes' `genPotential` / `_genPotential`.

#![allow(clippy::missing_panics_doc, clippy::items_after_statements)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use cubioxides::finder::gen_potential::gen_potential;
use cubioxides::layer::stack::LayerId;
use cubioxides::mc_version::MCVersion;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 93;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("gen_potential.bin")
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
        5 => MCVersion::V1_2,
        22 => MCVersion::V1_18,
        28 => MCVersion::V1_21,
        _ => panic!("unsupported mc ord {o}"),
    }
}

fn layer_from_ord(o: i32) -> LayerId {
    match o {
        14 => LayerId::Special1024,
        19 => LayerId::Mushroom256,
        20 => LayerId::DeepOcean256,
        21 => LayerId::Biome256,
        22 => LayerId::Bamboo256,
        24 => LayerId::Zoom64,
        25 => LayerId::BiomeEdge64,
        29 => LayerId::Hills64,
        30 => LayerId::Sunflower64,
        33 => LayerId::Zoom16,
        34 => LayerId::Shore16,
        35 => LayerId::SwampRiver16,
        37 => LayerId::Zoom4,
        47 => LayerId::RiverMix4,
        55 => LayerId::OceanMix4,
        56 => LayerId::Voronoi1,
        _ => panic!("unsupported layer ord {o}"),
    }
}

#[test]
fn gen_potential_matches_cubiomes() {
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
    const REC_LEN: usize = 4 + 4 + 4 + 4 + 8 + 8;

    for i in 0..count as usize {
        let r = &body[i * REC_LEN..(i + 1) * REC_LEN];
        let mc = mc_from_ord(read_i32(r, 0));
        let layer = layer_from_ord(read_i32(r, 4));
        let flags = read_u32(r, 8);
        let biome = read_i32(r, 12);
        let expected_l = read_u64(r, 16);
        let expected_m = read_u64(r, 24);
        let mut got_l: u64 = 0;
        let mut got_m: u64 = 0;
        gen_potential(mc, flags, layer, biome, &mut got_l, &mut got_m);
        assert_eq!(
            got_l, expected_l,
            "case {i} ({mc:?}, {layer:?}, flags={flags:#x}, biome={biome}): mL mismatch — rust {got_l:#x} vs cubiomes {expected_l:#x}",
        );
        assert_eq!(
            got_m, expected_m,
            "case {i} ({mc:?}, {layer:?}, flags={flags:#x}, biome={biome}): mM mismatch — rust {got_m:#x} vs cubiomes {expected_m:#x}",
        );
    }
}
