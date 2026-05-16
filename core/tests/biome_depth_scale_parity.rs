//! Parity test for `get_biome_depth_and_scale` vs cubiomes'
//! `getBiomeDepthAndScale`. Exercises every biome id in 0..256.

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::biome::get_biome_depth_and_scale;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 62;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Header {
    magic: [u8; 4],
    format_version: u16,
    kind: u16,
    record_count: u64,
    reserved: [u64; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BiomeDepthScaleRecord {
    id: i32,
    found: i32,
    grass: i32,
    pad: i32,
    depth_bits: u64,
    scale_bits: u64,
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("biome_depth_scale.bin")
}

#[test]
fn biome_depth_and_scale_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[BiomeDepthScaleRecord] = bytemuck::cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    for r in recs {
        let got = get_biome_depth_and_scale(r.id);
        if r.found == 0 {
            assert!(
                got.is_none(),
                "biome id {} should be unknown but got {:?}",
                r.id,
                got
            );
        } else {
            let g = got.unwrap_or_else(|| panic!("biome id {} should be known", r.id));
            assert!(
                g.depth.to_bits() == r.depth_bits
                    && g.scale.to_bits() == r.scale_bits
                    && g.grass == r.grass,
                "biome id {} mismatch: got (depth={:#x}, scale={:#x}, grass={}), want (depth={:#x}, scale={:#x}, grass={})",
                r.id,
                g.depth.to_bits(),
                g.scale.to_bits(),
                g.grass,
                r.depth_bits,
                r.scale_bits,
                r.grass
            );
        }
    }
}
