//! `setup_biome_filter` parity vs cubiomes' `setupBiomeFilter`.

#![allow(
    clippy::missing_panics_doc,
    clippy::items_after_statements,
    clippy::needless_range_loop,
    clippy::too_many_lines
)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use cubioxides::finder::biome_filter::setup_biome_filter;
use cubioxides::mc_version::MCVersion;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 92;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("biome_filter.bin")
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
    // cubiomes' MC enum starts at MC_UNDEF=0, MC_B1_7=1, MC_B1_8=2,
    // MC_1_0=3, MC_1_1=4, MC_1_2=5, …, MC_1_14=17, MC_1_17=20,
    // MC_1_18=22, MC_1_21=28. Rust's `MCVersion` matches the
    // ordering exactly.
    match o {
        10 => MCVersion::V1_7,
        17 => MCVersion::V1_14,
        _ => panic!("unsupported mc ord {o}"),
    }
}

#[test]
fn biome_filter_matches_cubiomes() {
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
    // Layout: mc(4) flags(4) req_len(4) exc_len(4) any_len(4)
    // + req[8*4] + exc[8*4] + any[8*4] = 20 + 96 = 116
    // + masks[26*8] = 208 → 324
    // + special_cnt(4) + out_flags(4) = 332
    const REC_LEN: usize = 332;

    for i in 0..count as usize {
        let r = &body[i * REC_LEN..(i + 1) * REC_LEN];
        let mc = mc_from_ord(read_i32(r, 0));
        let flags = read_u32(r, 4);
        let req_len = read_i32(r, 8) as usize;
        let exc_len = read_i32(r, 12) as usize;
        let any_len = read_i32(r, 16) as usize;
        let mut req = [0_i32; 8];
        let mut exc = [0_i32; 8];
        let mut any = [0_i32; 8];
        for j in 0..8 {
            req[j] = read_i32(r, 20 + j * 4);
            exc[j] = read_i32(r, 52 + j * 4);
            any[j] = read_i32(r, 84 + j * 4);
        }
        let mut expected = [0_u64; 26];
        for j in 0..26 {
            expected[j] = read_u64(r, 116 + j * 8);
        }
        let expected_special_cnt = read_i32(r, 116 + 26 * 8);
        let expected_flags = read_u32(r, 116 + 26 * 8 + 4);

        let bf = setup_biome_filter(mc, flags, &req[..req_len], &exc[..exc_len], &any[..any_len])
            .expect("filter should build");

        let got = [
            bf.temps_to_find,
            bf.otemp_to_find,
            bf.major_to_find,
            bf.edges_to_find,
            bf.rares_to_find,
            bf.rares_to_find_m,
            bf.shore_to_find,
            bf.shore_to_find_m,
            bf.river_to_find,
            bf.river_to_find_m,
            bf.ocean_to_find,
            bf.temps_to_excl,
            bf.major_to_excl,
            bf.edges_to_excl,
            bf.rares_to_excl,
            bf.rares_to_excl_m,
            bf.shore_to_excl,
            bf.shore_to_excl_m,
            bf.river_to_excl,
            bf.river_to_excl_m,
            bf.biome_to_excl,
            bf.biome_to_excl_m,
            bf.biome_to_find,
            bf.biome_to_find_m,
            bf.biome_to_pick,
            bf.biome_to_pick_m,
        ];
        let names = [
            "temps_to_find",
            "otemp_to_find",
            "major_to_find",
            "edges_to_find",
            "rares_to_find",
            "rares_to_find_m",
            "shore_to_find",
            "shore_to_find_m",
            "river_to_find",
            "river_to_find_m",
            "ocean_to_find",
            "temps_to_excl",
            "major_to_excl",
            "edges_to_excl",
            "rares_to_excl",
            "rares_to_excl_m",
            "shore_to_excl",
            "shore_to_excl_m",
            "river_to_excl",
            "river_to_excl_m",
            "biome_to_excl",
            "biome_to_excl_m",
            "biome_to_find",
            "biome_to_find_m",
            "biome_to_pick",
            "biome_to_pick_m",
        ];
        for j in 0..26 {
            assert_eq!(
                got[j], expected[j],
                "case {i} ({mc:?}): {} mismatch — rust {:#x} vs cubiomes {:#x}",
                names[j], got[j], expected[j],
            );
        }
        assert_eq!(
            bf.special_cnt, expected_special_cnt,
            "case {i}: special_cnt mismatch",
        );
        assert_eq!(bf.flags, expected_flags, "case {i}: flags mismatch");
    }
}
