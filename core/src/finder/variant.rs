//! Structure variant data — `getVariant` partial port.
//!
//! Bit-exact port of the `Village` + `Bastion` arms of cubiomes'
//! `getVariant` from `finders.c`. The full function covers
//! `Village`, `Bastion`, `Ancient_City`, `Ruined_Portal`,
//! `Monument`, `Igloo`, `Geode`, and `End_City`; this commit ships
//! just the `Village` + `Bastion` arms, which is what
//! `is_viable_structure_pos`'s 1.18+ Overworld and 1.18+ Nether
//! paths consume. The remaining arms can be added on demand.

#![allow(
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::items_after_statements,
    clippy::unit_arg,
    clippy::struct_excessive_bools,
    clippy::doc_markdown
)]

use crate::finder::StructureType;
use crate::finder::population_seed::{chunk_generate_rng, get_population_seed};
use crate::finder::viability::is_viable_feature_biome;
use crate::mc_version::MCVersion;
use crate::rng::JavaRng;

/// Mirrors cubiomes' `STRUCT(StructureVariant)`. Fields not used
/// by a given arm default to `false`/`0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StructureVariant {
    /// Abandoned (zombie village) flag.
    pub abandoned: bool,
    /// Giant portal variant (Ruined_Portal).
    pub giant: bool,
    /// Underground portal (Ruined_Portal).
    pub underground: bool,
    /// Air-pocket portal (Ruined_Portal).
    pub airpocket: bool,
    /// Igloo basement flag.
    pub basement: bool,
    /// Geode "cracked" flag.
    pub cracked: bool,
    /// Geode size (and Igloo middle-piece count).
    pub size: u8,
    /// Starting piece index (cubiomes' `start`, sentinel `255`
    /// (uint8_t -1) until the structure-specific arm assigns it).
    pub start: u8,
    /// Biome variant ID (e.g. cubiomes encodes meadow→plains for
    /// Village). `-1` means unassigned.
    pub biome: i16,
    /// Rotation: 0=identity, 1=cw90, 2=cw180, 3=ccw90.
    pub rotation: u8,
    /// Mirror flag (Igloo, Desert_Pyramid, Jungle_Temple, Swamp_Hut).
    pub mirror: u8,
    /// Bounding-box offset relative to the chunk origin.
    pub x: i16,
    /// Bounding-box origin Y (Bastion: 0; Village: 320 sentinel).
    pub y: i16,
    /// Bounding-box offset relative to the chunk origin.
    pub z: i16,
    /// Bounding-box dimensions.
    pub sx: i16,
    /// Bounding-box dimensions.
    pub sy: i16,
    /// Bounding-box dimensions.
    pub sz: i16,
}

/// `getVariant(sv, structType, mc, seed, x, z, biomeID)` — return
/// the structure variant for the placement at block-coords `(x, z)`.
///
/// Returns `Some(variant)` on success, `None` for unsupported
/// `(structureType, mc)` combinations or biomes that fail the
/// Village viability gate. Bastion ignores the `biome_id` argument.
#[must_use]
#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
pub fn get_variant(
    structure_type: StructureType,
    mc: MCVersion,
    seed: u64,
    x: i32,
    z: i32,
    biome_id: i32,
) -> Option<StructureVariant> {
    use StructureType::*;
    let mut r = StructureVariant {
        start: u8::MAX, // cubiomes: `r->start = -1` (uint8_t wraps to 255)
        biome: -1,
        y: 320,
        ..Default::default()
    };
    let mut rng = chunk_generate_rng(seed, x >> 4, z >> 4);

    match structure_type {
        Village => get_variant_village(&mut r, &mut rng, mc, biome_id, x, z),
        Bastion => Some(get_variant_bastion(&mut r, &mut rng, mc, x, z)),
        AncientCity => Some(get_variant_ancient_city(&mut r, &mut rng, x, z)),
        TrialChambers => Some(get_variant_trial_chambers(&mut r, &mut rng)),
        Monument => Some(get_variant_monument(&mut r)),
        DesertPyramid | JungleTemple | SwampHut => {
            Some(get_variant_temple(&mut r, &mut rng, mc, structure_type))
        }
        Igloo => Some(get_variant_igloo(&mut r, &mut rng, mc, seed, x, z)),
        _ => None,
    }
    .map(|()| r)
}

fn get_variant_monument(r: &mut StructureVariant) {
    r.x = -29;
    r.z = -29;
    r.sx = 58;
    r.sz = 58;
}

