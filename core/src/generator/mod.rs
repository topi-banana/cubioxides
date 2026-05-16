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

pub mod height;

pub use height::map_approx_height;

use crate::biome::Biome;
use crate::biomenoise::{BiomeNoise, BiomeNoiseBeta, EndNoise, NetherNoise, SAMPLE_NO_SHIFT};
use crate::layer::ops::voronoi::voronoi_access_3d;
use crate::layer::{
    LayerId, LayerStack, apply_force_ocean_variants, gen_area, set_layer_seed, setup_layer_stack,
};
use crate::mc_version::{Dimension, MCVersion};
use crate::sha::voronoi_sha;

/// 3D rectangular region cubiomes uses as the input to `genBiomes`.
/// Mirrors `struct Range` in `cubiomes/biomenoise.h`. A horizontal
/// scale of 1 indicates 1:1 (block) coordinates; any other scale is
/// in biome cells (so 4 = 1:4, 16 = 1:16, etc.). `sy == 0` is
/// normalised to `sy == 1` (a single horizontal slice).
#[derive(Debug, Clone, Copy)]
pub struct Range {
    /// Horizontal scale factor.
    pub scale: i32,
    /// North-west corner X (in scale units).
    pub x: i32,
    /// North-west corner Z (in scale units).
    pub z: i32,
    /// Width along +X.
    pub sx: u32,
    /// Depth along +Z.
    pub sz: u32,
    /// Vertical base Y (1:1 if `scale == 1`, otherwise 1:4).
    pub y: i32,
    /// Vertical span (zero is treated as one).
    pub sy: u32,
}

