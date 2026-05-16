//! Layer DAG storage and per-MC-version construction.
//!
//! Bit-exact port of cubiomes' `LayerStack` + `setupLayer` +
//! `setupScale` + `setupLayerStack`. Each [`LayerId`] is a fixed
//! index into the stack's `layers: [LayerNode; L_NUM]` array — the
//! same convention cubiomes uses with `LayerStack::layers`. Parent
//! edges are `Option<LayerId>`, which sidesteps the borrow-checker
//! pain of intrusive `*Layer` pointers in cubiomes' C code.
//!
//! Only the DAG **construction** lives here. Driving the DAG (the
//! `mapfunc_t` dispatcher cubiomes calls `genArea`) lands in a
//! follow-up commit; this file's parity coverage instead verifies the
//! pre-flight state of every node — `layer_salt`, `start_salt`, and
//! `start_seed` after [`set_layer_seed`].

#![allow(clippy::too_many_lines, clippy::too_many_arguments, unused_assignments)]

use crate::mc_version::MCVersion;
use crate::noise::PerlinNoise;
use crate::rng::{get_layer_salt, get_start_salt, get_start_seed, mc_step_seed};

/// `LAYER_INIT_SHA` from cubiomes — sentinel `layer_salt` selecting
/// the SHA-256-driven Voronoi initialization path inside
/// [`set_layer_seed`].
pub const LAYER_INIT_SHA: u64 = u64::MAX;

/// Layer DAG node id. Indices match cubiomes' `enum LayerId` so a
/// `LayerNode` for `Continent4096` lives at `layers[0]` on both sides.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[allow(missing_docs)]
pub enum LayerId {
    Continent4096 = 0,
    Zoom4096 = 1,
    Land4096 = 2,
    Zoom2048 = 3,
    Land2048 = 4,
    Zoom1024 = 5,
    Land1024A = 6,
    Land1024B = 7,
    Land1024C = 8,
    Island1024 = 9,
    Snow1024 = 10,
    Land1024D = 11,
    Cool1024 = 12,
    Heat1024 = 13,
    Special1024 = 14,
    Zoom512 = 15,
    Land512 = 16,
    Zoom256 = 17,
    Land256 = 18,
    Mushroom256 = 19,
    DeepOcean256 = 20,
    Biome256 = 21,
    Bamboo256 = 22,
    Zoom128 = 23,
    Zoom64 = 24,
    BiomeEdge64 = 25,
    Noise256 = 26,
    Zoom128Hills = 27,
    Zoom64Hills = 28,
    Hills64 = 29,
    Sunflower64 = 30,
    Zoom32 = 31,
    Land32 = 32,
    Zoom16 = 33,
    Shore16 = 34,
    SwampRiver16 = 35,
    Zoom8 = 36,
    Zoom4 = 37,
    Smooth4 = 38,
    Zoom128River = 39,
    Zoom64River = 40,
    Zoom32River = 41,
    Zoom16River = 42,
    Zoom8River = 43,
    Zoom4River = 44,
    River4 = 45,
    Smooth4River = 46,
    RiverMix4 = 47,
    OceanTemp256 = 48,
    Zoom128Ocean = 49,
    Zoom64Ocean = 50,
    Zoom32Ocean = 51,
    Zoom16Ocean = 52,
    Zoom8Ocean = 53,
    Zoom4Ocean = 54,
    OceanMix4 = 55,
    Voronoi1 = 56,
    ZoomLargeA = 57,
    ZoomLargeB = 58,
    ZoomLRiverA = 59,
    ZoomLRiverB = 60,
    /// Extra slot used when `FORCE_OCEAN_VARIANTS` is set: holds an
    /// `OceanMixMod` node that wraps the original `entry_16`.
    /// Mirrors cubiomes' `g->xlayer[2]`. Outside of that flag this
    /// slot stays at [`LayerOp::None`].
    XOceanMix16 = 61,
    /// `OceanMixMod` wrapper around the original `entry_64`. See
    /// [`Self::XOceanMix16`].
    XOceanMix64 = 62,
    /// `OceanMixMod` wrapper around the original `entry_256`. See
    /// [`Self::XOceanMix16`].
    XOceanMix256 = 63,
}

/// Number of layer slots in a [`LayerStack`]. Matches cubiomes'
/// `L_NUM` plus three reserved slots for the `FORCE_OCEAN_VARIANTS`
/// `xlayer` overlay (cubiomes uses `xlayer[2..5]` so we keep the
/// count of "real" cubiomes layers plus those three).
pub const L_NUM: usize = LayerId::XOceanMix256 as usize + 1;

