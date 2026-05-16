//! Structure locators — bit-exact ports of cubiomes' `finders.c` /
//! `finders.h`.
//!
//! This first M6 commit ships the structure-position layer: given a
//! world seed and region coordinates, find the in-region attempt
//! position for each structure type. The biome-viability checks
//! (`isViableStructurePos`) live in subsequent commits, as do
//! `Mineshaft`, `Bastion` (1.18+), and the decorator-feature
//! structures (`End_Gateway`, `End_Island`, `Desert_Well`, `Geode`).

#![allow(
    clippy::many_single_char_names,
    clippy::unreadable_literal,
    clippy::enum_glob_use
)]

pub mod end;
pub mod locate_biome;
pub mod mineshaft;
pub mod population_seed;
pub mod quadbase;
pub mod slime;
pub mod spawn;
pub mod stronghold;

pub use end::{EndIsland, get_end_islands, is_end_chunk_empty, map_end_island_height};
pub use locate_biome::{id_matches, locate_biome};
pub use mineshaft::get_mineshafts;
pub use population_seed::{chunk_generate_rng, get_population_seed};
pub use quadbase::{
    LOW20_QUAD_CLASSIC, LOW20_QUAD_HUT_BARELY, LOW20_QUAD_HUT_NORMAL, LOW20_QUAD_IDEAL, QuadHutCst,
    get_optimal_afk, get_quad_hut_cst, is_quad_base_feature_24, is_quad_base_feature_24_classic,
};
pub use slime::is_slime_chunk;
pub use spawn::{estimate_spawn, get_spawn};
pub use stronghold::{
    StrongholdIter, init_first_stronghold, is_stronghold_biome, next_stronghold,
    next_stronghold_no_biome,
};

use crate::mc_version::MCVersion;
use crate::rng::{JavaRng, Xoroshiro};

/// 2D block-coordinate position. Mirrors cubiomes' `STRUCT(Pos)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Pos {
    /// World-X (in blocks).
    pub x: i32,
    /// World-Z (in blocks).
    pub z: i32,
}

/// Enumeration of every structure cubiomes knows about. Ordinals
/// match cubiomes' `enum StructureType` exactly, so an `i32` cast
/// can be compared against fixture data 1:1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
#[allow(missing_docs)]
pub enum StructureType {
    Feature = 0,
    DesertPyramid = 1,
    JungleTemple = 2,
    SwampHut = 3,
    Igloo = 4,
    Village = 5,
    OceanRuin = 6,
    Shipwreck = 7,
    Monument = 8,
    Mansion = 9,
    Outpost = 10,
    RuinedPortal = 11,
    RuinedPortalN = 12,
    AncientCity = 13,
    Treasure = 14,
    Mineshaft = 15,
    DesertWell = 16,
    Geode = 17,
    Fortress = 18,
    Bastion = 19,
    EndCity = 20,
    EndGateway = 21,
    EndIsland = 22,
    TrailRuins = 23,
    TrialChambers = 24,
}

impl StructureType {
    /// Map back from the C enum ordinal.
    #[must_use]
    pub const fn from_ord(ord: i32) -> Option<Self> {
        Some(match ord {
            0 => Self::Feature,
            1 => Self::DesertPyramid,
            2 => Self::JungleTemple,
            3 => Self::SwampHut,
            4 => Self::Igloo,
            5 => Self::Village,
            6 => Self::OceanRuin,
            7 => Self::Shipwreck,
            8 => Self::Monument,
            9 => Self::Mansion,
            10 => Self::Outpost,
            11 => Self::RuinedPortal,
            12 => Self::RuinedPortalN,
            13 => Self::AncientCity,
            14 => Self::Treasure,
            15 => Self::Mineshaft,
            16 => Self::DesertWell,
            17 => Self::Geode,
            18 => Self::Fortress,
            19 => Self::Bastion,
            20 => Self::EndCity,
            21 => Self::EndGateway,
            22 => Self::EndIsland,
            23 => Self::TrailRuins,
            24 => Self::TrialChambers,
            _ => return None,
        })
    }
}

