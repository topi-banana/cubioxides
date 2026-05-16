//! `parse_biome_colors` parity vs cubiomes' `parseBiomeColors`.

#![allow(clippy::missing_panics_doc, clippy::items_after_statements)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use cubioxides::colors::parse_biome_colors;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 89;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("parse_biome_colors.bin")
}

#[test]
fn parse_biome_colors_matches_cubiomes() {
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
        let input_len = i32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        let input = std::str::from_utf8(&bytes[offset..offset + input_len]).expect("utf8");
        offset += input_len;
        let expected_result = i32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let expected_palette: [u8; 768] = bytes[offset..offset + 768].try_into().unwrap();
        offset += 768;

        let mut palette = [[0_u8; 3]; 256];
        let result = parse_biome_colors(&mut palette, input);
        let mut palette_flat = [0_u8; 768];
        for (j, rgb) in palette.iter().enumerate() {
            palette_flat[j * 3] = rgb[0];
            palette_flat[j * 3 + 1] = rgb[1];
            palette_flat[j * 3 + 2] = rgb[2];
        }
        assert_eq!(
            result, expected_result,
            "case {i} input={input:?}: result rust {result} vs cubiomes {expected_result}",
        );
        for j in 0..256 {
            for k in 0..3 {
                let cidx = j * 3 + k;
                assert_eq!(
                    palette_flat[cidx], expected_palette[cidx],
                    "case {i} input={input:?}: palette[{j}][{k}] rust {} vs cubiomes {}",
                    palette_flat[cidx], expected_palette[cidx],
                );
            }
        }
    }
}