fn get_variant_temple(
    r: &mut StructureVariant,
    rng: &mut JavaRng,
    mc: MCVersion,
    structure_type: StructureType,
) {
    use StructureType::*;
    let (sx, sy, sz): (i16, i16, i16) = match structure_type {
        DesertPyramid => (21, 15, 21),
        JungleTemple => (12, 10, 15),
        SwampHut => (7, 7, 9),
        _ => unreachable!(),
    };
    r.sy = sy;
    if mc.is_before(MCVersion::V1_20) {
        // Pre-1.20: no rotation roll, raw size.
        r.sx = sx;
        r.sz = sz;
        return;
    }
    // 1.20+: orientation = nextInt(4) → rotation + mirror + swapped size.
    match rng.next_int(4) {
        0 => {
            r.rotation = 0;
            r.mirror = 0;
            r.sx = sx;
            r.sz = sz;
        }
        1 => {
            r.rotation = 1;
            r.mirror = 0;
            r.sx = sz;
            r.sz = sx;
        }
        2 => {
            r.rotation = 0;
            r.mirror = 1;
            r.sx = sx;
            r.sz = sz;
        }
        3 => {
            r.rotation = 1;
            r.mirror = 1;
            r.sx = sz;
            r.sz = sx;
        }
        _ => unreachable!(),
    }
}

fn get_variant_igloo(
    r: &mut StructureVariant,
    rng: &mut JavaRng,
    mc: MCVersion,
    seed: u64,
    x: i32,
    z: i32,
) {
    if mc.is_before(MCVersion::V1_13) {
        // Pre-1.13: re-seed off the population seed of the
        // *previous* chunk.
        let pop = get_population_seed(mc, seed, (x >> 4) - 1, (z >> 4) - 1);
        *rng = JavaRng::new(pop);
    }
    r.rotation = rng.next_int(4) as u8;
    r.basement = rng.next_double() < 0.5;
    r.size = (rng.next_int(8) + 4) as u8;
    let (sx, sy, sz): (i16, i16, i16) = (7, 5, 8);
    r.sy = sy;
    match r.rotation {
        0 => {
            r.rotation = 0;
            r.mirror = 0;
            r.sx = sx;
            r.sz = sz;
        }
        1 => {
            r.rotation = 1;
            r.mirror = 0;
            r.sx = sz;
            r.sz = sx;
        }
        2 => {
            r.rotation = 0;
            r.mirror = 1;
            r.sx = sx;
            r.sz = sz;
        }
        3 => {
            r.rotation = 1;
            r.mirror = 1;
            r.sx = sz;
            r.sz = sx;
        }
        _ => unreachable!(),
    }
}

fn get_variant_ancient_city(r: &mut StructureVariant, rng: &mut JavaRng, mut x: i32, mut z: i32) {
    r.rotation = rng.next_int(4) as u8;
    r.start = 1 + rng.next_int(3) as u8; // city_center_1..3
    let mut sx: i16 = 18;
    let sy: i16 = 31;
    let mut sz: i16 = 41;
    // First rotation block computes the "raw" position via the
    // (x>0) / (x<0) sign trick.
    match r.rotation {
        0 => {
            x = -i32::from(x > 0);
            z = -i32::from(z > 0);
            r.sx = sx;
            r.sz = sz;
        }
        1 => {
            x = i32::from(x < 0) - sz as i32;
            z = -i32::from(z > 0);
            r.sx = sz;
            r.sz = sx;
        }
        2 => {
            x = i32::from(x < 0) - sx as i32;
            z = i32::from(z < 0) - sz as i32;
            r.sx = sx;
            r.sz = sz;
        }
        3 => {
            x = -i32::from(x > 0);
            z = i32::from(z < 0) - sx as i32;
            r.sx = sz;
            r.sz = sx;
        }
        _ => unreachable!(),
    }
    // Second rotation block uses city_anchor (sx=13, sz=20).
    sx = 13;
    sz = 20;
    match r.rotation {
        0 => {
            r.x = (x - sx as i32) as i16;
            r.z = (z - sz as i32) as i16;
        }
        1 => {
            r.x = (x + sz as i32) as i16;
            r.z = (z - sx as i32) as i16;
        }
        2 => {
            r.x = (x + sx as i32) as i16;
            r.z = (z + sz as i32) as i16;
        }
        3 => {
            r.x = (x - sz as i32) as i16;
            r.z = (z + sx as i32) as i16;
        }
        _ => unreachable!(),
    }
    r.y = -27;
    r.sy = sy;
}