impl LayerId {
    /// Numeric index into [`LayerStack::layers`].
    #[inline]
    #[must_use]
    pub const fn as_index(self) -> usize {
        self as usize
    }
}

/// Identifies which `mapfunc_t` to execute for a [`LayerNode`].
///
/// Cubiomes stores a raw function pointer per node; here we carry an
/// enum and dispatch at run time. That keeps the layer graph
/// `Copy`-able and crate-internal (no borrow-checker fights with
/// trait object lifetimes).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[allow(missing_docs)]
pub enum LayerOp {
    /// Empty slot, never executed.
    None,
    Continent,
    ZoomFuzzy,
    Zoom,
    Land,
    LandB18,
    Land16,
    Snow,
    Snow16,
    Island,
    Cool,
    Heat,
    Special,
    Mushroom,
    DeepOcean,
    Biome,
    Bamboo,
    BiomeEdge,
    Hills,
    Sunflower,
    Shore,
    SwampRiver,
    Smooth,
    Noise,
    River,
    RiverMix,
    OceanTemp,
    OceanMix,
    /// Cubiomes' `mapOceanMixMod` — used only when
    /// `FORCE_OCEAN_VARIANTS` is set on the [`crate::generator::Generator`].
    /// Reads `p` (the original land entry) and `p2` (the
    /// `L_ZOOM_*_OCEAN` ocean layer) and replaces oceanic biomes in
    /// `p` with the temperature-variant from `p2`.
    OceanMixMod,
    Voronoi,
    Voronoi114,
}

/// A single node in the layer DAG. Mirrors the public fields of
/// cubiomes' `struct Layer` — minus the C-style function pointer
/// (replaced with [`LayerOp`]) and the intrusive parent pointers
/// (replaced with [`LayerId`]).
#[derive(Clone, Copy, Debug)]
pub struct LayerNode {
    /// Dispatch tag.
    pub op: LayerOp,
    /// Minecraft version this layer was set up for.
    pub mc: MCVersion,
    /// Zoom factor relative to the parent.
    pub zoom: i8,
    /// Maximum border required from the parent layer.
    pub edge: i8,
    /// Block scale of one output cell. Filled in by `setup_scale`
    /// during [`setup_layer_stack`].
    pub scale: i32,
    /// Pre-init layer salt (raw value assigned by
    /// [`setup_layer_stack`]).
    pub layer_salt: u64,
    /// Per-world layer salt (filled in by [`set_layer_seed`]).
    pub start_salt: u64,
    /// Per-world per-layer chunk-seed base (filled in by
    /// [`set_layer_seed`]).
    pub start_seed: u64,
    /// First parent layer (typically the upstream biome chain).
    pub p: Option<LayerId>,
    /// Optional second parent (e.g. hills, rivers, oceans).
    pub p2: Option<LayerId>,
}

impl Default for LayerNode {
    fn default() -> Self {
        Self {
            op: LayerOp::None,
            mc: MCVersion::Undef,
            zoom: 0,
            edge: 0,
            scale: 0,
            layer_salt: 0,
            start_salt: 0,
            start_seed: 0,
            p: None,
            p2: None,
        }
    }
}

/// A fully assembled layer DAG plus the entry-scale shortcuts cubiomes
/// caches on `LayerStack`. Heap-allocate via `Box::new(LayerStack::new())`
/// — the inline `[LayerNode; L_NUM]` is large enough to want indirection.
#[derive(Clone, Debug)]
pub struct LayerStack {
    /// All layer nodes, indexed by [`LayerId`].
    pub layers: [LayerNode; L_NUM],
    /// Entry node for 1:1 scale (Voronoi).
    pub entry_1: Option<LayerId>,
    /// Entry node for 1:4 scale (`RiverMix4` / `OceanMix4`).
    pub entry_4: Option<LayerId>,
    /// Entry node for 1:16 scale (`SwampRiver16` / `Shore16`).
    pub entry_16: Option<LayerId>,
    /// Entry node for 1:64 scale (Hills64 / Sunflower64).
    pub entry_64: Option<LayerId>,
    /// Entry node for 1:256 scale (Biome256 / Bamboo256).
    pub entry_256: Option<LayerId>,
    /// Ocean-temperature Perlin noise used by `mapOceanTemp` (1.13+).
    pub ocean_rnd: Option<PerlinNoise>,
}