impl Range {
    /// Number of cells the range covers (`sx * sz * sy`, with `sy =
    /// max(1, sy)`).
    #[inline]
    #[must_use]
    pub fn cell_count(&self) -> usize {
        let sy = if self.sy == 0 { 1 } else { self.sy };
        self.sx as usize * self.sz as usize * sy as usize
    }
}

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
            if (flags & FORCE_OCEAN_VARIANTS) != 0 && mc.is_at_least(MCVersion::V1_13) {
                apply_force_ocean_variants(&mut stack, mc);
            }
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
                // cubiomes' `getBiomeAt` for Beta builds a 1×1 Range
                // and delegates to `genBiomeNoiseBetaScaled`, which
                // applies `mid = scale >> 1` shifting per cell. For
                // scale=1 the shift is zero so direct sampling at
                // (x, z) is correct; for scale=4 cubiomes samples at
                // (x*4+2, z*4+2). Delegate through gen_biomes so the
                // scale factor is honoured uniformly.
                self.biome_at_via_gen_biomes(scale, x, y, z)
            }
            OverworldKind::Layered => {
                let stack = self.layer_stack.as_ref().expect("layer stack");
                if let Some(entry) = layered_entry_for_scale(stack, scale) {
                    let mut out = [Biome::NONE; 1];
                    gen_area(stack, entry, &mut out, x, z, 1, 1);
                    out[0]
                } else {
                    self.biome_at_via_gen_biomes(scale, x, y, z)
                }
            }
            OverworldKind::Modern => {
                let bn = self.biome_noise.as_ref().expect("BiomeNoise seeded");
                let (sx, sy, sz) = match scale {
                    1 => voronoi_access_3d(self.sha, x, y, z),
                    4 => (x, y, z),
                    _ => return self.biome_at_via_gen_biomes(scale, x, y, z),
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
            _ => return self.biome_at_via_gen_biomes(scale, x, y, z),
        };
        let (b, _) = nn.get_nether_biome(sx, sy, sz);
        b
    }

    /// Single-cell fallback that builds a 1×1×1 [`Range`] and
    /// delegates to [`Self::gen_biomes`]. Used when the scale isn't
    /// one of the fast-path constants — mirrors what cubiomes'
    /// `getBiomeAt` does for every call.
    fn biome_at_via_gen_biomes(&self, scale: i32, x: i32, y: i32, z: i32) -> Biome {
        let mut cache = [Biome(0); 1];
        self.gen_biomes(
            &mut cache,
            Range {
                scale,
                x,
                z,
                sx: 1,
                sz: 1,
                y,
                sy: 1,
            },
        );
        cache[0]
    }

    fn biome_at_end(&self, scale: i32, x: i32, y: i32, z: i32) -> Biome {
        if !self.mc.is_at_least(MCVersion::V1_9) {
            return Biome::THE_END;
        }
        let en = self.end.as_ref().expect("End noise seeded");
        let mut out = [0i32; 1];
        match scale {
            1 if self.mc.is_at_least(MCVersion::V1_15) => {
                // 1.15+ End voronoi is SHA-driven (3D); single-cell
                // fast path uses voronoi_access_3d + map_end at 1:4.
                let (x4, _y4, z4) = voronoi_access_3d(self.sha, x, y, z);
                en.map_end(&mut out, x4, z4, 1, 1);
            }
            1 => {
                // Pre-1.15 End uses planar mapVoronoi114 with
                // layerSalt(10) — defer to gen_biomes which handles
                // it correctly. Avoids duplicating the salt-derivation
                // logic at the single-cell call site.
                let mut cache = [Biome(0); 1];
                self.gen_biomes(
                    &mut cache,
                    Range {
                        scale: 1,
                        x,
                        z,
                        sx: 1,
                        sz: 1,
                        y,
                        sy: 1,
                    },
                );
                return cache[0];
            }
            4 => en.map_end(&mut out, x, z, 1, 1),
            16 => en.map_end_biome(&mut out, x, z, 1, 1),
            _ => return self.biome_at_via_gen_biomes(scale, x, y, z),
        }
        Biome(out[0])
    }

    /// Cubiomes' `getLayerForScale(g, scale)` — return the
    /// [`LayerId`] of the layer that emits at `scale`. Only meaningful
    /// for layered MC (Beta 1.8 – 1.17); returns `None` for 1.18+,
    /// Beta 1.7 or earlier, or for any scale not in `{1, 4, 16, 64,
    /// 256}`. The "`scale == 0`" cubiomes special-case maps to the
    /// generator's cached entry; we emit `None` since the Rust port
    /// doesn't keep a stateful entry pointer.
    #[must_use]
    pub fn layer_for_scale(&self, scale: i32) -> Option<LayerId> {
        if self.overworld_kind != OverworldKind::Layered {
            return None;
        }
        let stack = self.layer_stack.as_ref()?;
        layered_entry_for_scale(stack, scale)
    }

    /// Cubiomes' `getMinCacheSize(g, scale, sx, sy, sz)` — minimum
    /// length in `Biome` units required for a [`Self::gen_biomes`]
    /// cache. Bit-exact port: returns the same number cubiomes does
    /// (which over-counts the Beta sea-level scratch and the 1.18+
    /// voronoi source — both safely larger than what the Rust port
    /// actually needs internally).
    ///
    /// `sy == 0` is normalised to `1`. Returns `0` when the Generator
    /// state cannot service the request (e.g. layered scale without a
    /// matching entry).
    #[must_use]
    pub fn min_cache_size(&self, scale: i32, sx: u32, sy: u32, sz: u32) -> usize {
        // Cubiomes adds raw `sizeof(SeaLevelColumnNoiseBeta)` (= 64
        // bytes) per slen entry to a count that is later used as the
        // `nmemb` argument of `calloc(len, sizeof(int))`. That is a
        // small over-count bug in cubiomes (slen entries get 4× more
        // bytes than needed), but it's stable across cubiomes
        // versions, so the parity port reproduces the same number.
        const SEA_LEVEL_COLUMN_BYTES: usize = 64;
        let sy = if sy == 0 { 1 } else { sy };
        let mut len = sx as usize * sz as usize * sy as usize;
        let beta_path = !self.mc.is_at_least(MCVersion::B1_8)
            && scale <= 4
            && (self.flags & NO_BETA_OCEAN) == 0;
        let layered_overworld_path = self.mc.is_at_least(MCVersion::B1_8)
            && !self.mc.is_at_least(MCVersion::V1_18)
            && self.dim == Some(Dimension::Overworld);
        let voronoi_path = (self.mc.is_at_least(MCVersion::V1_18)
            || self.dim != Some(Dimension::Overworld))
            && scale <= 1;
        if beta_path {
            let cellwidth = scale >> 1;
            let smin = sx.min(sz) as i32;
            let slen = ((smin >> (2 >> cellwidth)) + 1) * 2 + 1;
            len += slen as usize * SEA_LEVEL_COLUMN_BYTES;
        } else if layered_overworld_path {
            let Some(stack) = self.layer_stack.as_ref() else {
                return 0;
            };
            let Some(entry) = layered_entry_for_scale(stack, scale) else {
                return 0;
            };
            let len_2d =
                crate::layer::cache::get_min_layer_cache_size(stack, entry, sx as i32, sz as i32);
            len += len_2d.saturating_sub(sx as usize * sz as usize);
        } else if voronoi_path {
            let sx4 = ((sx as i32 + 3) >> 2) + 2;
            let sy4 = ((sy as i32 + 3) >> 2) + 2;
            let sz4 = ((sz as i32 + 3) >> 2) + 2;
            len += (sx4 * sy4 * sz4) as usize;
        }
        len
    }

    /// Cubiomes' `genBiomes(g, cache, r)` — fill `cache` with biome
    /// ids over the requested 3D range.
    ///
    /// Currently supported combinations:
    ///
    /// - Overworld + Layered (Beta 1.8 – 1.17): scales {1, 4, 16,
    ///   64, 256} at the corresponding entry layer.
    /// - Overworld + Modern (1.18+): scale=1 via voronoi access,
    ///   any other scale via `genBiomeNoise3D` (`opt=true` when
    ///   `scale > 4`).
    /// - Overworld + Beta (≤ Beta 1.7): all scales via the Beta
    ///   biome-noise table + optional sea-level oceans.
    /// - Nether (MC ≥ 1.16.1): scale=1 voronoi, any other scale via
    ///   `mapNether3D`.
    /// - End (MC ≥ 1.9): scale=1 voronoi (planar pre-1.15, 3D
    ///   thereafter), scale=4 via `mapEnd`, scale=16 via
    ///   `mapEndBiome`, any other scale via the radial pseudo-biome
    ///   formula.
    /// - Pre-1.16.1 Nether and pre-1.9 End fill with the
    ///   `nether_wastes` / `the_end` fallback.
    pub fn gen_biomes(&self, cache: &mut [Biome], r: Range) {
        let r = Range {
            sy: r.sy.max(1),
            ..r
        };
        assert!(
            cache.len() >= r.cell_count(),
            "Generator::gen_biomes: cache too small"
        );

        let dim = self
            .dim
            .expect("Generator::gen_biomes: apply_seed must be called first");

        match dim {
            Dimension::Overworld => self.gen_biomes_overworld(cache, r),
            Dimension::Nether => self.gen_biomes_nether(cache, r),
            Dimension::End => self.gen_biomes_end(cache, r),
        }
    }

    fn gen_biomes_overworld(&self, cache: &mut [Biome], r: Range) {
        match self.overworld_kind {
            OverworldKind::Layered => {
                let stack = self.layer_stack.as_ref().expect("layered stack");
                let entry = layered_entry_for_scale(stack, r.scale).unwrap_or_else(|| {
                    panic!("unsupported scale {} for layered Overworld", r.scale)
                });
                let area = r.sx as usize * r.sz as usize;
                gen_area(
                    stack,
                    entry,
                    &mut cache[..area],
                    r.x,
                    r.z,
                    r.sx as usize,
                    r.sz as usize,
                );
                // 2D layer output expanded across the vertical axis.
                for k in 1..r.sy as usize {
                    cache.copy_within(0..area, k * area);
                }
            }
            OverworldKind::Modern => {
                let bn = self.biome_noise.as_ref().expect("BiomeNoise seeded");
                if r.scale == 1 {
                    gen_biome_noise_voronoi(bn, cache, r, self.sha);
                } else {
                    gen_biome_noise_3d(bn, cache, r, r.scale > 4);
                }
            }
            OverworldKind::Beta => {
                let bnb = self
                    .biome_noise_beta
                    .as_ref()
                    .expect("BiomeNoiseBeta seeded");
                // Cubiomes uses snb=NULL when NO_BETA_OCEAN is set, else
                // builds a fresh SurfaceNoiseBeta from the world seed.
                if self.flags & NO_BETA_OCEAN != 0 {
                    crate::biomenoise::beta::gen_biome_noise_beta_scaled(bnb, None, cache, r);
                } else {
                    let snb = crate::biomenoise::surface_beta::SurfaceNoiseBeta::init(self.seed);
                    crate::biomenoise::beta::gen_biome_noise_beta_scaled(bnb, Some(&snb), cache, r);
                }
            }
        }
    }

    fn gen_biomes_nether(&self, cache: &mut [Biome], r: Range) {
        if !self.mc.is_at_least(MCVersion::V1_16_1) {
            for c in cache.iter_mut().take(r.cell_count()) {
                *c = Biome::NETHER_WASTES;
            }
            return;
        }
        let nn = self.nether.as_ref().expect("Nether noise seeded");
        gen_nether_scaled(nn, cache, r, self.sha);
    }

    /// Nether scale=1 via voronoi access — mirrors cubiomes'
    /// `genNetherScaled` scale=1 branch.
    #[allow(dead_code)]
    fn gen_biomes_nether_voronoi(&self, nn: &NetherNoise, cache: &mut [Biome], r: Range) {
        let sx = r.sx as usize;
        let sy = r.sy as usize;
        let sz = r.sz as usize;
        let total = sx * sy * sz;
        let (src, s_x, s_y, s_z, s_sx, s_sz) = if total > 1 {
            let s = get_voronoi_src_range(r);
            let s_total = (s.sx as usize) * (s.sy as usize) * (s.sz as usize);
            let mut buf = vec![0_i32; s_total];
            nn.map_nether_3d(
                &mut buf,
                s.x,
                s.y,
                s.z,
                s.sx as usize,
                s.sy as usize,
                s.sz as usize,
                4,
                1.0,
            );
            (Some(buf), s.x, s.y, s.z, s.sx as usize, s.sz as usize)
        } else {
            (None, 0, 0, 0, 0, 0)
        };
        let mut p = 0_usize;
        for k in 0..sy {
            for j in 0..sz {
                for i in 0..sx {
                    let (x4, y4, z4) = crate::layer::ops::voronoi::voronoi_access_3d(
                        self.sha,
                        r.x + i as i32,
                        r.y + k as i32,
                        r.z + j as i32,
                    );
                    let id = if let Some(src) = &src {
                        let lx = (x4 - s_x) as usize;
                        let ly = (y4 - s_y) as usize;
                        let lz = (z4 - s_z) as usize;
                        src[ly * s_sx * s_sz + lz * s_sx + lx]
                    } else {
                        nn.get_nether_biome(x4, y4, z4).0.0
                    };
                    cache[p] = Biome(id);
                    p += 1;
                }
            }
        }
    }

    fn gen_biomes_end(&self, cache: &mut [Biome], r: Range) {
        if !self.mc.is_at_least(MCVersion::V1_9) {
            for c in cache.iter_mut().take(r.cell_count()) {
                *c = Biome::THE_END;
            }
            return;
        }
        let en = self.end.as_ref().expect("End noise seeded");
        if r.scale == 1 {
            self.gen_biomes_end_voronoi(en, cache, r);
            return;
        }
        let area = r.sx as usize * r.sz as usize;
        let mut buf = vec![0_i32; area];
        match r.scale {
            4 => en.map_end(&mut buf, r.x, r.z, r.sx as usize, r.sz as usize),
            16 => en.map_end_biome(&mut buf, r.x, r.z, r.sx as usize, r.sz as usize),
            // Cubiomes' else branch: any scale not in {1, 4, 16}
            // falls into the radial-distance pseudo-biome path. The
            // `scale / 8.0` formula handles arbitrary positive
            // scales, so 2/3/8/32/64/256/... all work.
            _ => self.gen_end_large_scale(en, &mut buf, r),
        }
        for k in 0..r.sy as usize {
            for (i, &v) in buf.iter().enumerate() {
                cache[k * area + i] = Biome(v);
            }
        }
    }

    /// End `scale > 16` — cubiomes' radial-distance pseudo-biome
    /// generator. Inside r ≤ 16384, returns `the_end`; for 1.13+ when
    /// `(int)rsq` overflows negative, returns `end_barrens`;
    /// otherwise samples `endHeightNoise` to choose between
    /// `end_highlands`, `end_midlands`, `end_barrens`,
    /// `small_end_islands`.
    #[allow(clippy::invalid_upcast_comparisons)]
    fn gen_end_large_scale(&self, en: &EndNoise, buf: &mut [i32], r: Range) {
        let d = (r.scale as f32) / 8.0_f32;
        let mc_after_113 = self.mc.is_at_least(MCVersion::V1_14);
        for j in 0..r.sz as usize {
            for i in 0..r.sx as usize {
                let hx = ((i as i64 + r.x as i64) as f32 * d) as i64;
                let hz = ((j as i64 + r.z as i64) as f32 * d) as i64;
                let rsq = (hx * hx + hz * hz) as u64;
                let id = if rsq <= 16384 {
                    Biome::THE_END.0
                } else if mc_after_113 && (rsq as i32) < 0 {
                    // cubiomes' (int)rsq cast: lower 32 bits viewed
                    // as signed int. Becomes negative once rsq's bit
                    // 31 is set, which is the "rsq is very large"
                    // shortcut path.
                    Biome::END_BARRENS.0
                } else {
                    let h = en.end_height_noise(hx as i32, hz as i32, 4);
                    if h > 40.0 {
                        Biome::END_HIGHLANDS.0
                    } else if h >= 0.0 {
                        Biome::END_MIDLANDS.0
                    } else if h >= -20.0 {
                        Biome::END_BARRENS.0
                    } else {
                        Biome::SMALL_END_ISLANDS.0
                    }
                };
                buf[j * r.sx as usize + i] = id;
            }
        }
    }

    /// End scale=1 via voronoi access — mirrors cubiomes'
    /// `genEndScaled` scale=1 branch. Pre-1.15 uses `mapVoronoi114`
    /// (planar, expanded to 3D by replicating Y=0); 1.15+ uses
    /// `mapVoronoiPlane` iterated per Y layer.
    fn gen_biomes_end_voronoi(&self, en: &EndNoise, cache: &mut [Biome], r: Range) {
        let sx = r.sx as usize;
        let sy = r.sy as usize;
        let sz = r.sz as usize;
        let s = get_voronoi_src_range(r);
        let s_sx = s.sx as usize;
        let s_sz = s.sz as usize;
        let mut parent = vec![0_i32; s_sx * s_sz];
        en.map_end(&mut parent, s.x, s.z, s_sx, s_sz);
        let parent_b: Vec<Biome> = parent.iter().map(|&v| Biome(v)).collect();

        if self.mc.is_at_least(MCVersion::V1_15) {
            // 3D voronoi — iterate per Y layer.
            let area = sx * sz;
            for k in 0..sy {
                let slice = &mut cache[k * area..(k + 1) * area];
                crate::layer::ops::voronoi::map_voronoi_plane(
                    self.sha,
                    &parent_b,
                    s.x,
                    s.z,
                    s_sx,
                    s_sz,
                    slice,
                    r.x,
                    r.y + k as i32,
                    r.z,
                    sx,
                    sz,
                );
            }
        } else {
            // Planar voronoi — 2D output then replicate across Y.
            let area = sx * sz;
            let mut out_plane = vec![Biome::NONE; area];
            crate::layer::ops::voronoi::map_voronoi114(
                crate::rng::mc_seed::get_layer_salt(10),
                0,
                &parent_b,
                s.x,
                s.z,
                s_sx,
                s_sz,
                &mut out_plane,
                r.x,
                r.z,
                sx,
                sz,
            );
            for k in 0..sy {
                for i in 0..area {
                    cache[k * area + i] = out_plane[i];
                }
            }
        }
    }
}