/// Per-version structure placement parameters. Mirrors cubiomes'
/// `STRUCT(StructureConfig)`.
#[derive(Debug, Clone, Copy)]
pub struct StructureConfig {
    /// Per-type salt mixed into the region seed.
    pub salt: i32,
    /// Region size in chunks.
    pub region_size: i8,
    /// Square sub-region used for the in-region uniform draw.
    pub chunk_range: i8,
    /// Structure type discriminant (cubiomes' `structType`).
    pub struct_type: u8,
    /// Dimension marker (0 = Overworld, -1 = Nether, 1 = End).
    pub dim: i8,
    /// Rarity for decorator-feature placements. Unused for region
    /// structures — those leave `rarity = 0.0`.
    pub rarity: f32,
}

const fn cfg(
    salt: i32,
    region_size: i8,
    chunk_range: i8,
    struct_type: StructureType,
    dim: i8,
    rarity: f32,
) -> StructureConfig {
    StructureConfig {
        salt,
        region_size,
        chunk_range,
        struct_type: struct_type as u8,
        dim,
        rarity,
    }
}

/// `getStructureConfig(type, mc, sconf)` — return the
/// version-specific [`StructureConfig`] or `None` if the type is
/// not supported on `mc`.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn get_structure_config(ty: StructureType, mc: MCVersion) -> Option<StructureConfig> {
    use StructureType::*;
    let cfg = match ty {
        Feature => {
            if mc.is_at_least(MCVersion::V1_13) {
                return None;
            }
            cfg(14357617, 32, 24, Feature, 0, 0.0)
        }
        DesertPyramid => {
            if !mc.is_at_least(MCVersion::V1_3) {
                return None;
            }
            // Desert pyramids share salt 14357617 across 1.3-1.12
            // and 1.13+; cubiomes leaves the two configs identical.
            cfg(14357617, 32, 24, DesertPyramid, 0, 0.0)
        }
        JungleTemple => {
            if !mc.is_at_least(MCVersion::V1_3) {
                return None;
            }
            if mc.is_at_least(MCVersion::V1_13) {
                cfg(14357619, 32, 24, JungleTemple, 0, 0.0)
            } else {
                cfg(14357617, 32, 24, JungleTemple, 0, 0.0)
            }
        }
        SwampHut => {
            if !mc.is_at_least(MCVersion::V1_4) {
                return None;
            }
            if mc.is_at_least(MCVersion::V1_13) {
                cfg(14357620, 32, 24, SwampHut, 0, 0.0)
            } else {
                cfg(14357617, 32, 24, SwampHut, 0, 0.0)
            }
        }
        Igloo => {
            if !mc.is_at_least(MCVersion::V1_9) {
                return None;
            }
            if mc.is_at_least(MCVersion::V1_13) {
                cfg(14357618, 32, 24, Igloo, 0, 0.0)
            } else {
                cfg(14357617, 32, 24, Igloo, 0, 0.0)
            }
        }
        Village => {
            if !mc.is_at_least(MCVersion::B1_8) {
                return None;
            }
            if mc.is_at_least(MCVersion::V1_18) {
                cfg(10387312, 34, 26, Village, 0, 0.0)
            } else {
                cfg(10387312, 32, 24, Village, 0, 0.0)
            }
        }
        OceanRuin => {
            if !mc.is_at_least(MCVersion::V1_13) {
                return None;
            }
            if mc.is_at_least(MCVersion::V1_16_1) {
                cfg(14357621, 20, 12, OceanRuin, 0, 0.0)
            } else {
                cfg(14357621, 16, 8, OceanRuin, 0, 0.0)
            }
        }
        Shipwreck => {
            if !mc.is_at_least(MCVersion::V1_13) {
                return None;
            }
            if mc.is_at_least(MCVersion::V1_16_1) {
                cfg(165745295, 24, 20, Shipwreck, 0, 0.0)
            } else {
                cfg(165745295, 16, 8, Shipwreck, 0, 0.0)
            }
        }
        Monument => {
            if !mc.is_at_least(MCVersion::V1_8) {
                return None;
            }
            cfg(10387313, 32, 27, Monument, 0, 0.0)
        }
        Mansion => {
            if !mc.is_at_least(MCVersion::V1_11) {
                return None;
            }
            cfg(10387319, 80, 60, Mansion, 0, 0.0)
        }
        Outpost => {
            if !mc.is_at_least(MCVersion::V1_14) {
                return None;
            }
            cfg(165745296, 32, 24, Outpost, 0, 0.0)
        }
        RuinedPortal => {
            if !mc.is_at_least(MCVersion::V1_16_1) {
                return None;
            }
            cfg(34222645, 40, 25, RuinedPortal, 0, 0.0)
        }
        RuinedPortalN => {
            if !mc.is_at_least(MCVersion::V1_16_1) {
                return None;
            }
            if mc.is_at_least(MCVersion::V1_18) {
                cfg(34222645, 40, 25, RuinedPortal, -1, 0.0)
            } else {
                cfg(34222645, 25, 15, RuinedPortalN, -1, 0.0)
            }
        }
        AncientCity => {
            if !mc.is_at_least(MCVersion::V1_19_2) {
                return None;
            }
            cfg(20083232, 24, 16, AncientCity, 0, 0.0)
        }
        Treasure => {
            if !mc.is_at_least(MCVersion::V1_13) {
                return None;
            }
            cfg(10387320, 1, 1, Treasure, 0, 0.0)
        }
        EndCity => {
            if !mc.is_at_least(MCVersion::V1_9) {
                return None;
            }
            cfg(10387313, 20, 9, EndCity, 1, 0.0)
        }
        Fortress => {
            if !mc.is_at_least(MCVersion::V1_0) {
                return None;
            }
            if mc.is_at_least(MCVersion::V1_16_1) {
                cfg(30084232, 27, 23, Fortress, -1, 0.0)
            } else {
                cfg(0, 16, 8, Fortress, -1, 0.0)
            }
        }
        Bastion => {
            if !mc.is_at_least(MCVersion::V1_16_1) {
                return None;
            }
            cfg(30084232, 27, 23, Bastion, -1, 0.0)
        }
        TrailRuins => {
            if !mc.is_at_least(MCVersion::V1_20) {
                return None;
            }
            cfg(83469867, 34, 26, TrailRuins, 0, 0.0)
        }
        TrialChambers => {
            if !mc.is_at_least(MCVersion::V1_21_1) {
                return None;
            }
            cfg(94251327, 34, 22, TrialChambers, 0, 0.0)
        }
        EndIsland => {
            // cubiomes: `return mc >= MC_1_13` (decorator features
            // are only supported from 1.13+).
            if !mc.is_at_least(MCVersion::V1_13) {
                return None;
            }
            if mc.is_at_least(MCVersion::V1_17) {
                // s_end_island = { 0, 1, 1, End_Island, DIM_END, 1.f/14 }
                cfg(0, 1, 1, EndIsland, 1, 1.0_f32 / 14.0)
            } else {
                // s_end_island_116 = { 0, 1, 1, End_Island, DIM_END, 14 }
                cfg(0, 1, 1, EndIsland, 1, 14.0)
            }
        }
        EndGateway => {
            // cubiomes: `return mc >= MC_1_13` (1.11/1.12 generate
            // gateways via a different RNG that isn't predictable).
            // Watch out: cubiomes' `MC_1_16` is the *latest* 1.16
            // (= 1.16.5), so `mc <= MC_1_16` covers 1.16.1..=1.16.5.
            // The Rust threshold for "starts being a 1.16.x" is
            // therefore `is_at_least(V1_16_1)`, not `V1_16`.
            if !mc.is_at_least(MCVersion::V1_13) {
                return None;
            }
            if mc.is_at_least(MCVersion::V1_18) {
                // s_end_gateway = { 40000, 1, 1, End_Gateway, DIM_END, 1.f/700 }
                cfg(40000, 1, 1, EndGateway, 1, 1.0_f32 / 700.0)
            } else if mc.is_at_least(MCVersion::V1_17) {
                // s_end_gateway_117 = { 40013, 1, 1, End_Gateway, DIM_END, 1.f/700 }
                cfg(40013, 1, 1, EndGateway, 1, 1.0_f32 / 700.0)
            } else if mc.is_at_least(MCVersion::V1_16_1) {
                // s_end_gateway_116 = { 40013, 1, 1, End_Gateway, DIM_END, 700 }
                cfg(40013, 1, 1, EndGateway, 1, 700.0)
            } else {
                // s_end_gateway_115 = { 30000, 1, 1, End_Gateway, DIM_END, 700 }
                cfg(30000, 1, 1, EndGateway, 1, 700.0)
            }
        }
        DesertWell => {
            // cubiomes: wells exist since 1.2 but cubiomes only
            // supports the decorator-feature predictor for 1.13+.
            // Same `V1_16_1` threshold subtlety as EndGateway.
            if !mc.is_at_least(MCVersion::V1_13) {
                return None;
            }
            if mc.is_at_least(MCVersion::V1_18) {
                // s_desert_well = { 40002, 1, 1, Desert_Well, 0, 1.f/1000 }
                cfg(40002, 1, 1, DesertWell, 0, 1.0_f32 / 1000.0)
            } else if mc.is_at_least(MCVersion::V1_16_1) {
                // s_desert_well_117 = { 40013, 1, 1, Desert_Well, 0, 1.f/1000 }
                cfg(40013, 1, 1, DesertWell, 0, 1.0_f32 / 1000.0)
            } else {
                // s_desert_well_115 = { 30010, 1, 1, Desert_Well, 0, 1.f/1000 }
                cfg(30010, 1, 1, DesertWell, 0, 1.0_f32 / 1000.0)
            }
        }
        Geode => {
            // cubiomes: `return mc >= MC_1_17` (geodes were added
            // in 1.17 with caves & cliffs).
            if !mc.is_at_least(MCVersion::V1_17) {
                return None;
            }
            if mc.is_at_least(MCVersion::V1_18) {
                // s_geode = { 20002, 1, 1, Geode, 0, 1.f/24 }
                cfg(20002, 1, 1, Geode, 0, 1.0_f32 / 24.0)
            } else {
                // s_geode_117 = { 20000, 1, 1, Geode, 0, 1.f/24 }
                cfg(20000, 1, 1, Geode, 0, 1.0_f32 / 24.0)
            }
        }
        // Mineshaft / Bastion 1.18+ land in follow-ups.
        Mineshaft => return None,
    };
    Some(cfg)
}