impl LayerStack {
    /// Construct an empty stack with all nodes set to
    /// [`LayerOp::None`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            layers: [LayerNode::default(); L_NUM],
            entry_1: None,
            entry_4: None,
            entry_16: None,
            entry_64: None,
            entry_256: None,
            ocean_rnd: None,
        }
    }

    /// Read-only access to a node by id.
    #[inline]
    #[must_use]
    pub fn node(&self, id: LayerId) -> &LayerNode {
        &self.layers[id.as_index()]
    }
}

impl Default for LayerStack {
    fn default() -> Self {
        Self::new()
    }
}

/// `setupLayer` — populate `layers[id]` and return `id` for chaining.
/// Identical to cubiomes' `setupLayer` modulo Rust ergonomics: layer
/// salts of 0 or `LAYER_INIT_SHA` are kept raw, everything else is
/// pre-processed by [`get_layer_salt`].
fn setup_layer(
    layers: &mut [LayerNode; L_NUM],
    id: LayerId,
    op: LayerOp,
    mc: MCVersion,
    zoom: i8,
    edge: i8,
    salt_base: u64,
    p: Option<LayerId>,
    p2: Option<LayerId>,
) -> LayerId {
    let layer_salt = if salt_base == 0 || salt_base == LAYER_INIT_SHA {
        salt_base
    } else {
        get_layer_salt(salt_base)
    };
    layers[id.as_index()] = LayerNode {
        op,
        mc,
        zoom,
        edge,
        scale: 0,
        layer_salt,
        start_salt: 0,
        start_seed: 0,
        p,
        p2,
    };
    id
}

/// Recursive scale propagation, mirroring cubiomes' static
/// `setupScale`. Starts at the 1:1 entry layer and walks parents,
/// multiplying by `zoom` each hop.
fn setup_scale(layers: &mut [LayerNode; L_NUM], id: LayerId, scale: i32) {
    layers[id.as_index()].scale = scale;
    let zoom = layers[id.as_index()].zoom;
    let p = layers[id.as_index()].p;
    let p2 = layers[id.as_index()].p2;
    let child_scale = scale * i32::from(zoom);
    if let Some(parent) = p {
        setup_scale(layers, parent, child_scale);
    }
    if let Some(parent) = p2 {
        setup_scale(layers, parent, child_scale);
    }
}

/// `setLayerSeed` — populate `layer_salt`, `start_salt`, and
/// `start_seed` for every node reachable from `entry`. Walks the DAG
/// post-order so that parents are seeded before children. When an
/// `OceanTemp` node is encountered, re-initialise the stack's
/// `ocean_rnd` Perlin noise from the world seed (cubiomes does the
/// same via the layer's `noise` pointer).
pub fn set_layer_seed(stack: &mut LayerStack, entry: LayerId, world_seed: u64) {
    set_layer_seed_recursive(stack, entry, world_seed);
}

fn set_layer_seed_recursive(stack: &mut LayerStack, id: LayerId, world_seed: u64) {
    let p = stack.layers[id.as_index()].p;
    let p2 = stack.layers[id.as_index()].p2;
    if let Some(parent) = p2 {
        set_layer_seed_recursive(stack, parent, world_seed);
    }
    if let Some(parent) = p {
        set_layer_seed_recursive(stack, parent, world_seed);
    }

    if matches!(stack.layers[id.as_index()].op, LayerOp::OceanTemp) {
        // Cubiomes' setLayerSeed initialises the layer's `noise`
        // PerlinNoise from the world seed. Mirror that here against
        // the stack-level `ocean_rnd` field.
        let mut rng = crate::rng::JavaRng::new(world_seed);
        stack.ocean_rnd = Some(crate::noise::PerlinNoise::from_java(&mut rng));
    }

    let node = &mut stack.layers[id.as_index()];
    let ls = node.layer_salt;
    if ls == 0 {
        node.start_salt = 0;
        node.start_seed = 0;
    } else if ls == LAYER_INIT_SHA {
        node.start_salt = crate::sha::voronoi_sha(world_seed);
        node.start_seed = 0;
    } else {
        let st = get_start_salt(world_seed, ls);
        node.start_salt = st;
        node.start_seed = mc_step_seed(st, 0);
        debug_assert_eq!(node.start_seed, get_start_seed(world_seed, ls));
    }
}