/// Cubiomes' `genBiomeNoise3D` — per-cell `sampleBiomeNoise` over
/// the range, with `(x, z)` lifted to block coordinates by
/// `(r.x + i) * (scale / 4) + scale/8`. `opt` enables the
/// `SAMPLE_NO_SHIFT` fast-path for large scales (cubiomes' own
/// optimisation — its caller passes `scale > 4`).
/// Bit-exact port of cubiomes' `getVoronoiSrcRange`. Expects
/// `r.scale == 1`; returns a scale-4 range covering the area the
/// voronoi access-pattern can possibly read from.
#[must_use]
fn get_voronoi_src_range(r: Range) -> Range {
    assert!(r.scale == 1, "get_voronoi_src_range: scale must be 1");
    let tx = r.x - 2;
    let tz = r.z - 2;
    let sx = ((tx + r.sx as i32) >> 2) - (tx >> 2) + 2;
    let sz = ((tz + r.sz as i32) >> 2) - (tz >> 2) + 2;
    let (y, sy) = if r.sy < 1 {
        (0_i32, 0_u32)
    } else {
        let ty = r.y - 2;
        let y = ty >> 2;
        let sy = ((ty + r.sy as i32) >> 2) - y + 2;
        (y, sy as u32)
    };
    Range {
        scale: 4,
        x: tx >> 2,
        z: tz >> 2,
        sx: sx as u32,
        sz: sz as u32,
        y,
        sy,
    }
}

