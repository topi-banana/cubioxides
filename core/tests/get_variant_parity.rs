//! Parity test for `get_variant` vs cubiomes' `getVariant`. Covers
//! Village (all 5 supported biomes × 3 MC versions including the
//! meadow→plains alias) and Bastion (4 MC versions, including the
//! 1.16.1 start/rotation swap).

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::finder::{StructureType, get_variant};
use cubioxides::mc_version::MCVersion;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 68;

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
struct GetVariantRecord {
    structure_type: i32,
    mc: i32,
    biome_id: i32,
    rc: i32,
    seed: u64,
    x: i32,
    z: i32,
    fields: [i32; 11],
    pad: i32,
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("get_variant.bin")
}

fn mc_from_ord(ord: i32) -> MCVersion {
    match ord {
        17 => MCVersion::V1_14,
        19 => MCVersion::V1_16_1,
        21 => MCVersion::V1_17,
        22 => MCVersion::V1_18,
        28 => MCVersion::V1_21,
        other => panic!("unsupported MC ordinal: {other}"),
    }
}

#[test]
fn get_variant_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[GetVariantRecord] = bytemuck::cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    for (i, r) in recs.iter().enumerate() {
        let mc = mc_from_ord(r.mc);
        let sty = StructureType::from_ord(r.structure_type)
            .unwrap_or_else(|| panic!("unknown structure type ord {}", r.structure_type));
        let got = get_variant(sty, mc, r.seed, r.x, r.z, r.biome_id);
        let want_rc = r.rc != 0;
        if got.is_none() {
            assert!(
                !want_rc,
                "get_variant at record {i}: expected rc=1 but Rust returned None (mc={mc:?}, sty={sty:?}, biome={})",
                r.biome_id
            );
            continue;
        }
        assert!(
            want_rc,
            "get_variant at record {i}: expected rc=0 but Rust returned Some(_) (mc={mc:?}, sty={sty:?}, biome={})",
            r.biome_id
        );
        let v = got.unwrap();
        let got_fields = [
            i32::from(v.abandoned),
            i32::from(v.cracked),
            v.start as i32,
            v.biome as i32,
            v.rotation as i32,
            v.x as i32,
            v.y as i32,
            v.z as i32,
            v.sx as i32,
            v.sy as i32,
            v.sz as i32,
        ];
        assert!(
            got_fields == r.fields,
            "get_variant mismatch at record {i} (mc={mc:?}, sty={sty:?}, biome={}, seed={:#x}, x={}, z={}):\n  got: {:?}\n want: {:?}",
            r.biome_id,
            r.seed,
            r.x,
            r.z,
            got_fields,
            r.fields,
        );
    }
}
