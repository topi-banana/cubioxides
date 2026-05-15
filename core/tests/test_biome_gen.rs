//! Port of cubiomes' `testBiomeGen1x1` from `tests.c` — the canonical
//! Overworld biome-generation regression test. For each MC version,
//! seed the layer stack with a deterministic per-cell seed and sample
//! the entry-scale-4 layer at 64×64 cells (`bits = 6`), folding
//! every result into a 32-bit hash. The expected hashes are taken
//! verbatim from cubiomes' `b6_hashes` array.
//!
//! Only the layer-based MC versions (≤ 1.16) are covered here: 1.18+
//! lives on the `BiomeNoise` path (M4) which is not yet ported. Beta
//! 1.8 is omitted because cubiomes itself records `0x00000000` for it
//! (i.e. no reference value has been pinned upstream).

#![allow(clippy::missing_panics_doc)]

use cubioxides::biome::Biome;
use cubioxides::layer::{LayerStack, gen_area, set_layer_seed, setup_layer_stack};
use cubioxides::mc_version::MCVersion;

fn hash32(mut x: u32) -> u32 {
    x ^= x >> 15;
    x = x.wrapping_mul(0xd168_aaad);
    x ^= x >> 15;
    x = x.wrapping_mul(0xaf72_3597);
    x ^= x >> 15;
    x
}

/// Mirror of cubiomes' `getRef(mc, dim=0, bits, scale=4, spread=1)`
/// for the layered Overworld path. Walks `[-r, r) x [-r, r)`, seeds
/// the stack per cell, samples the 1:4 entry layer once, and folds
/// each `(seed, id)` into the running XOR digest.
fn get_ref_layered(mc: MCVersion, bits: i32) -> u32 {
    let r: i32 = 1 << (bits - 1);
    let mut stack = Box::new(LayerStack::new());
    setup_layer_stack(&mut stack, mc, false);
    let entry_1 = stack.entry_1.expect("entry_1");
    let entry_4 = stack.entry_4.expect("entry_4");

    let mut digest: u32 = 0;
    let mut out = [Biome::NONE; 1];
    for x in -r..r {
        for z in -r..r {
            // cubiomes' seed pattern: s = (z << bits) ^ x, kept in
            // signed 64-bit before reinterpreting as the world seed.
            let s = ((z as i64) << bits) ^ (x as i64);
            set_layer_seed(&mut stack, entry_1, s as u64);
            gen_area(&stack, entry_4, &mut out, x, z, 1, 1);
            let id = out[0].id();
            // hash32(s ^ (id << 2 * bits)) — cubiomes truncates s to
            // `int` (i32) before folding.
            let folded = (s as i32) ^ (id << (2 * bits));
            digest ^= hash32(folded as u32);
        }
    }
    digest
}

#[test]
fn b6_hashes_layered_mc() {
    // (mc, expected) — copied directly from cubiomes/tests.c b6_hashes,
    // matched against the same index in mc_vers. 1.17 has no entry in
    // upstream; pre-1.18 starts at 1.16.
    let cases: &[(MCVersion, u32)] = &[
        (MCVersion::V1_16, 0xde9a_6574),
        (MCVersion::V1_15, 0x3a56_8a6d),
        (MCVersion::V1_13, 0x96c9_7323),
        (MCVersion::V1_12, 0xbc75_e996),
        (MCVersion::V1_9, 0xe27a_45a2),
        (MCVersion::V1_7, 0xbc75_e996),
        (MCVersion::V1_6, 0x15b4_7206),
        (MCVersion::V1_2, 0x2d7e_0fed),
        (MCVersion::V1_1, 0x5cbf_4709),
        (MCVersion::V1_0, 0xbd79_4adb),
    ];
    for &(mc, expected) in cases {
        let got = get_ref_layered(mc, 6);
        assert_eq!(
            got, expected,
            "testBiomeGen1x1 b6 mismatch for MC {mc:?}: expected {expected:#010x}, got {got:#010x}"
        );
    }
}