/// Transpose a base seed such that structure positions are moved by
/// `(reg_x, reg_z)` regions. Mirrors cubiomes' static inline
/// `moveStructure`.
#[must_use]
pub const fn move_structure(base_seed: u64, reg_x: i32, reg_z: i32) -> u64 {
    base_seed
        .wrapping_sub((reg_x as i64 as u64).wrapping_mul(341873128712))
        .wrapping_sub((reg_z as i64 as u64).wrapping_mul(132897987541))
        & 0xffff_ffff_ffff
}

const LCG_K: u64 = 0x0005_deec_e66d;
const LCG_M: u64 = (1 << 48) - 1;
const LCG_B: u64 = 0xb;

/// `getFeatureChunkInRegion(config, seed, reg_x, reg_z)` — uniform
/// in-region offset (0..`config.chunk_range`).
#[must_use]
pub fn get_feature_chunk_in_region(
    config: StructureConfig,
    seed: u64,
    reg_x: i32,
    reg_z: i32,
) -> Pos {
    let mut s = seed
        .wrapping_add((reg_x as i64 as u64).wrapping_mul(341873128712))
        .wrapping_add((reg_z as i64 as u64).wrapping_mul(132897987541))
        .wrapping_add(config.salt as i64 as u64);
    s ^= LCG_K;
    s = (s.wrapping_mul(LCG_K).wrapping_add(LCG_B)) & LCG_M;

    let r = config.chunk_range as u64;
    let (px, pz);
    if r & (r - 1) != 0 {
        // not a power of two — modulo path
        px = ((s >> 17) as u32 % r as u32) as i32;
        s = (s.wrapping_mul(LCG_K).wrapping_add(LCG_B)) & LCG_M;
        pz = ((s >> 17) as u32 % r as u32) as i32;
    } else {
        // power of two — cubiomes uses the Java RNG nextInt fast path.
        px = ((r.wrapping_mul(s >> 17)) >> 31) as i32;
        s = (s.wrapping_mul(LCG_K).wrapping_add(LCG_B)) & LCG_M;
        pz = ((r.wrapping_mul(s >> 17)) >> 31) as i32;
    }
    Pos { x: px, z: pz }
}

