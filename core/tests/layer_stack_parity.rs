//! Parity test: cubioxides' `setup_layer_stack` + `set_layer_seed`
//! vs cubiomes' `setupLayerStack` + `setLayerSeed`. Reads the binary
//! fixture produced by `fixtures-gen layers` (kind = 37) and compares
//! `(layer_salt, start_salt, start_seed)` slot-by-slot for every node
//! in the stack across a matrix of MC versions, world seeds, and the
//! `largeBiomes` toggle.
//!
//! Cubiomes' index ordering is mirrored in [`cubioxides::layer::LayerId`];
//! the test consumes the file by reading a small header per record
//! followed by `L_NUM * 3` `u64`s.

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::layer::stack::{LayerStack, set_layer_seed, setup_layer_stack};
use cubioxides::mc_version::MCVersion;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 37;

/// Number of cubiomes-side layer slots (`L_VORONOI_1` + 4
/// large-biome zooms = 61). The Rust `L_NUM` adds three xlayer
/// slots reserved for `FORCE_OCEAN_VARIANTS`; the fixture,
/// generated against cubiomes' enum, only covers the first 61.
const CUBIOMES_L_NUM: usize = 61;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct FileHeader {
    magic: [u8; 4],
    format_version: u16,
    kind: u16,
    record_count: u64,
    reserved: [u64; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct LayerStackHeader {
    mc: u32,
    large_biomes: u32,
    world_seed: u64,
}

const RECORD_BYTES: usize =
    std::mem::size_of::<LayerStackHeader>() + CUBIOMES_L_NUM * 3 * std::mem::size_of::<u64>();

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("fixtures")
        .join("layers")
        .join("layer_stack.bin")
}

fn mc_from_ord(ord: u32) -> MCVersion {
    match ord {
        2 => MCVersion::B1_8,
        3 => MCVersion::V1_0,
        4 => MCVersion::V1_1,
        5 => MCVersion::V1_2,
        6 => MCVersion::V1_3,
        9 => MCVersion::V1_6,
        10 => MCVersion::V1_7,
        15 => MCVersion::V1_12,
        16 => MCVersion::V1_13,
        17 => MCVersion::V1_14,
        22 => MCVersion::V1_18,
        25 => MCVersion::V1_20,
        other => panic!("unsupported MC ordinal in fixture: {other}"),
    }
}

#[test]
fn setup_layer_stack_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let header_size = std::mem::size_of::<FileHeader>();
    let (header_bytes, body_bytes) = bytes.split_at(header_size);
    let header: &FileHeader = bytemuck::from_bytes(header_bytes);
    assert_eq!(header.magic, MAGIC, "wrong magic");
    assert_eq!(
        header.format_version, FORMAT_VERSION,
        "wrong format version"
    );
    assert_eq!(header.kind, KIND, "wrong fixture kind");
    let count = header.record_count as usize;
    assert_eq!(
        body_bytes.len(),
        count * RECORD_BYTES,
        "fixture body length doesn't match {count} records of {RECORD_BYTES} bytes"
    );
    let layer_state_bytes = CUBIOMES_L_NUM * 3 * std::mem::size_of::<u64>();
    let lstack_hdr_size = std::mem::size_of::<LayerStackHeader>();

    let mut stack = Box::new(LayerStack::new());
    for record_idx in 0..count {
        let offset = record_idx * RECORD_BYTES;
        let (rec_hdr_bytes, payload) =
            body_bytes[offset..offset + RECORD_BYTES].split_at(lstack_hdr_size);
        let rec_hdr: &LayerStackHeader = bytemuck::from_bytes(rec_hdr_bytes);
        let expected: &[u64] = bytemuck::cast_slice(&payload[..layer_state_bytes]);

        let mc = mc_from_ord(rec_hdr.mc);
        let large_biomes = rec_hdr.large_biomes != 0;

        setup_layer_stack(&mut stack, mc, large_biomes);
        let entry = stack.entry_1.expect("entry_1 set");
        set_layer_seed(&mut stack, entry, rec_hdr.world_seed);

        for layer_idx in 0..CUBIOMES_L_NUM {
            let node = &stack.layers[layer_idx];
            let exp_layer_salt = expected[3 * layer_idx];
            let exp_start_salt = expected[3 * layer_idx + 1];
            let exp_start_seed = expected[3 * layer_idx + 2];
            assert_eq!(
                node.layer_salt, exp_layer_salt,
                "layer_salt mismatch (record {record_idx}, mc ord {}, large_biomes {}, layer {layer_idx}): got {:#x}, want {:#x}",
                rec_hdr.mc, rec_hdr.large_biomes, node.layer_salt, exp_layer_salt
            );
            assert_eq!(
                node.start_salt,
                exp_start_salt,
                "start_salt mismatch (record {record_idx}, mc ord {}, large_biomes {}, layer {layer_idx}, world {:#x}): got {:#x}, want {:#x}",
                rec_hdr.mc,
                rec_hdr.large_biomes,
                rec_hdr.world_seed,
                node.start_salt,
                exp_start_salt
            );
            assert_eq!(
                node.start_seed,
                exp_start_seed,
                "start_seed mismatch (record {record_idx}, mc ord {}, large_biomes {}, layer {layer_idx}, world {:#x}): got {:#x}, want {:#x}",
                rec_hdr.mc,
                rec_hdr.large_biomes,
                rec_hdr.world_seed,
                node.start_seed,
                exp_start_seed
            );
        }
    }
}
