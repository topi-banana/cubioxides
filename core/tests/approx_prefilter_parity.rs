//! `approx_prefilter_at_layer` parity vs cubiomes' `BF_APPROX` path.
//! The Rust port stops at the prefilter (cubiomes continues with
//! the swap-map chain), so we can only assert the invariant
//! "rust says reject (false) → cubiomes also rejects (result=0)".
//! When rust says pass (true), cubiomes' final answer can be
//! anything (0, 1, or 2) depending on the unimplemented chain.

#![allow(
    clippy::missing_panics_doc,
    clippy::items_after_statements,
    clippy::many_single_char_names,
    clippy::needless_range_loop
)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use cubioxides::finder::biome_filter::setup_biome_filter;
use cubioxides::finder::check_for_biomes::approx_prefilter_at_layer;
use cubioxides::layer::stack::{LayerStack, setup_layer_stack};
use cubioxides::mc_version::MCVersion;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 81;
const BF_APPROX: u32 = 0x1;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("approx_prefilter.bin")
}

fn read_i32(b: &[u8], off: usize) -> i32 {
    i32::from_le_bytes(b[off..off + 4].try_into().unwrap())
}
fn read_u64(b: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(b[off..off + 8].try_into().unwrap())
}

fn mc_from_ord(o: i32) -> MCVersion {
    match o {
        17 => MCVersion::V1_14,
        _ => panic!("unsupported mc ord {o}"),
    }
}

#[test]
fn approx_prefilter_consistent_with_cubiomes() {
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
    // mc(4) seed(8) entry_scale,x,z,w,h(5*4) req_len,exc_len,any_len(3*4)
    // + req[8]*4 + exc[8]*4 + any[8]*4 + result(4) = 144
    const REC_LEN: usize = 4 + 8 + 4 * 5 + 4 * 3 + 4 * 8 * 3 + 4;

    for i in 0..count as usize {
        let r = &body[i * REC_LEN..(i + 1) * REC_LEN];
        let mc = mc_from_ord(read_i32(r, 0));
        let seed = read_u64(r, 4);
        let entry_scale = read_i32(r, 12);
        let x = read_i32(r, 16);
        let z = read_i32(r, 20);
        let w = read_i32(r, 24);
        let h = read_i32(r, 28);
        let req_len = read_i32(r, 32) as usize;
        let exc_len = read_i32(r, 36) as usize;
        let any_len = read_i32(r, 40) as usize;
        let mut req = [0_i32; 8];
        let mut exc = [0_i32; 8];
        let mut any = [0_i32; 8];
        for j in 0..8 {
            req[j] = read_i32(r, 44 + j * 4);
            exc[j] = read_i32(r, 76 + j * 4);
            any[j] = read_i32(r, 108 + j * 4);
        }
        let cubiomes_result = read_i32(r, 140);

        let filter = setup_biome_filter(
            mc,
            BF_APPROX,
            &req[..req_len],
            &exc[..exc_len],
            &any[..any_len],
        )
        .expect("filter");
        let mut stack = LayerStack::default();
        setup_layer_stack(&mut stack, mc, false);
        let prefilter_pass =
            approx_prefilter_at_layer(&stack, &filter, seed, entry_scale, x, z, w as u32, h as u32);

        if !prefilter_pass {
            assert_eq!(
                cubiomes_result, 0,
                "case {i}: rust prefilter rejected but cubiomes returned {cubiomes_result} \
                 (seed={seed:#x} req={req:?}[..{req_len}])",
            );
        }
        // No assertion when prefilter passes — cubiomes' result depends
        // on the unimplemented swap-map chain.
    }
}