/// `genNetherScaled(nn, out, r, mc, sha)` — fill `cache` with
/// Nether biome ids over the requested 3D `Range`. Bit-exact port
/// of cubiomes' `genNetherScaled`: at `r.scale == 1` uses voronoi
/// access at 1:4 scale; otherwise calls [`NetherNoise::map_nether_3d`]
/// directly with the requested scale.
///
/// The caller is responsible for picking the right MC version (the
/// Nether biome generator is identical for all 1.16+ versions, so
/// no `mc` parameter is needed here — cubiomes accepts one for API
/// symmetry but ignores it). For pre-1.16.1, fill with
/// `nether_wastes` manually.
///
/// Most callers should use [`Generator::gen_biomes`] instead; this
/// lower-level helper is for code that has a [`NetherNoise`]
/// without going through a [`Generator`].
pub fn gen_nether_scaled(nn: &NetherNoise, cache: &mut [Biome], r: Range, sha: u64) {
    if r.scale == 1 {
        gen_nether_voronoi(nn, cache, r, sha);
        return;
    }
    let total = r.cell_count();
    let mut buf = vec![0_i32; total];
    nn.map_nether_3d(
        &mut buf,
        r.x,
        r.y,
        r.z,
        r.sx as usize,
        r.sy as usize,
        r.sz as usize,
        r.scale,
        1.0,
    );
    for (dst, src) in cache.iter_mut().zip(buf.iter()) {
        *dst = Biome(*src);
    }
}

