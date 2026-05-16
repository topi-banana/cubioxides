//! Stronghold iteration — `initFirstStronghold` + a no-biome
//! variant of `nextStronghold` (the 1.19.3+ path that doesn't
//! consult the biome generator).
//!
//! The biome-checked variant relies on `locateBiome` /
//! `isStrongholdBiome`, which require the full Overworld biome
//! pipeline and a much larger surface; it lands in a follow-up.

#![allow(clippy::many_single_char_names)]

use crate::biome::Biome;
use crate::finder::Pos;
use crate::finder::locate_biome::locate_biome;
use crate::generator::Generator;
use crate::mc_version::MCVersion;
use crate::rng::JavaRng;

/// `isStrongholdBiome(mc, id)` — does this biome allow stronghold
/// generation on `mc`? Mirrors cubiomes' `isStrongholdBiome` from
/// `finders.c`.
#[must_use]
pub fn is_stronghold_biome(mc: MCVersion, id: i32) -> bool {
    if !Biome::is_overworld_id(mc, id) {
        return false;
    }
    if Biome::is_oceanic_id(id) {
        return false;
    }
    match id {
        // plains, mushroom_fields, taiga_hills — 1.7+
        1 | 14 | 19 => mc.is_at_least(MCVersion::V1_7),
        // swamp — 1.6 and earlier
        6 => !mc.is_at_least(MCVersion::V1_7),
        // river, frozen_river, beach, snowy_beach, swamp_hills,
        // mangrove_swamp, deep_dark — never
        7 | 11 | 16 | 26 | 134 | 183 | 184 => false,
        // mushroom_field_shore — 1.13+
        15 => mc.is_at_least(MCVersion::V1_13),
        // stone_shore — 1.17 and earlier
        25 => !mc.is_at_least(MCVersion::V1_18),
        // bamboo_jungle / bamboo_jungle_hills — emulates MC-199298:
        //   1.15 and earlier or 1.18+
        168 | 169 => !mc.is_at_least(MCVersion::V1_16_1) || mc.is_at_least(MCVersion::V1_18),
        _ => true,
    }
}

const PI: f64 = std::f64::consts::PI;

/// Stronghold iteration state — bit-exact mirror of cubiomes'
/// `STRUCT(StrongholdIter)`. The Rust port keeps the same field
/// names + types so it round-trips through the C struct via FFI
/// for parity checks.
#[derive(Debug, Clone, Copy)]
pub struct StrongholdIter {
    /// Accurate location of the current stronghold (populated by
    /// `next_stronghold_*`).
    pub pos: Pos,
    /// Approximate location of the *next* stronghold (±112 blocks).
    pub nextapprox: Pos,
    /// 0-indexed counter incremented per stronghold.
    pub index: i32,
    /// Current ring number (0-indexed; first 3 strongholds are
    /// ring 0).
    pub ringnum: i32,
    /// Max strongholds in the current ring.
    pub ringmax: i32,
    /// Index within the current ring.
    pub ringidx: i32,
    /// Next angle (radians) within the ring.
    pub angle: f64,
    /// Next distance from origin (in chunks).
    pub dist: f64,
    /// Java-RNG state mid-stream.
    pub rnds: JavaRng,
    /// Configured MC version.
    pub mc: MCVersion,
}

/// Approximate location of the first stronghold, plus an
/// optionally-initialised iterator for the rest. Mirrors cubiomes'
/// `initFirstStronghold(sh, mc, s48)`.
#[must_use]
pub fn init_first_stronghold(mc: MCVersion, s48: u64) -> (Pos, StrongholdIter) {
    let mut rnds = JavaRng::new(s48);
    let angle = 2.0 * PI * rnds.next_double();
    let dist = if mc.is_at_least(MCVersion::V1_9) {
        (4.0 * 32.0) + (rnds.next_double() - 0.5) * 32.0 * 2.5
    } else {
        (1.25 + rnds.next_double()) * 32.0
    };
    let px = ((angle.cos() * dist).round() as i32 * 16) + 8;
    let pz = ((angle.sin() * dist).round() as i32 * 16) + 8;
    let p = Pos { x: px, z: pz };
    let iter = StrongholdIter {
        pos: Pos { x: 0, z: 0 },
        nextapprox: p,
        index: 0,
        ringnum: 0,
        ringmax: 3,
        ringidx: 0,
        angle,
        dist,
        rnds,
        mc,
    };
    (p, iter)
}

/// Build cubiomes' `(validB, validM)` masks for stronghold biomes
/// on the given MC.
fn stronghold_biome_masks(mc: MCVersion) -> (u64, u64) {
    let mut valid_b: u64 = 0;
    let mut valid_m: u64 = 0;
    for i in 0..64 {
        if is_stronghold_biome(mc, i) {
            valid_b |= 1u64 << i;
        }
        if is_stronghold_biome(mc, i + 128) {
            valid_m |= 1u64 << i;
        }
    }
    (valid_b, valid_m)
}

