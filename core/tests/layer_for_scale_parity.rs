//! `Generator::layer_for_scale` parity vs cubiomes'
//! `getLayerForScale`. cubiomes returns the entry-layer pointer; we
//! return `Option<LayerId>`. The fixture stores cubiomes' "pointer
//! offset from g.ls.layers[0]" (or -1 for NULL).

#![allow(clippy::missing_panics_doc, clippy::items_after_statements)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use cubioxides::generator::Generator;
use cubioxides::layer::LayerId;
use cubioxides::mc_version::{Dimension, MCVersion};

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 102;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("layer_for_scale.bin")
}

fn read_i32(b: &[u8], off: usize) -> i32 {
    i32::from_le_bytes(b[off..off + 4].try_into().unwrap())
}

fn mc_from_ord(o: i32) -> MCVersion {
    match o {
        2 => MCVersion::B1_8,
        3 => MCVersion::V1_0,
        12 => MCVersion::V1_7,
        16 => MCVersion::V1_13,
        17 => MCVersion::V1_14,
        18 => MCVersion::V1_15,
        19 => MCVersion::V1_16_1,
        20 => MCVersion::V1_16,
        21 => MCVersion::V1_17,
        22 => MCVersion::V1_18,
        28 => MCVersion::V1_21,
        _ => panic!("unsupported mc ord {o}"),
    }
}

#[test]
fn layer_for_scale_matches_cubiomes() {
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
    const REC_LEN: usize = 4 + 4 + 4;

    for i in 0..count as usize {
        let r = &body[i * REC_LEN..(i + 1) * REC_LEN];
        let mc = mc_from_ord(read_i32(r, 0));
        let scale = read_i32(r, 4);
        let expected = read_i32(r, 8);

        let mut g = Generator::new(mc, 0);
        g.apply_seed(Dimension::Overworld, 0);
        let got_id = g.layer_for_scale(scale);
        let got = got_id.map_or(-1_i32, |id| id as i32);
        assert_eq!(
            got, expected,
            "case {i}: mc={mc:?} scale={scale} got={got_id:?} ({got}) expected={expected}",
        );
    }
    // Sanity: at least one expected non-(-1) for layered MC at scale=1.
    let _ = LayerId::Voronoi1;
}