fn get_variant_trial_chambers(r: &mut StructureVariant, rng: &mut JavaRng) {
    r.y = (rng.next_int(1 + 20) + -40) as i16;
    r.rotation = rng.next_int(4) as u8;
    r.start = rng.next_int(2) as u8;
    r.sx = 19;
    r.sy = 20;
    r.sz = 19;
    match r.rotation {
        0 => {}
        1 => {
            r.x = 1 - r.sz;
            r.z = 0;
        }
        2 => {
            r.x = 1 - r.sx;
            r.z = 1 - r.sz;
        }
        3 => {
            r.x = 0;
            r.z = 1 - r.sx;
        }
        _ => unreachable!(),
    }
}

fn get_variant_village(
    r: &mut StructureVariant,
    rng: &mut JavaRng,
    mc: MCVersion,
    biome_id: i32,
    x: i32,
    z: i32,
) -> Option<()> {
    if mc.is_before(MCVersion::V1_10) {
        return None;
    }
    if !is_viable_feature_biome(mc, StructureType::Village, biome_id) {
        return None;
    }
    if mc.is_before(MCVersion::V1_14) {
        // 1.10 - 1.13: random abandoned check.
        let skip = if mc == MCVersion::V1_13 { 10 } else { 11 };
        rng.skip_n(skip);
        r.abandoned = rng.next_int(50) == 0;
        return Some(());
    }

    r.biome = biome_id as i16;
    r.rotation = rng.next_int(4) as u8;

    // Effective biome — `meadow` falls through to `plains` for the
    // variant table.
    const PLAINS: i32 = 1;
    const DESERT: i32 = 2;
    const SAVANNA: i32 = 35;
    const TAIGA: i32 = 5;
    const SNOWY_TUNDRA: i32 = 12;
    const MEADOW: i32 = 177;
    let key_biome = if biome_id == MEADOW {
        r.biome = PLAINS as i16;
        PLAINS
    } else {
        biome_id
    };

    let (sx, sy, sz);
    match key_biome {
        PLAINS => {
            let t = rng.next_int(204);
            if t < 50 {
                r.start = 0;
                (sx, sy, sz) = (9, 4, 9);
            } else if t < 100 {
                r.start = 1;
                (sx, sy, sz) = (10, 7, 10);
            } else if t < 150 {
                r.start = 2;
                (sx, sy, sz) = (8, 5, 15);
            } else if t < 200 {
                r.start = 3;
                (sx, sy, sz) = (11, 9, 11);
            } else if t < 201 {
                r.start = 0;
                (sx, sy, sz) = (9, 4, 9);
                r.abandoned = true;
            } else if t < 202 {
                r.start = 1;
                (sx, sy, sz) = (10, 7, 10);
                r.abandoned = true;
            } else if t < 203 {
                r.start = 2;
                (sx, sy, sz) = (8, 5, 15);
                r.abandoned = true;
            } else {
                r.start = 3;
                (sx, sy, sz) = (11, 9, 11);
                r.abandoned = true;
            }
        }
        DESERT => {
            let t = rng.next_int(250);
            if t < 98 {
                r.start = 1;
                (sx, sy, sz) = (17, 6, 9);
            } else if t < 196 {
                r.start = 2;
                (sx, sy, sz) = (12, 6, 12);
            } else if t < 245 {
                r.start = 3;
                (sx, sy, sz) = (15, 6, 15);
            } else if t < 247 {
                r.start = 1;
                (sx, sy, sz) = (17, 6, 9);
                r.abandoned = true;
            } else if t < 249 {
                r.start = 2;
                (sx, sy, sz) = (12, 6, 12);
                r.abandoned = true;
            } else {
                r.start = 3;
                (sx, sy, sz) = (15, 6, 15);
                r.abandoned = true;
            }
        }
        SAVANNA => {
            let t = rng.next_int(459);
            if t < 100 {
                r.start = 1;
                (sx, sy, sz) = (14, 5, 12);
            } else if t < 150 {
                r.start = 2;
                (sx, sy, sz) = (11, 6, 11);
            } else if t < 300 {
                r.start = 3;
                (sx, sy, sz) = (9, 6, 11);
            } else if t < 450 {
                r.start = 4;
                (sx, sy, sz) = (9, 6, 9);
            } else if t < 452 {
                r.start = 1;
                (sx, sy, sz) = (14, 5, 12);
                r.abandoned = true;
            } else if t < 453 {
                r.start = 2;
                (sx, sy, sz) = (11, 6, 11);
                r.abandoned = true;
            } else if t < 456 {
                r.start = 3;
                (sx, sy, sz) = (9, 6, 11);
                r.abandoned = true;
            } else {
                r.start = 4;
                (sx, sy, sz) = (9, 6, 9);
                r.abandoned = true;
            }
        }
        TAIGA => {
            let t = rng.next_int(100);
            if t < 49 {
                r.start = 1;
                (sx, sy, sz) = (22, 3, 18);
            } else if t < 98 {
                r.start = 2;
                (sx, sy, sz) = (9, 7, 9);
            } else if t < 99 {
                r.start = 1;
                (sx, sy, sz) = (22, 3, 18);
                r.abandoned = true;
            } else {
                r.start = 2;
                (sx, sy, sz) = (9, 7, 9);
                r.abandoned = true;
            }
        }
        SNOWY_TUNDRA => {
            let t = rng.next_int(306);
            if t < 100 {
                r.start = 1;
                (sx, sy, sz) = (12, 8, 8);
            } else if t < 150 {
                r.start = 2;
                (sx, sy, sz) = (11, 5, 9);
            } else if t < 300 {
                r.start = 3;
                (sx, sy, sz) = (7, 7, 7);
            } else if t < 302 {
                r.start = 1;
                (sx, sy, sz) = (12, 8, 8);
                r.abandoned = true;
            } else if t < 303 {
                r.start = 2;
                (sx, sy, sz) = (11, 5, 9);
                r.abandoned = true;
            } else {
                r.start = 3;
                (sx, sy, sz) = (7, 7, 7);
                r.abandoned = true;
            }
        }
        _ => return None,
    }
    rotate_village_bastion(r, mc, x, z, sx, sy, sz);
    Some(())
}