/// `nextStronghold(sh, g)` — biome-aware variant. Snaps `sh.pos`
/// to the closest matching stronghold biome and advances `sh` to
/// the next stronghold's approximate position. Returns the
/// countdown (strongholds remaining after this call).
///
/// Requires `g` to be `apply_seed`-ed on Overworld with the same
/// world seed `sh` was initialised from. Pre-B1.8 returns 0 (no
/// strongholds in that era).
pub fn next_stronghold(sh: &mut StrongholdIter, g: &Generator) -> i32 {
    if !sh.mc.is_at_least(MCVersion::B1_8) {
        return 0;
    }
    let (valid_b, valid_m) = stronghold_biome_masks(sh.mc);

    if sh.mc.is_at_least(MCVersion::V1_19) {
        // 1.19.4+: fresh local Java RNG seeded from rnds.next_long.
        let mut lbr = JavaRng::new(sh.rnds.next_long());
        let (pos, _) = locate_biome(
            g,
            sh.nextapprox.x,
            0,
            sh.nextapprox.z,
            112,
            valid_b,
            valid_m,
            &mut lbr,
        );
        sh.pos = pos;
    } else {
        // B1.8 – 1.19.2: locate_biome shares the iterator's RNG.
        let (pos, _) = locate_biome(
            g,
            sh.nextapprox.x,
            0,
            sh.nextapprox.z,
            112,
            valid_b,
            valid_m,
            &mut sh.rnds,
        );
        sh.pos = pos;
    }

    advance_iter_state(sh);
    if sh.mc.is_at_least(MCVersion::V1_9) {
        128 - (sh.index - 1)
    } else {
        3 - (sh.index - 1)
    }
}

/// Common ring-advance logic shared between the biome-aware and
/// no-biome stronghold iterators.
fn advance_iter_state(sh: &mut StrongholdIter) {
    // Snap to chunk staircase position (4, 4 inside the chunk).
    sh.pos.x = (sh.pos.x & !15) + 4;
    sh.pos.z = (sh.pos.z & !15) + 4;

    sh.ringidx += 1;
    sh.angle += 2.0 * PI / f64::from(sh.ringmax);

    if sh.ringidx == sh.ringmax {
        sh.ringnum += 1;
        sh.ringidx = 0;
        sh.ringmax += 2 * sh.ringmax / (sh.ringnum + 1);
        if sh.ringmax > 128 - sh.index {
            sh.ringmax = 128 - sh.index;
        }
        sh.angle += sh.rnds.next_double() * PI * 2.0;
    }

    sh.dist = if sh.mc.is_at_least(MCVersion::V1_9) {
        (4.0 * 32.0)
            + (6.0 * f64::from(sh.ringnum) * 32.0)
            + (sh.rnds.next_double() - 0.5) * 32.0 * 2.5
    } else {
        (1.25 + sh.rnds.next_double()) * 32.0
    };

    sh.nextapprox.x = ((sh.angle.cos() * sh.dist).round() as i32 * 16) + 8;
    sh.nextapprox.z = ((sh.angle.sin() * sh.dist).round() as i32 * 16) + 8;
    sh.index += 1;
}

/// MC 1.19.4+ no-biome stronghold advance — mirrors cubiomes'
/// `nextStronghold(sh, g=NULL)` for `mc > MC_1_19_2`. Returns the
/// number of strongholds remaining after this one.
///
/// For pre-1.19.4 the biome-checked variant `next_stronghold` must
/// be used instead.
pub fn next_stronghold_no_biome(sh: &mut StrongholdIter) -> i32 {
    assert!(
        sh.mc.is_at_least(MCVersion::V1_19),
        "next_stronghold_no_biome requires MC ≥ 1.19.4 (got {:?})",
        sh.mc
    );

    // Advance the Java RNG once (cubiomes' `nextLong(&sh->rnds)`).
    let _ = sh.rnds.next_long();
    sh.pos = sh.nextapprox;
    advance_iter_state(sh);
    if sh.mc.is_at_least(MCVersion::V1_9) {
        128 - (sh.index - 1)
    } else {
        3 - (sh.index - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_first_stronghold_deterministic() {
        let (a, _) = init_first_stronghold(MCVersion::V1_18, 0xdead_beef);
        let (b, _) = init_first_stronghold(MCVersion::V1_18, 0xdead_beef);
        assert_eq!(a, b);
    }

    #[test]
    fn first_stronghold_is_near_4_32_chunks_for_1_9_plus() {
        // For 1.9+: dist = 4*32 + (next_double - 0.5)*32*2.5 ∈ [88, 168] chunks.
        // After multiplication by 16 + 8 offset, block distance from origin is
        // roughly 1400..2700.
        let (p, _) = init_first_stronghold(MCVersion::V1_18, 12345);
        let d = (f64::from(p.x).powi(2) + f64::from(p.z).powi(2)).sqrt();
        assert!(
            d > 1000.0 && d < 3000.0,
            "unexpected first stronghold distance {d}"
        );
    }
}