/// `getFeaturePos(config, seed, reg_x, reg_z)` — block-coordinate
/// position of the feature attempt.
#[must_use]
pub fn get_feature_pos(config: StructureConfig, seed: u64, reg_x: i32, reg_z: i32) -> Pos {
    let p = get_feature_chunk_in_region(config, seed, reg_x, reg_z);
    Pos {
        x: ((reg_x as i64 * config.region_size as i64 + p.x as i64) << 4) as i32,
        z: ((reg_z as i64 * config.region_size as i64 + p.z as i64) << 4) as i32,
    }
}

/// `getLargeStructureChunkInRegion`.
#[must_use]
pub fn get_large_structure_chunk_in_region(
    config: StructureConfig,
    seed: u64,
    reg_x: i32,
    reg_z: i32,
) -> Pos {
    let mut s = seed
        .wrapping_add((reg_x as i64 as u64).wrapping_mul(341873128712))
        .wrapping_add((reg_z as i64 as u64).wrapping_mul(132897987541))
        .wrapping_add(config.salt as i64 as u64);
    s ^= LCG_K;

    let cr = config.chunk_range as u32;
    s = (s.wrapping_mul(LCG_K).wrapping_add(LCG_B)) & LCG_M;
    let mut px = (s >> 17) as u32 % cr;
    s = (s.wrapping_mul(LCG_K).wrapping_add(LCG_B)) & LCG_M;
    px += (s >> 17) as u32 % cr;

    s = (s.wrapping_mul(LCG_K).wrapping_add(LCG_B)) & LCG_M;
    let mut pz = (s >> 17) as u32 % cr;
    s = (s.wrapping_mul(LCG_K).wrapping_add(LCG_B)) & LCG_M;
    pz += (s >> 17) as u32 % cr;

    Pos {
        x: (px >> 1) as i32,
        z: (pz >> 1) as i32,
    }
}