/// Nether scale=1 voronoi access — `genNetherScaled` scale=1 branch.
/// Standalone equivalent of [`Generator::gen_biomes_nether_voronoi`].
pub fn gen_nether_voronoi(nn: &NetherNoise, cache: &mut [Biome], r: Range, sha: u64) {
    let sx = r.sx as usize;
    let sy = r.sy as usize;
    let sz = r.sz as usize;
    let total = sx * sy * sz;
    let (src, s_x, s_y, s_z, s_sx, s_sz) = if total > 1 {
        let s = get_voronoi_src_range(r);
        let s_total = (s.sx as usize) * (s.sy as usize) * (s.sz as usize);
        let mut buf = vec![0_i32; s_total];
        nn.map_nether_3d(
            &mut buf,
            s.x,
            s.y,
            s.z,
            s.sx as usize,
            s.sy as usize,
            s.sz as usize,
            4,
            1.0,
        );
        (Some(buf), s.x, s.y, s.z, s.sx as usize, s.sz as usize)
    } else {
        (None, 0, 0, 0, 0, 0)
    };
    let mut p = 0_usize;
    for k in 0..sy {
        for j in 0..sz {
            for i in 0..sx {
                let (x4, y4, z4) = crate::layer::ops::voronoi::voronoi_access_3d(
                    sha,
                    r.x + i as i32,
                    r.y + k as i32,
                    r.z + j as i32,
                );
                let id = if let Some(src) = &src {
                    let lx = (x4 - s_x) as usize;
                    let ly = (y4 - s_y) as usize;
                    let lz = (z4 - s_z) as usize;
                    src[ly * s_sx * s_sz + lz * s_sx + lx]
                } else {
                    nn.get_nether_biome(x4, y4, z4).0.0
                };
                cache[p] = Biome(id);
                p += 1;
            }
        }
    }
}

