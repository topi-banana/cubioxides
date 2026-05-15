//! The unified MC-version + dimension biome generator.
//!
//! Bit-exact Rust port of cubiomes' `Generator` API:
//! `setupGenerator(mc, flags)` → [`Generator::new`],
//! `applySeed(dim, seed)` → [`Generator::apply_seed`], and
//! `getBiomeAt(scale, x, y, z)` → [`Generator::biome_at`]. The C
//! `union` over `(LayerStack, BiomeNoise, BiomeNoiseBeta)` is
//! replaced by three nullable boxed fields plus an
//! [`OverworldKind`] tag — the active variant is chosen at
//! construction time and never changes.
//!
//! This commit ships the single-cell `biome_at` API. The full
//! 3D-range `gen_biomes` (cubiomes' `genBiomes(cache, range)`) lands
//! in a follow-up.

use crate::biome::Biome;
use crate::biomenoise::{BiomeNoise, BiomeNoiseBeta, EndNoise, NetherNoise};
use crate::layer::ops::voronoi::voronoi_access_3d;
use crate::layer::{LayerId, LayerStack, gen_area, set_layer_seed, setup_layer_stack};
use crate::mc_version::{Dimension, MCVersion};
use crate::sha::voronoi_sha;

/// Cubiomes' `LARGE_BIOMES` flag (1.3+).
pub const LARGE_BIOMES: u32 = 0x1;
/// Cubiomes' `NO_BETA_OCEAN` flag.
pub const NO_BETA_OCEAN: u32 = 0x2;
/// Cubiomes' `FORCE_OCEAN_VARIANTS` flag.
pub const FORCE_OCEAN_VARIANTS: u32 = 0x4;

/// Which Overworld generation pipeline applies to the configured MC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverworldKind {
    /// Beta 1.7 and earlier — [`BiomeNoiseBeta`] + 64×64 lookup.
    Beta,
    /// Beta 1.8 through 1.17 — [`LayerStack`] DAG.
    Layered,
    /// 1.18+ — [`BiomeNoise`] (climate noise + spline + decision tree).
    Modern,
}

/// Unified biome generator. Construct with [`Self::new(mc, flags)`],
/// then call [`Self::apply_seed`] before any sampling.
#[derive(Debug, Clone)]
pub struct Generator {
    /// Configured Minecraft version.
    pub mc: MCVersion,
    /// Dimension last selected by [`Self::apply_seed`] (`None` until
    /// the first call).
    pub dim: Option<Dimension>,
    /// `LARGE_BIOMES` / `NO_BETA_OCEAN` / `FORCE_OCEAN_VARIANTS`
    /// bitmask. See the constants in this module.
    pub flags: u32,
    /// World seed last passed to [`Self::apply_seed`].
    pub seed: u64,
    /// Voronoi salt (cubiomes' `g->sha`). Populated for 1.15+ — see
    /// [`Self::apply_seed`].
    pub sha: u64,
    /// Selected Overworld pipeline.
    pub overworld_kind: OverworldKind,
    /// Layer DAG (only populated when [`Self::overworld_kind`] is
    /// `Layered`).
    pub layer_stack: Option<Box<LayerStack>>,
    /// 1.18+ biome noise (populated by [`Self::apply_seed`] when the
    /// dimension is Overworld and the MC is 1.18+).
    pub biome_noise: Option<Box<BiomeNoise>>,
    /// Pre-Beta-1.8 biome noise.
    pub biome_noise_beta: Option<BiomeNoiseBeta>,
    /// Nether noise (populated when dim = Nether and MC ≥ 1.16.1).
    pub nether: Option<NetherNoise>,
    /// End noise (populated when dim = End and MC ≥ 1.9).
    pub end: Option<EndNoise>,
}