fn get_variant_bastion(r: &mut StructureVariant, rng: &mut JavaRng, mc: MCVersion, x: i32, z: i32) {
    r.rotation = rng.next_int(4) as u8;
    r.start = rng.next_int(4) as u8;
    if mc == MCVersion::V1_16_1 {
        // Cubiomes: in 1.16.1 the start and rotation are swapped.
        std::mem::swap(&mut r.start, &mut r.rotation);
    }
    let (sx, sy, sz) = match r.start {
        0 => (46, 24, 46), // units/air_base
        1 => (30, 24, 48), // hoglin_stable/air_base
        2 => (38, 48, 38), // treasure/big_air_full
        3 => (16, 32, 32), // bridge/starting_pieces/entrance_base
        _ => unreachable!(),
    };
    rotate_village_bastion(r, mc, x, z, sx, sy, sz);
}

fn rotate_village_bastion(
    r: &mut StructureVariant,
    mc: MCVersion,
    x: i32,
    z: i32,
    sx: i16,
    sy: i16,
    sz: i16,
) {
    r.sy = sy;
    if mc.is_at_least(MCVersion::V1_18) {
        match r.rotation {
            0 => {
                r.x = 0;
                r.z = 0;
                r.sx = sx;
                r.sz = sz;
            }
            1 => {
                r.x = 1 - sz;
                r.z = 0;
                r.sx = sz;
                r.sz = sx;
            }
            2 => {
                r.x = 1 - sx;
                r.z = 1 - sz;
                r.sx = sx;
                r.sz = sz;
            }
            3 => {
                r.x = 0;
                r.z = 1 - sx;
                r.sx = sz;
                r.sz = sx;
            }
            _ => unreachable!(),
        }
    } else {
        // Pre-1.18: x<0 and z<0 contribute one block to the offset.
        let x_neg = i16::from(x < 0);
        let z_neg = i16::from(z < 0);
        match r.rotation {
            0 => {
                r.x = 0;
                r.z = 0;
                r.sx = sx;
                r.sz = sz;
            }
            1 => {
                r.x = x_neg - sz;
                r.z = 0;
                r.sx = sz;
                r.sz = sx;
            }
            2 => {
                r.x = x_neg - sx;
                r.z = z_neg - sz;
                r.sx = sx;
                r.sz = sz;
            }
            3 => {
                r.x = 0;
                r.z = z_neg - sx;
                r.sx = sz;
                r.sz = sx;
            }
            _ => unreachable!(),
        }
    }
}