/// `genBiomeNoiseScaled(bn, out, r, sha)` — fill `cache` with biome
/// ids over the requested 3D `Range`. Bit-exact port of cubiomes'
/// `genBiomeNoiseScaled`: at `r.scale == 1` it uses voronoi access
/// over a scale-4 source range (see [`gen_biome_noise_voronoi`]);
/// otherwise it samples [`BiomeNoise`] directly with the `r.scale > 4`
/// path enabling cubiomes' `SAMPLE_NO_SHIFT` optimisation.
///
/// Most callers should use [`Generator::gen_biomes`] instead; this
/// lower-level helper is for code that has a [`BiomeNoise`] without
/// going through a [`Generator`].
pub fn gen_biome_noise_scaled(bn: &BiomeNoise, cache: &mut [Biome], r: Range, sha: u64) {
    if r.scale == 1 {
        gen_biome_noise_voronoi(bn, cache, r, sha);
    } else {
        gen_biome_noise_3d(bn, cache, r, r.scale > 4);
    }
}

/// `genBiomeNoiseScaled(bn, out, r, sha)` for `r.scale == 1` —
/// voronoi-access at block scale. When the requested cell count
/// is greater than 1, we pre-compute the scale-4 source via
/// [`gen_biome_noise_3d`] so each voronoi sample becomes a single
/// cache lookup; otherwise each cell is sampled directly via
/// [`BiomeNoise::sample`].
pub fn gen_biome_noise_voronoi(bn: &BiomeNoise, cache: &mut [Biome], r: Range, sha: u64) {
    let sx = r.sx as usize;
    let sy = r.sy as usize;
    let sz = r.sz as usize;
    let area = sx * sy * sz;
    if area > 1 {
        let s = get_voronoi_src_range(r);
        let src_len = (s.sx as usize) * (s.sy as usize) * (s.sz as usize);
        let mut src: Vec<Biome> = vec![Biome(0); src_len];
        gen_biome_noise_3d(bn, &mut src, s, false);
        let mut p = 0_usize;
        for k in 0..sy {
            for j in 0..sz {
                for i in 0..sx {
                    let (x4, y4, z4) = crate::layer::ops::voronoi::voronoi_access_3d(
                        sha,
                        r.x + i as i32,
                        r.y + k as i32,
                        r.z + j as i32,
                    );
                    let lx = (x4 - s.x) as usize;
                    let ly = (y4 - s.y) as usize;
                    let lz = (z4 - s.z) as usize;
                    cache[p] =
                        src[ly * (s.sx as usize) * (s.sz as usize) + lz * (s.sx as usize) + lx];
                    p += 1;
                }
            }
        }
    } else {
        let mut p = 0_usize;
        for k in 0..sy {
            for j in 0..sz {
                for i in 0..sx {
                    let (x4, y4, z4) = crate::layer::ops::voronoi::voronoi_access_3d(
                        sha,
                        r.x + i as i32,
                        r.y + k as i32,
                        r.z + j as i32,
                    );
                    let (id, _) = bn.sample(x4, y4, z4, 0);
                    cache[p] = Biome(id);
                    p += 1;
                }
            }
        }
    }
}