/// Apply the `FORCE_OCEAN_VARIANTS` flag to a layered stack:
/// inject `OceanMixMod` wrappers at the 1:16, 1:64, 1:256 entries
/// so that biome generation always produces the temperature
/// variants on oceanic cells. Mirrors cubiomes' `setupGenerator`
/// branch when `flags & FORCE_OCEAN_VARIANTS && mc >= MC_1_13`.
///
/// Safe to call at any point after [`setup_layer_stack`] for an
/// `mc >= V1_13` stack; no-op otherwise (entry layers are assumed
/// non-None and the ocean-temperature chain to exist).
pub fn apply_force_ocean_variants(stack: &mut LayerStack, mc: MCVersion) {
    if !mc.is_at_least(MCVersion::V1_13) {
        return;
    }
    let (Some(orig_16), Some(orig_64), Some(orig_256)) =
        (stack.entry_16, stack.entry_64, stack.entry_256)
    else {
        return;
    };
    // Cubiomes: setupLayer(xlayer+2, &mapOceanMixMod, mc, 1, 0, 0,
    //                     entry_16, &g->ls.layers[L_ZOOM_16_OCEAN]);
    let layers = &mut stack.layers;
    setup_layer(
        layers,
        LayerId::XOceanMix16,
        LayerOp::OceanMixMod,
        mc,
        1,
        0,
        0,
        Some(orig_16),
        Some(LayerId::Zoom16Ocean),
    );
    setup_layer(
        layers,
        LayerId::XOceanMix64,
        LayerOp::OceanMixMod,
        mc,
        1,
        0,
        0,
        Some(orig_64),
        Some(LayerId::Zoom64Ocean),
    );
    setup_layer(
        layers,
        LayerId::XOceanMix256,
        LayerOp::OceanMixMod,
        mc,
        1,
        0,
        0,
        Some(orig_256),
        Some(LayerId::OceanTemp256),
    );
    // Propagate scales from the wrapped entries.
    setup_scale(layers, LayerId::XOceanMix16, 16);
    setup_scale(layers, LayerId::XOceanMix64, 64);
    setup_scale(layers, LayerId::XOceanMix256, 256);
    stack.entry_16 = Some(LayerId::XOceanMix16);
    stack.entry_64 = Some(LayerId::XOceanMix64);
    stack.entry_256 = Some(LayerId::XOceanMix256);
}