impl Generator {
    /// Cubiomes' `setupGenerator(g, mc, flags)`. Static state is
    /// initialised based on `mc`; seeding happens in
    /// [`Self::apply_seed`].
    #[must_use]
    pub fn new(mc: MCVersion, flags: u32) -> Self {
        let overworld_kind = if mc.is_at_least(MCVersion::V1_18) {
            OverworldKind::Modern
        } else if mc.is_at_least(MCVersion::B1_8) {
            OverworldKind::Layered
        } else {
            OverworldKind::Beta
        };

        let layer_stack = if matches!(overworld_kind, OverworldKind::Layered) {
            let mut stack = Box::new(LayerStack::new());
            setup_layer_stack(&mut stack, mc, flags & LARGE_BIOMES != 0);
            Some(stack)
        } else {
            None
        };

        Self {
            mc,
            dim: None,
            flags,
            seed: 0,
            sha: 0,
            overworld_kind,
            layer_stack,
            biome_noise: None,
            biome_noise_beta: None,
            nether: None,
            end: None,
        }
    }

    /// Cubiomes' `applySeed(g, dim, seed)`. Re-seeds the
    /// dimension-specific noise / layer state and recomputes
    /// `sha` (the Voronoi salt) when applicable.
    pub fn apply_seed(&mut self, dim: Dimension, seed: u64) {
        self.dim = Some(dim);
        self.seed = seed;
        self.sha = 0;

        match dim {
            Dimension::Overworld => match self.overworld_kind {
                OverworldKind::Beta => {
                    self.biome_noise_beta = Some(BiomeNoiseBeta::set_seed(seed));
                }
                OverworldKind::Layered => {
                    if let Some(stack) = &mut self.layer_stack {
                        let entry = stack.entry_1.expect("layered entry_1");
                        set_layer_seed(stack, entry, seed);
                    }
                }
                OverworldKind::Modern => {
                    let large = self.flags & LARGE_BIOMES != 0;
                    if let Some(bn) = &mut self.biome_noise {
                        bn.re_seed(seed, large);
                    } else {
                        self.biome_noise = Some(Box::new(BiomeNoise::new(self.mc, seed, large)));
                    }
                }
            },
            Dimension::Nether => {
                if self.mc.is_at_least(MCVersion::V1_16_1) {
                    self.nether = Some(NetherNoise::set_seed(seed));
                }
            }
            Dimension::End => {
                if self.mc.is_at_least(MCVersion::V1_9) {
                    self.end = Some(EndNoise::set_seed(self.mc, seed));
                }
            }
        }

        // sha — cubiomes' Voronoi salt. For 1.15+: layered OW uses
        // the layer-stack entry_1 startSalt (already computed via
        // set_layer_seed); everything else recomputes via voronoi_sha.
        if self.mc.is_at_least(MCVersion::V1_15) {
            self.sha =
                if dim == Dimension::Overworld && self.overworld_kind == OverworldKind::Layered {
                    let stack = self.layer_stack.as_ref().expect("layered stack");
                    let entry = stack.entry_1.expect("entry_1");
                    stack.node(entry).start_salt
                } else {
                    voronoi_sha(seed)
                };
        }
    }

    /// Cubiomes' `getBiomeAt(g, scale, x, y, z)` — return the biome
    /// at a single point. Supports `scale` ∈ {1, 4}; other scales
    /// land in the follow-up Range-based API.
    #[must_use]
    pub fn biome_at(&self, scale: i32, x: i32, y: i32, z: i32) -> Biome {
        let dim = self
            .dim
            .expect("Generator::biome_at: apply_seed must be called first");

        match dim {
            Dimension::Overworld => self.biome_at_overworld(scale, x, y, z),
            Dimension::Nether => self.biome_at_nether(scale, x, y, z),
            Dimension::End => self.biome_at_end(scale, x, y, z),
        }
    }

    fn biome_at_overworld(&self, scale: i32, x: i32, y: i32, z: i32) -> Biome {
        match self.overworld_kind {
            OverworldKind::Beta => {
                self.biome_noise_beta
                    .as_ref()
                    .expect("Beta noise seeded")
                    .sample(x, z)
                    .0
            }
            OverworldKind::Layered => {
                let stack = self.layer_stack.as_ref().expect("layer stack");
                let entry = layered_entry_for_scale(stack, scale)
                    .unwrap_or_else(|| panic!("unsupported scale {scale} for layered MC"));
                let mut out = [Biome::NONE; 1];
                gen_area(stack, entry, &mut out, x, z, 1, 1);
                out[0]
            }
            OverworldKind::Modern => {
                let bn = self.biome_noise.as_ref().expect("BiomeNoise seeded");
                let (sx, sy, sz) = match scale {
                    1 => voronoi_access_3d(self.sha, x, y, z),
                    4 => (x, y, z),
                    other => panic!("unsupported scale {other} for 1.18+ Overworld"),
                };
                let (id, _) = bn.sample(sx, sy, sz, 0);
                Biome(id)
            }
        }
    }