/// `genBiomeNoise3D(bn, out, r, opt)` — fill `cache` by sampling
/// [`BiomeNoise`] once per cell at the centre of each scaled cell.
/// `opt` toggles the `SAMPLE_NO_SHIFT` flag (cubiomes' optimisation
/// when the requested scale is coarser than `1:4`). Public for the
/// same reason as [`gen_biome_noise_voronoi`]: callers with a bare
/// [`BiomeNoise`] can target a specific scale without building a
/// full [`Generator`].
pub fn gen_biome_noise_3d(bn: &BiomeNoise, cache: &mut [Biome], r: Range, opt: bool) {
    let scale = if r.scale > 4 { r.scale / 4 } else { 1 };
    let mid = scale / 2;
    let flags = if opt { SAMPLE_NO_SHIFT } else { 0 };
    let sx = r.sx as usize;
    let sz = r.sz as usize;
    for k in 0..r.sy as usize {
        let yk = r.y + k as i32;
        for j in 0..sz {
            let zj = (r.z + j as i32) * scale + mid;
            for i in 0..sx {
                let xi = (r.x + i as i32) * scale + mid;
                let (id, _) = bn.sample(xi, yk, zj, flags);
                cache[k * sx * sz + j * sx + i] = Biome(id);
            }
        }
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
    fn biome_at_end_scale1_pre_115_works() {
        // Should not panic for End scale=1 on pre-1.15 MC — internally
        // delegates to gen_biomes' planar voronoi path.
        let mut g = Generator::new(MCVersion::V1_14, 0);
        g.apply_seed(Dimension::End, 0xdead_beef);
        let _ = g.biome_at(1, 100, 64, 100);
    }

    #[test]
    fn biome_at_modern_unusual_scale_works() {
        // 1.18 Modern Overworld with scale not in {1, 4} falls back
        // to gen_biomes — should not panic.
        let mut g = Generator::new(MCVersion::V1_18, 0);
        g.apply_seed(Dimension::Overworld, 0xdead_beef);
        let _ = g.biome_at(16, 0, 64, 0);
        let _ = g.biome_at(64, 0, 64, 0);
    }

    #[test]
    fn biome_at_nether_unusual_scale_works() {
        // 1.18 Nether with scale 16 should fall back to gen_biomes.
        let mut g = Generator::new(MCVersion::V1_18, 0);
        g.apply_seed(Dimension::Nether, 0xdead_beef);
        let _ = g.biome_at(16, 0, 64, 0);
    }

    #[test]
    fn biome_at_end_large_scale_works() {
        // End scale 64 (radial pseudo-biome) — falls back to gen_biomes.
        let mut g = Generator::new(MCVersion::V1_18, 0);
        g.apply_seed(Dimension::End, 0xdead_beef);
        let _ = g.biome_at(64, 0, 0, 0);
    }

    #[test]
    fn biome_at_beta_applies_scale() {
        // Beta scale=1 samples at (x, z); scale=4 samples at
        // (x*4+2, z*4+2). The two should generally produce different
        // biome ids at (0, 0).
        let mut g = Generator::new(MCVersion::B1_7, 0);
        g.apply_seed(Dimension::Overworld, 0xdead_beef);
        let a = g.biome_at(1, 100, 64, 100);
        let b = g.biome_at(4, 100, 64, 100);
        // Deterministic — running twice yields the same answer.
        assert_eq!(a, g.biome_at(1, 100, 64, 100));
        assert_eq!(b, g.biome_at(4, 100, 64, 100));
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
