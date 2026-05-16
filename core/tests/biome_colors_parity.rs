//! `init_biome_colors` / `init_biome_type_colors` parity vs cubiomes.
//! 768-byte byte-by-byte comparison.

#![allow(clippy::missing_panics_doc, clippy::doc_markdown)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::colors::{init_biome_colors, init_biome_type_colors};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Header {
    magic: [u8; 4],
    format_version: u16,
    kind: u16,
    record_count: u64,
    reserved: [u64; 2],
}

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 94;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("biome_colors.bin")
}

#[test]
fn biome_colors_match_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    assert_eq!(body.len(), 768 * 2, "expected 2x 768-byte palettes");

    let cubiomes_colors = &body[..768];
    let cubiomes_type = &body[768..];

    let rust_colors = init_biome_colors();
    let rust_type = init_biome_type_colors();

    // Flatten the 256x3 arrays for byte comparison.
    let rust_colors_flat: &[u8] = bytemuck::cast_slice(&rust_colors);
    let rust_type_flat: &[u8] = bytemuck::cast_slice(&rust_type);

    let mut diffs = 0;
    for (i, (&r, &c)) in rust_colors_flat.iter().zip(cubiomes_colors).enumerate() {
        if r != c {
            if diffs < 5 {
                eprintln!(
                    "colors byte {} (biome {}, ch {}): rust 0x{:02x} vs cubiomes 0x{:02x}",
                    i,
                    i / 3,
                    i % 3,
                    r,
                    c
                );
            }
            diffs += 1;
        }
    }
    assert_eq!(
        diffs, 0,
        "biome color palette mismatch ({diffs} bytes diff)"
    );

    let mut diffs = 0;
    for (&r, &c) in rust_type_flat.iter().zip(cubiomes_type) {
        if r != c {
            diffs += 1;
        }
    }
    assert_eq!(diffs, 0, "biome type color palette mismatch");
}