/// `getLargeStructurePos`.
#[must_use]
pub fn get_large_structure_pos(config: StructureConfig, seed: u64, reg_x: i32, reg_z: i32) -> Pos {
    let p = get_large_structure_chunk_in_region(config, seed, reg_x, reg_z);
    Pos {
        x: ((reg_x as i64 * config.region_size as i64 + p.x as i64) << 4) as i32,
        z: ((reg_z as i64 * config.region_size as i64 + p.z as i64) << 4) as i32,
    }
}

/// `setAttemptSeed` — cubiomes' helper that mutates `s` into the
/// chunk-attempt RNG state used by Outpost.
fn set_attempt_seed(s: u64, cx: i32, cz: i32) -> JavaRng {
    let value = s ^ ((cx >> 4) as i64 as u64) ^ (((cz >> 4) as i64 as u64) << 4);
    let mut rng = JavaRng::new(value);
    rng.next(31);
    rng
}

/// `getRegPos` — mutating helper used by `Fortress` / `Bastion`.
fn get_reg_pos(seed: u64, reg_x: i32, reg_z: i32, config: StructureConfig) -> (Pos, JavaRng) {
    let value = (reg_x as i64 as u64)
        .wrapping_mul(341873128712)
        .wrapping_add((reg_z as i64 as u64).wrapping_mul(132897987541))
        .wrapping_add(seed)
        .wrapping_add(config.salt as i64 as u64);
    let mut rng = JavaRng::new(value);
    let x_chunks = rng.next_int(config.chunk_range as i32);
    let z_chunks = rng.next_int(config.chunk_range as i32);
    let pos = Pos {
        x: ((reg_x as i64 * config.region_size as i64 + x_chunks as i64) << 4) as i32,
        z: ((reg_z as i64 * config.region_size as i64 + z_chunks as i64) << 4) as i32,
    };
    (pos, rng)
}