    fn biome_at_nether(&self, scale: i32, x: i32, y: i32, z: i32) -> Biome {
        if !self.mc.is_at_least(MCVersion::V1_16_1) {
            return Biome::NETHER_WASTES;
        }
        let nn = self.nether.as_ref().expect("Nether noise seeded");
        let (sx, sy, sz) = match scale {
            1 => voronoi_access_3d(self.sha, x, y, z),
            4 => (x, y, z),
            other => panic!("unsupported scale {other} for Nether"),
        };
        let (b, _) = nn.get_nether_biome(sx, sy, sz);
        b
    }

    fn biome_at_end(&self, scale: i32, x: i32, y: i32, z: i32) -> Biome {
        if !self.mc.is_at_least(MCVersion::V1_9) {
            return Biome::THE_END;
        }
        let en = self.end.as_ref().expect("End noise seeded");
        let mut out = [0i32; 1];
        match scale {
            1 => {
                // Voronoi → map_end at 1:4. cubiomes' pre-1.15 End
                // path actually uses `mapVoronoi114` with the
                // layer-salt-10 chunk seed, but matching that
                // bit-exactly requires re-deriving the salt; we
                // restrict scale=1 to MC ≥ 1.15 where the simpler
                // SHA-driven `voronoi_access_3d` applies.
                assert!(
                    self.mc.is_at_least(MCVersion::V1_15),
                    "Generator::biome_at: End scale=1 requires MC >= 1.15"
                );
                let (x4, _y4, z4) = voronoi_access_3d(self.sha, x, y, z);
                en.map_end(&mut out, x4, z4, 1, 1);
            }
            4 => en.map_end(&mut out, x, z, 1, 1),
            16 => en.map_end_biome(&mut out, x, z, 1, 1),
            other => panic!("unsupported scale {other} for End"),
        }
        Biome(out[0])
    }
}

fn layered_entry_for_scale(stack: &LayerStack, scale: i32) -> Option<LayerId> {
    match scale {
        1 => stack.entry_1,
        4 => stack.entry_4,
        16 => stack.entry_16,
        64 => stack.entry_64,
        256 => stack.entry_256,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_for_1_18_picks_modern() {
        let g = Generator::new(MCVersion::V1_18, 0);
        assert_eq!(g.overworld_kind, OverworldKind::Modern);
        assert!(g.layer_stack.is_none());
    }

    #[test]
    fn new_for_1_12_picks_layered() {
        let g = Generator::new(MCVersion::V1_12, 0);
        assert_eq!(g.overworld_kind, OverworldKind::Layered);
        assert!(g.layer_stack.is_some());
    }

    #[test]
    fn new_for_b1_7_picks_beta() {
        let g = Generator::new(MCVersion::B1_7, 0);
        assert_eq!(g.overworld_kind, OverworldKind::Beta);
    }

    #[test]
    fn biome_at_layered_1_12_deterministic() {
        let mut g = Generator::new(MCVersion::V1_12, 0);
        g.apply_seed(Dimension::Overworld, 0xdead_beef);
        let a = g.biome_at(4, 0, 64, 0);
        let b = g.biome_at(4, 0, 64, 0);
        assert_eq!(a, b);
    }

    #[test]
    fn biome_at_modern_1_18_deterministic() {
        let mut g = Generator::new(MCVersion::V1_18, 0);
        g.apply_seed(Dimension::Overworld, 0xdead_beef);
        let a = g.biome_at(4, 100, 64, 100);
        let b = g.biome_at(4, 100, 64, 100);
        assert_eq!(a, b);
    }
}
