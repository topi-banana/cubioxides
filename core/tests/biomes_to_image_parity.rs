//! `biomes_to_image` parity vs cubiomes' `biomesToImage`.

#![allow(clippy::missing_panics_doc, clippy::doc_markdown)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::colors::{biomes_to_image, init_biome_colors};

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
const KIND: u16 = 95;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("biomes_to_image.bin")
}

#[test]
fn biomes_to_image_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, mut body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let palette = init_biome_colors();

    for case in 0..h.record_count {
        // Parse header: [sx u32][sy u32][pixscale u32][flip u8][invalid u8][pad u16]
        let sx = u32::from_le_bytes(body[0..4].try_into().unwrap());
        let sy = u32::from_le_bytes(body[4..8].try_into().unwrap());
        let pixscale = u32::from_le_bytes(body[8..12].try_into().unwrap());
        let flip = body[12] != 0;
        let cubiomes_invalid = body[13] != 0;
        body = &body[16..];

        let n_biomes = (sx * sy) as usize;
        let mut biomes: Vec<i32> = Vec::with_capacity(n_biomes);
        for i in 0..n_biomes {
            let v = i32::from_le_bytes(body[i * 4..i * 4 + 4].try_into().unwrap());
            biomes.push(v);
        }
        body = &body[n_biomes * 4..];

        let pixel_bytes = (sx * pixscale * sy * pixscale * 3) as usize;
        let cubiomes_pixels = &body[..pixel_bytes];
        body = &body[pixel_bytes..];

        let mut rust_pixels = vec![0u8; pixel_bytes];
        let rust_invalid =
            biomes_to_image(&mut rust_pixels, &palette, &biomes, sx, sy, pixscale, flip);
        assert_eq!(
            rust_invalid, cubiomes_invalid,
            "case {case}: invalid flag mismatch — rust {rust_invalid} vs cubiomes {cubiomes_invalid}"
        );
        let mut diffs = 0;
        for (i, (&r, &c)) in rust_pixels.iter().zip(cubiomes_pixels).enumerate() {
            if r != c {
                if diffs < 5 {
                    eprintln!("case {case} byte {i}: rust 0x{r:02x} vs cubiomes 0x{c:02x}");
                }
                diffs += 1;
            }
        }
        assert_eq!(diffs, 0, "case {case}: {diffs} pixel byte mismatches");
    }
}