/// `getStructurePos(type, mc, seed, reg_x, reg_z)` — return the
/// validated structure position in the given region, or `None` if
/// no structure is placed there. Single-attempt structures
/// (`Mineshaft`, `Bastion` on 1.18+, decorator features) are not
/// yet supported and return `None`.
#[must_use]
pub fn get_structure_pos(
    ty: StructureType,
    mc: MCVersion,
    seed: u64,
    reg_x: i32,
    reg_z: i32,
) -> Option<Pos> {
    use StructureType::*;
    let config = get_structure_config(ty, mc)?;

    match ty {
        Feature | DesertPyramid | JungleTemple | SwampHut | Igloo | Village | OceanRuin
        | Shipwreck | RuinedPortal | RuinedPortalN | AncientCity | TrailRuins | TrialChambers => {
            Some(get_feature_pos(config, seed, reg_x, reg_z))
        }

        Monument | Mansion => Some(get_large_structure_pos(config, seed, reg_x, reg_z)),

        EndCity => {
            let p = get_large_structure_pos(config, seed, reg_x, reg_z);
            // cubiomes: only valid if distance² ≥ 1008²
            if (p.x as i64 * p.x as i64 + p.z as i64 * p.z as i64) >= 1008 * 1008 {
                Some(p)
            } else {
                None
            }
        }

        Outpost => {
            let p = get_feature_pos(config, seed, reg_x, reg_z);
            // Outpost's secondary RNG check.
            let mut rng = set_attempt_seed(seed, p.x >> 4, p.z >> 4);
            if rng.next_int(5) == 0 { Some(p) } else { None }
        }

        Treasure => {
            let px = reg_x * 16 + 9;
            let pz = reg_z * 16 + 9;
            let mut s = (reg_x as i64 as u64)
                .wrapping_mul(341873128712)
                .wrapping_add((reg_z as i64 as u64).wrapping_mul(132897987541))
                .wrapping_add(seed)
                .wrapping_add(config.salt as i64 as u64);
            // setSeed(&seed, seed)
            let mut rng = JavaRng::new(s);
            // cubiomes' `nextFloat(&seed) < 0.01`
            let _ = &mut s; // silence
            if rng.next_float() < 0.01 {
                Some(Pos { x: px, z: pz })
            } else {
                None
            }
        }

        Fortress => {
            if mc.is_at_least(MCVersion::V1_18) {
                Some(get_feature_pos(config, seed, reg_x, reg_z))
            } else if mc.is_at_least(MCVersion::V1_16_1) {
                let (p, mut rng) = get_reg_pos(seed, reg_x, reg_z, config);
                if rng.next_int(5) < 2 { Some(p) } else { None }
            } else {
                let mut rng = set_attempt_seed(seed, reg_x * 16, reg_z * 16);
                let valid = rng.next_int(3) == 0;
                let nx = rng.next_int(8);
                let nz = rng.next_int(8);
                let p = Pos {
                    x: (reg_x * 16 + nx + 4) * 16,
                    z: (reg_z * 16 + nz + 4) * 16,
                };
                if valid { Some(p) } else { None }
            }
        }

        Bastion => {
            if mc.is_at_least(MCVersion::V1_18) {
                // 1.18+: standard feature pos, then chunk-RNG check.
                let p = get_feature_pos(config, seed, reg_x, reg_z);
                let mut rng = population_seed::chunk_generate_rng(seed, p.x >> 4, p.z >> 4);
                if rng.next_int(5) >= 2 { Some(p) } else { None }
            } else {
                let (p, mut rng) = get_reg_pos(seed, reg_x, reg_z, config);
                if rng.next_int(5) >= 2 { Some(p) } else { None }
            }
        }

        EndGateway | EndIsland | DesertWell | Geode => {
            decorator_attempt_pos(config, mc, seed, reg_x, reg_z)
        }

        Mineshaft => None,
    }
}