/// `setupLayerStack` — build the layer DAG for `mc` (and the
/// `LARGE_BIOMES` toggle). After this call every populated [`LayerId`]
/// is a valid node and the entry-scale shortcuts are filled in.
///
/// This is a near-mechanical translation of cubiomes' `setupLayerStack`
/// in `generator.c`; the comments cross-reference the upstream
/// branches.
pub fn setup_layer_stack(stack: &mut LayerStack, mc: MCVersion, large_biomes: bool) {
    // Pre-1.3 has no large-biome variant.
    let large_biomes = large_biomes && mc.is_at_least(MCVersion::V1_3);

    *stack = LayerStack::new();
    let layers = &mut stack.layers;

    let mut p: LayerId;
    let map_land: LayerOp;

    // ----- continent → biome chain (varies by version) -----
    if mc == MCVersion::B1_8 {
        // L_CONTINENT_4096 here actually represents the 1:8192 scale,
        // but cubiomes reuses the slot.
        map_land = LayerOp::LandB18;
        p = setup_layer(
            layers,
            LayerId::Continent4096,
            LayerOp::Continent,
            mc,
            1,
            0,
            1,
            None,
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom4096,
            LayerOp::ZoomFuzzy,
            mc,
            2,
            3,
            2000,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Land4096,
            map_land,
            mc,
            1,
            2,
            1,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom2048,
            LayerOp::Zoom,
            mc,
            2,
            3,
            2001,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Land2048,
            map_land,
            mc,
            1,
            2,
            2,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom1024,
            LayerOp::Zoom,
            mc,
            2,
            3,
            2002,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Land1024A,
            map_land,
            mc,
            1,
            2,
            3,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom512,
            LayerOp::Zoom,
            mc,
            2,
            3,
            2003,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Land512,
            map_land,
            mc,
            1,
            2,
            3,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom256,
            LayerOp::Zoom,
            mc,
            2,
            3,
            2004,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Land256,
            map_land,
            mc,
            1,
            2,
            3,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Biome256,
            LayerOp::Biome,
            mc,
            1,
            0,
            200,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom128,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1000,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom64,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1001,
            Some(p),
            None,
        );
        // River noise layer chain branches off Land256 in B1.8.
        p = setup_layer(
            layers,
            LayerId::Noise256,
            LayerOp::Noise,
            mc,
            1,
            0,
            100,
            Some(LayerId::Land256),
            None,
        );
    } else if mc.ord() <= MCVersion::V1_6.ord() {
        map_land = LayerOp::Land16;
        p = setup_layer(
            layers,
            LayerId::Continent4096,
            LayerOp::Continent,
            mc,
            1,
            0,
            1,
            None,
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom2048,
            LayerOp::ZoomFuzzy,
            mc,
            2,
            3,
            2000,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Land2048,
            map_land,
            mc,
            1,
            2,
            1,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom1024,
            LayerOp::Zoom,
            mc,
            2,
            3,
            2001,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Land1024A,
            map_land,
            mc,
            1,
            2,
            2,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Snow1024,
            LayerOp::Snow16,
            mc,
            1,
            2,
            2,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom512,
            LayerOp::Zoom,
            mc,
            2,
            3,
            2002,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Land512,
            map_land,
            mc,
            1,
            2,
            3,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom256,
            LayerOp::Zoom,
            mc,
            2,
            3,
            2003,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Land256,
            map_land,
            mc,
            1,
            2,
            4,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Mushroom256,
            LayerOp::Mushroom,
            mc,
            1,
            2,
            5,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Biome256,
            LayerOp::Biome,
            mc,
            1,
            0,
            200,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom128,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1000,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom64,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1001,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Noise256,
            LayerOp::Noise,
            mc,
            1,
            0,
            100,
            Some(LayerId::Mushroom256),
            None,
        );
    } else {
        map_land = LayerOp::Land;
        p = setup_layer(
            layers,
            LayerId::Continent4096,
            LayerOp::Continent,
            mc,
            1,
            0,
            1,
            None,
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom2048,
            LayerOp::ZoomFuzzy,
            mc,
            2,
            3,
            2000,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Land2048,
            map_land,
            mc,
            1,
            2,
            1,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom1024,
            LayerOp::Zoom,
            mc,
            2,
            3,
            2001,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Land1024A,
            map_land,
            mc,
            1,
            2,
            2,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Land1024B,
            map_land,
            mc,
            1,
            2,
            50,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Land1024C,
            map_land,
            mc,
            1,
            2,
            70,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Island1024,
            LayerOp::Island,
            mc,
            1,
            2,
            2,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Snow1024,
            LayerOp::Snow,
            mc,
            1,
            2,
            2,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Land1024D,
            map_land,
            mc,
            1,
            2,
            3,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Cool1024,
            LayerOp::Cool,
            mc,
            1,
            2,
            2,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Heat1024,
            LayerOp::Heat,
            mc,
            1,
            2,
            2,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Special1024,
            LayerOp::Special,
            mc,
            1,
            2,
            3,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom512,
            LayerOp::Zoom,
            mc,
            2,
            3,
            2002,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom256,
            LayerOp::Zoom,
            mc,
            2,
            3,
            2003,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Land256,
            map_land,
            mc,
            1,
            2,
            4,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Mushroom256,
            LayerOp::Mushroom,
            mc,
            1,
            2,
            5,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::DeepOcean256,
            LayerOp::DeepOcean,
            mc,
            1,
            2,
            4,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Biome256,
            LayerOp::Biome,
            mc,
            1,
            0,
            200,
            Some(p),
            None,
        );
        if mc.is_at_least(MCVersion::V1_14) {
            p = setup_layer(
                layers,
                LayerId::Bamboo256,
                LayerOp::Bamboo,
                mc,
                1,
                0,
                1001,
                Some(p),
                None,
            );
        }
        p = setup_layer(
            layers,
            LayerId::Zoom128,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1000,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom64,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1001,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::BiomeEdge64,
            LayerOp::BiomeEdge,
            mc,
            1,
            2,
            1000,
            Some(p),
            None,
        );
        // River noise hangs off DeepOcean256 for 1.7+.
        p = setup_layer(
            layers,
            LayerId::Noise256,
            LayerOp::Noise,
            mc,
            1,
            0,
            100,
            Some(LayerId::DeepOcean256),
            None,
        );
    }

    // ----- hills/river zoom chain -----
    if mc.ord() <= MCVersion::V1_0.ord() {
        // No hills chain at all.
    } else if mc.ord() <= MCVersion::V1_12.ord() {
        p = setup_layer(
            layers,
            LayerId::Zoom128Hills,
            LayerOp::Zoom,
            mc,
            2,
            3,
            0,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom64Hills,
            LayerOp::Zoom,
            mc,
            2,
            3,
            0,
            Some(p),
            None,
        );
    } else {
        p = setup_layer(
            layers,
            LayerId::Zoom128Hills,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1000,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom64Hills,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1001,
            Some(p),
            None,
        );
    }

    // ----- biome → shore → smooth / river chain (varies by version) -----
    if mc.ord() <= MCVersion::V1_0.ord() {
        p = setup_layer(
            layers,
            LayerId::Zoom32,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1000,
            Some(LayerId::Zoom64),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Land32,
            map_land,
            mc,
            1,
            2,
            3,
            Some(p),
            None,
        );
        // 1.0 reuses the Shore16 slot for what is actually scale 1:32.
        p = setup_layer(
            layers,
            LayerId::Shore16,
            LayerOp::Shore,
            mc,
            1,
            2,
            1000,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom16,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1001,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom8,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1002,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom4,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1003,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Smooth4,
            LayerOp::Smooth,
            mc,
            1,
            2,
            1000,
            Some(p),
            None,
        );

        p = setup_layer(
            layers,
            LayerId::Zoom128River,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1000,
            Some(LayerId::Noise256),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom64River,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1001,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom32River,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1002,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom16River,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1003,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom8River,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1004,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom4River,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1005,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::River4,
            LayerOp::River,
            mc,
            1,
            2,
            1,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Smooth4River,
            LayerOp::Smooth,
            mc,
            1,
            2,
            1000,
            Some(p),
            None,
        );
    } else if mc.ord() <= MCVersion::V1_6.ord() {
        p = setup_layer(
            layers,
            LayerId::Hills64,
            LayerOp::Hills,
            mc,
            1,
            2,
            1000,
            Some(LayerId::Zoom64),
            Some(LayerId::Zoom64Hills),
        );
        p = setup_layer(
            layers,
            LayerId::Zoom32,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1000,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Land32,
            map_land,
            mc,
            1,
            2,
            3,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom16,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1001,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Shore16,
            LayerOp::Shore,
            mc,
            1,
            2,
            1000,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::SwampRiver16,
            LayerOp::SwampRiver,
            mc,
            1,
            0,
            1000,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom8,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1002,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom4,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1003,
            Some(p),
            None,
        );

        if large_biomes {
            p = setup_layer(
                layers,
                LayerId::ZoomLargeA,
                LayerOp::Zoom,
                mc,
                2,
                3,
                1004,
                Some(p),
                None,
            );
            p = setup_layer(
                layers,
                LayerId::ZoomLargeB,
                LayerOp::Zoom,
                mc,
                2,
                3,
                1005,
                Some(p),
                None,
            );
        }

        p = setup_layer(
            layers,
            LayerId::Smooth4,
            LayerOp::Smooth,
            mc,
            1,
            2,
            1000,
            Some(p),
            None,
        );

        // River chain.
        p = setup_layer(
            layers,
            LayerId::Zoom128River,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1000,
            Some(LayerId::Noise256),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom64River,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1001,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom32River,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1002,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom16River,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1003,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom8River,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1004,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom4River,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1005,
            Some(p),
            None,
        );
        if large_biomes {
            p = setup_layer(
                layers,
                LayerId::ZoomLRiverA,
                LayerOp::Zoom,
                mc,
                2,
                3,
                1006,
                Some(p),
                None,
            );
            p = setup_layer(
                layers,
                LayerId::ZoomLRiverB,
                LayerOp::Zoom,
                mc,
                2,
                3,
                1007,
                Some(p),
                None,
            );
        }
        p = setup_layer(
            layers,
            LayerId::River4,
            LayerOp::River,
            mc,
            1,
            2,
            1,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Smooth4River,
            LayerOp::Smooth,
            mc,
            1,
            2,
            1000,
            Some(p),
            None,
        );
    } else {
        // 1.7+ — hills branches off the biome-edge chain.
        p = setup_layer(
            layers,
            LayerId::Hills64,
            LayerOp::Hills,
            mc,
            1,
            2,
            1000,
            Some(LayerId::BiomeEdge64),
            Some(LayerId::Zoom64Hills),
        );
        p = setup_layer(
            layers,
            LayerId::Sunflower64,
            LayerOp::Sunflower,
            mc,
            1,
            0,
            1001,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom32,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1000,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Land32,
            map_land,
            mc,
            1,
            2,
            3,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom16,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1001,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Shore16,
            LayerOp::Shore,
            mc,
            1,
            2,
            1000,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom8,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1002,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom4,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1003,
            Some(p),
            None,
        );

        if large_biomes {
            p = setup_layer(
                layers,
                LayerId::ZoomLargeA,
                LayerOp::Zoom,
                mc,
                2,
                3,
                1004,
                Some(p),
                None,
            );
            p = setup_layer(
                layers,
                LayerId::ZoomLargeB,
                LayerOp::Zoom,
                mc,
                2,
                3,
                1005,
                Some(p),
                None,
            );
        }

        p = setup_layer(
            layers,
            LayerId::Smooth4,
            LayerOp::Smooth,
            mc,
            1,
            2,
            1000,
            Some(p),
            None,
        );

        // River chain — 1.7+ hangs the river off Noise256, with
        // slightly different salts than 1.6 (1000/1001/1000/1001/...).
        p = setup_layer(
            layers,
            LayerId::Zoom128River,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1000,
            Some(LayerId::Noise256),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom64River,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1001,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom32River,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1000,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom16River,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1001,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom8River,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1002,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom4River,
            LayerOp::Zoom,
            mc,
            2,
            3,
            1003,
            Some(p),
            None,
        );

        if large_biomes && mc == MCVersion::V1_7 {
            p = setup_layer(
                layers,
                LayerId::ZoomLRiverA,
                LayerOp::Zoom,
                mc,
                2,
                3,
                1004,
                Some(p),
                None,
            );
            p = setup_layer(
                layers,
                LayerId::ZoomLRiverB,
                LayerOp::Zoom,
                mc,
                2,
                3,
                1005,
                Some(p),
                None,
            );
        }

        p = setup_layer(
            layers,
            LayerId::River4,
            LayerOp::River,
            mc,
            1,
            2,
            1,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Smooth4River,
            LayerOp::Smooth,
            mc,
            1,
            2,
            1000,
            Some(p),
            None,
        );
    }

    p = setup_layer(
        layers,
        LayerId::RiverMix4,
        LayerOp::RiverMix,
        mc,
        1,
        0,
        100,
        Some(LayerId::Smooth4),
        Some(LayerId::Smooth4River),
    );

    // ----- Voronoi / ocean-mix tail -----
    if mc.ord() <= MCVersion::V1_12.ord() {
        p = setup_layer(
            layers,
            LayerId::Voronoi1,
            LayerOp::Voronoi114,
            mc,
            4,
            3,
            10,
            Some(p),
            None,
        );
    } else {
        // Ocean variants + final Voronoi.
        p = setup_layer(
            layers,
            LayerId::OceanTemp256,
            LayerOp::OceanTemp,
            mc,
            1,
            0,
            2,
            None,
            None,
        );
        // `ocean_rnd` lives on the stack itself; cubiomes wires
        // `p->noise = &g->oceanRnd` here, but our [`set_layer_seed`]
        // initializes the PerlinNoise from the world seed on every
        // call so we just remember to populate it.
        p = setup_layer(
            layers,
            LayerId::Zoom128Ocean,
            LayerOp::Zoom,
            mc,
            2,
            3,
            2001,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom64Ocean,
            LayerOp::Zoom,
            mc,
            2,
            3,
            2002,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom32Ocean,
            LayerOp::Zoom,
            mc,
            2,
            3,
            2003,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom16Ocean,
            LayerOp::Zoom,
            mc,
            2,
            3,
            2004,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom8Ocean,
            LayerOp::Zoom,
            mc,
            2,
            3,
            2005,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::Zoom4Ocean,
            LayerOp::Zoom,
            mc,
            2,
            3,
            2006,
            Some(p),
            None,
        );
        p = setup_layer(
            layers,
            LayerId::OceanMix4,
            LayerOp::OceanMix,
            mc,
            1,
            17,
            100,
            Some(LayerId::RiverMix4),
            Some(LayerId::Zoom4Ocean),
        );

        if mc.ord() <= MCVersion::V1_14.ord() {
            p = setup_layer(
                layers,
                LayerId::Voronoi1,
                LayerOp::Voronoi114,
                mc,
                4,
                3,
                10,
                Some(p),
                None,
            );
        } else {
            p = setup_layer(
                layers,
                LayerId::Voronoi1,
                LayerOp::Voronoi,
                mc,
                4,
                3,
                LAYER_INIT_SHA,
                Some(p),
                None,
            );
        }
    }

    stack.entry_1 = Some(p);
    stack.entry_4 = Some(if mc.ord() <= MCVersion::V1_12.ord() {
        LayerId::RiverMix4
    } else {
        LayerId::OceanMix4
    });
    if large_biomes {
        stack.entry_16 = Some(LayerId::Zoom4);
        stack.entry_64 = Some(if mc.ord() <= MCVersion::V1_6.ord() {
            LayerId::SwampRiver16
        } else {
            LayerId::Shore16
        });
        stack.entry_256 = Some(if mc.ord() <= MCVersion::V1_6.ord() {
            LayerId::Hills64
        } else {
            LayerId::Sunflower64
        });
    } else if mc.is_at_least(MCVersion::V1_1) {
        stack.entry_16 = Some(if mc.ord() <= MCVersion::V1_6.ord() {
            LayerId::SwampRiver16
        } else {
            LayerId::Shore16
        });
        stack.entry_64 = Some(if mc.ord() <= MCVersion::V1_6.ord() {
            LayerId::Hills64
        } else {
            LayerId::Sunflower64
        });
        stack.entry_256 = Some(if mc.ord() <= MCVersion::V1_14.ord() {
            LayerId::Biome256
        } else {
            LayerId::Bamboo256
        });
    } else {
        stack.entry_16 = Some(LayerId::Zoom16);
        stack.entry_64 = Some(LayerId::Zoom64);
        stack.entry_256 = Some(LayerId::Biome256);
    }

    setup_scale(layers, p, 1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_stack_is_empty() {
        let s = LayerStack::new();
        assert!(s.entry_1.is_none());
        assert_eq!(s.node(LayerId::Continent4096).op, LayerOp::None);
    }

    #[test]
    fn setup_layer_stack_1_18_populates_voronoi_entry() {
        let mut s = LayerStack::new();
        setup_layer_stack(&mut s, MCVersion::V1_18, false);
        assert_eq!(s.entry_1, Some(LayerId::Voronoi1));
        assert_eq!(s.entry_4, Some(LayerId::OceanMix4));
        assert_eq!(s.entry_256, Some(LayerId::Bamboo256));
        assert_eq!(s.node(LayerId::Voronoi1).layer_salt, LAYER_INIT_SHA);
        // Scale propagation: Voronoi is 1:1, its parent OceanMix4 is 1:4.
        assert_eq!(s.node(LayerId::Voronoi1).scale, 1);
        assert_eq!(s.node(LayerId::OceanMix4).scale, 4);
    }

    #[test]
    fn set_layer_seed_propagates_to_parents() {
        let mut s = LayerStack::new();
        setup_layer_stack(&mut s, MCVersion::V1_18, false);
        let entry = s.entry_1.unwrap();
        set_layer_seed(&mut s, entry, 0xdead_beef_1234);
        // Voronoi1 takes the SHA path; start_seed stays 0.
        let v = s.node(LayerId::Voronoi1);
        assert_eq!(v.start_seed, 0);
        assert_eq!(v.start_salt, crate::sha::voronoi_sha(0xdead_beef_1234));
        // Continent4096 takes the standard path; start_seed != 0.
        let c = s.node(LayerId::Continent4096);
        assert_ne!(c.start_salt, 0);
    }

    #[test]
    fn apply_force_ocean_variants_redirects_entries() {
        let mut s = LayerStack::new();
        setup_layer_stack(&mut s, MCVersion::V1_16_1, false);
        let orig_16 = s.entry_16.unwrap();
        let orig_64 = s.entry_64.unwrap();
        let orig_256 = s.entry_256.unwrap();
        apply_force_ocean_variants(&mut s, MCVersion::V1_16_1);
        // entry_1 / entry_4 are unchanged.
        assert_eq!(s.entry_1, Some(LayerId::Voronoi1));
        assert_eq!(s.entry_4, Some(LayerId::OceanMix4));
        // entry_16 / 64 / 256 now point to xlayer wrappers.
        assert_eq!(s.entry_16, Some(LayerId::XOceanMix16));
        assert_eq!(s.entry_64, Some(LayerId::XOceanMix64));
        assert_eq!(s.entry_256, Some(LayerId::XOceanMix256));
        // The wrappers carry OceanMixMod and the right parents.
        let x16 = s.node(LayerId::XOceanMix16);
        assert_eq!(x16.op, LayerOp::OceanMixMod);
        assert_eq!(x16.p, Some(orig_16));
        assert_eq!(x16.p2, Some(LayerId::Zoom16Ocean));
        assert_eq!(x16.scale, 16);
        let x64 = s.node(LayerId::XOceanMix64);
        assert_eq!(x64.p, Some(orig_64));
        assert_eq!(x64.p2, Some(LayerId::Zoom64Ocean));
        let x256 = s.node(LayerId::XOceanMix256);
        assert_eq!(x256.p, Some(orig_256));
        assert_eq!(x256.p2, Some(LayerId::OceanTemp256));
    }

    #[test]
    fn apply_force_ocean_variants_noop_pre_113() {
        let mut s = LayerStack::new();
        setup_layer_stack(&mut s, MCVersion::V1_12, false);
        let pre_16 = s.entry_16;
        apply_force_ocean_variants(&mut s, MCVersion::V1_12);
        assert_eq!(s.entry_16, pre_16);
        assert_eq!(s.node(LayerId::XOceanMix16).op, LayerOp::None);
    }
}