/// Shared decorator-feature placement (`End_Gateway`, `End_Island`,
/// `Desert_Well`, `Geode`). Mirrors the common case-arm in cubiomes'
/// `getStructurePos`: `regX`/`regZ` are interpreted as *chunk*
/// coordinates (since `region_size = chunk_range = 1`).
///
/// The roll uses `getPopulationSeed + salt` as the per-feature seed,
/// then on MC ≥ 1.18 runs the Xoroshiro float-rarity check followed
/// by two `xNextIntJ(16)` offset draws. On older versions the same
/// applies via Java RNG, with an extra branch when `rarity ≥ 1.0`
/// to use `nextInt((int)rarity) != 0` instead of `nextFloat`.
fn decorator_attempt_pos(
    config: StructureConfig,
    mc: MCVersion,
    seed: u64,
    reg_x: i32,
    reg_z: i32,
) -> Option<Pos> {
    let bx = reg_x.wrapping_mul(16);
    let bz = reg_z.wrapping_mul(16);
    let pop = population_seed::get_population_seed(mc, seed, bx, bz);
    let salt_u = config.salt as i64 as u64;
    if mc.is_at_least(MCVersion::V1_18) {
        let mut xr = Xoroshiro::new(pop.wrapping_add(salt_u));
        if xr.next_float() >= config.rarity {
            return None;
        }
        let ox = xr.next_int_j(16);
        let oz = xr.next_int_j(16);
        Some(Pos {
            x: bx + ox,
            z: bz + oz,
        })
    } else {
        let mut rng = JavaRng::new(pop.wrapping_add(salt_u));
        if config.rarity < 1.0 {
            if rng.next_float() >= config.rarity {
                return None;
            }
        } else if rng.next_int(config.rarity as i32) != 0 {
            return None;
        }
        let ox = rng.next_int(16);
        let oz = rng.next_int(16);
        Some(Pos {
            x: bx + ox,
            z: bz + oz,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_structure_round_trips() {
        let s = 0x1234_5678_9abc;
        let m = move_structure(s, 3, -2);
        let back = move_structure(m, -3, 2);
        // moveStructure is linear modulo 2^48; round trip recovers
        // the low 48 bits of the original seed.
        assert_eq!(back, s & 0xffff_ffff_ffff);
    }

    #[test]
    fn config_lookup_respects_mc_lower_bounds() {
        assert!(get_structure_config(StructureType::Village, MCVersion::B1_7).is_none());
        assert!(get_structure_config(StructureType::Village, MCVersion::B1_8).is_some());
        assert!(get_structure_config(StructureType::Bastion, MCVersion::V1_15).is_none());
        assert!(get_structure_config(StructureType::Bastion, MCVersion::V1_16_1).is_some());
    }

    #[test]
    fn village_pos_deterministic() {
        let a = get_structure_pos(StructureType::Village, MCVersion::V1_18, 0xdead_beef, 0, 0);
        let b = get_structure_pos(StructureType::Village, MCVersion::V1_18, 0xdead_beef, 0, 0);
        assert_eq!(a, b);
    }
}
