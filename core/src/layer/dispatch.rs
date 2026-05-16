//! Layer DAG dispatcher — the Rust counterpart of cubiomes' `genArea`.
//!
//! Given a [`LayerStack`] previously initialised with
//! [`super::stack::setup_layer_stack`] and seeded with
//! [`super::stack::set_layer_seed`], [`gen_area`] walks the DAG from
//! the requested entry node down to the leaves, computes each
//! parent's input rectangle, allocates a scratch buffer, recurses,
//! and finally calls the per-op `map_*` function from
//! [`super::ops`].
//!
//! The dispatcher is intentionally allocation-heavy: each layer hop
//! gets a fresh `Vec<Biome>` for its parent's output. Cubiomes
//! amortises these via a single large `out` buffer pre-sized by
//! `getMinLayerCacheSize`, but a naive allocator is enough to
//! validate bit-exact compatibility; cache-pooling can land later
//! once the layer pipeline is established.

#![allow(clippy::many_single_char_names, clippy::too_many_lines)]

use crate::biome::Biome;

use super::ops::{
    bamboo::map_bamboo,
    biome::map_biome,
    biome_edge::map_biome_edge,
    continent::map_continent,
    deep_ocean::map_deep_ocean,
    hills::map_hills,
    island::map_island,
    land::{map_land, map_land_b18, map_land16},
    mushroom::map_mushroom,
    noise::map_noise,
    ocean_mix::{map_ocean_mix, map_ocean_mix_mod, ocean_land_bbox},
    ocean_temp::map_ocean_temp,
    river::map_river,
    river_mix::map_river_mix,
    shore::map_shore,
    smooth::map_smooth,
    snow::{map_snow, map_snow16},
    special::map_special,
    sunflower::map_sunflower,
    swamp_river::map_swamp_river,
    temperature::{map_cool, map_heat},
    voronoi::{map_voronoi, map_voronoi114},
    zoom::{map_zoom, map_zoom_fuzzy},
};
use super::stack::{LayerId, LayerOp, LayerStack};

/// Run the layer at `id` (and all its parents) and write the
/// `(w, h)`-sized output into `out` starting at `(x, z)`. Mirrors
/// cubiomes' `genArea(layer, out, x, z, w, h)`.
///
/// `stack` must already have been initialised with
/// `setup_layer_stack` and (if the layer reads `start_salt` /
/// `start_seed`) seeded with `set_layer_seed`.
pub fn gen_area(
    stack: &LayerStack,
    id: LayerId,
    out: &mut [Biome],
    x: i32,
    z: i32,
    w: usize,
    h: usize,
) {
    assert!(out.len() >= w * h, "gen_area: out slice too small");
    let node = &stack.layers[id.as_index()];

    match node.op {
        LayerOp::None => panic!("gen_area: unset layer at id {id:?}"),

        LayerOp::Continent => {
            map_continent(node.start_seed, out, x, z, w, h);
        }

        LayerOp::Zoom | LayerOp::ZoomFuzzy => {
            let p = node.p.expect("Zoom needs a parent");
            let parent_x = x >> 1;
            let parent_z = z >> 1;
            let parent_w = (((x + w as i32) >> 1) - parent_x + 1) as usize;
            let parent_h = (((z + h as i32) >> 1) - parent_z + 1) as usize;
            let mut parent = vec![Biome::NONE; parent_w * parent_h];
            gen_area(
                stack,
                p,
                &mut parent,
                parent_x,
                parent_z,
                parent_w,
                parent_h,
            );
            if matches!(node.op, LayerOp::ZoomFuzzy) {
                map_zoom_fuzzy(
                    node.start_salt,
                    node.start_seed,
                    &parent,
                    parent_x,
                    parent_z,
                    parent_w,
                    parent_h,
                    out,
                    x,
                    z,
                    w,
                    h,
                );
            } else {
                map_zoom(
                    node.start_salt,
                    node.start_seed,
                    &parent,
                    parent_x,
                    parent_z,
                    parent_w,
                    parent_h,
                    out,
                    x,
                    z,
                    w,
                    h,
                );
            }
        }

        LayerOp::Land
        | LayerOp::LandB18
        | LayerOp::Land16
        | LayerOp::Snow
        | LayerOp::Snow16
        | LayerOp::Island
        | LayerOp::Cool
        | LayerOp::Heat
        | LayerOp::Mushroom
        | LayerOp::DeepOcean
        | LayerOp::BiomeEdge
        | LayerOp::Shore
        | LayerOp::River
        | LayerOp::Smooth => {
            let p = node.p.expect("edge=2 layer needs a parent");
            let parent_w = w + 2;
            let parent_h = h + 2;
            let mut parent = vec![Biome::NONE; parent_w * parent_h];
            gen_area(stack, p, &mut parent, x - 1, z - 1, parent_w, parent_h);
            match node.op {
                LayerOp::Land => {
                    map_land(node.start_salt, node.start_seed, &parent, out, x, z, w, h);
                }
                LayerOp::LandB18 => {
                    map_land_b18(node.start_seed, &parent, out, x, z, w, h);
                }
                LayerOp::Land16 => {
                    map_land16(node.start_salt, node.start_seed, &parent, out, x, z, w, h);
                }
                LayerOp::Snow => map_snow(node.start_seed, &parent, out, x, z, w, h),
                LayerOp::Snow16 => map_snow16(node.start_seed, &parent, out, x, z, w, h),
                LayerOp::Island => map_island(node.start_seed, &parent, out, x, z, w, h),
                LayerOp::Cool => map_cool(&parent, out, w, h),
                LayerOp::Heat => map_heat(&parent, out, w, h),
                LayerOp::Mushroom => map_mushroom(node.start_seed, &parent, out, x, z, w, h),
                LayerOp::DeepOcean => map_deep_ocean(&parent, out, w, h),
                LayerOp::BiomeEdge => map_biome_edge(node.mc, &parent, out, w, h),
                LayerOp::Shore => map_shore(node.mc, &parent, out, w, h),
                LayerOp::River => map_river(node.mc, &parent, out, w, h),
                LayerOp::Smooth => map_smooth(node.start_seed, &parent, out, x, z, w, h),
                _ => unreachable!(),
            }
        }

        LayerOp::Special => {
            let p = node.p.expect("Special needs a parent");
            let mut parent = vec![Biome::NONE; w * h];
            gen_area(stack, p, &mut parent, x, z, w, h);
            map_special(node.start_salt, node.start_seed, &parent, out, x, z, w, h);
        }
        LayerOp::Biome => {
            let p = node.p.expect("Biome needs a parent");
            let mut parent = vec![Biome::NONE; w * h];
            gen_area(stack, p, &mut parent, x, z, w, h);
            map_biome(node.mc, node.start_seed, &parent, out, x, z, w, h);
        }
        LayerOp::Bamboo => {
            let p = node.p.expect("Bamboo needs a parent");
            let mut parent = vec![Biome::NONE; w * h];
            gen_area(stack, p, &mut parent, x, z, w, h);
            map_bamboo(node.start_seed, &parent, out, x, z, w, h);
        }
        LayerOp::Noise => {
            let p = node.p.expect("Noise needs a parent");
            let mut parent = vec![Biome::NONE; w * h];
            gen_area(stack, p, &mut parent, x, z, w, h);
            map_noise(node.mc, node.start_seed, &parent, out, x, z, w, h);
        }
        LayerOp::SwampRiver => {
            let p = node.p.expect("SwampRiver needs a parent");
            let mut parent = vec![Biome::NONE; w * h];
            gen_area(stack, p, &mut parent, x, z, w, h);
            map_swamp_river(node.start_seed, &parent, out, x, z, w, h);
        }
        LayerOp::Sunflower => {
            let p = node.p.expect("Sunflower needs a parent");
            let mut parent = vec![Biome::NONE; w * h];
            gen_area(stack, p, &mut parent, x, z, w, h);
            map_sunflower(node.start_seed, &parent, out, x, z, w, h);
        }

        LayerOp::Hills => {
            let p = node.p.expect("Hills needs a parent");
            let p2 = node.p2.expect("Hills needs a second parent");
            let parent_w = w + 2;
            let parent_h = h + 2;
            let mut biome_parent = vec![Biome::NONE; parent_w * parent_h];
            let mut river_parent = vec![Biome::NONE; parent_w * parent_h];
            gen_area(
                stack,
                p,
                &mut biome_parent,
                x - 1,
                z - 1,
                parent_w,
                parent_h,
            );
            gen_area(
                stack,
                p2,
                &mut river_parent,
                x - 1,
                z - 1,
                parent_w,
                parent_h,
            );
            map_hills(
                node.mc,
                node.start_salt,
                node.start_seed,
                &biome_parent,
                &river_parent,
                out,
                x,
                z,
                w,
                h,
            );
        }

        LayerOp::RiverMix => {
            let p = node.p.expect("RiverMix needs a parent");
            let p2 = node.p2.expect("RiverMix needs a second parent");
            let mut biome_parent = vec![Biome::NONE; w * h];
            let mut river_parent = vec![Biome::NONE; w * h];
            gen_area(stack, p, &mut biome_parent, x, z, w, h);
            gen_area(stack, p2, &mut river_parent, x, z, w, h);
            map_river_mix(node.mc, &biome_parent, &river_parent, out, w, h);
        }

        LayerOp::OceanTemp => {
            let noise = stack
                .ocean_rnd
                .as_ref()
                .expect("OceanTemp requires stack.ocean_rnd to be initialised");
            map_ocean_temp(noise, out, x, z, w, h);
        }

        LayerOp::OceanMix => {
            let p = node.p.expect("OceanMix needs a biome parent (p)");
            let p2 = node.p2.expect("OceanMix needs an ocean parent (p2)");
            let mut ocean = vec![Biome::NONE; w * h];
            gen_area(stack, p2, &mut ocean, x, z, w, h);
            let (lx0, lx1, lz0, lz1) = ocean_land_bbox(&ocean, w, h);
            let lw = (lx1 - lx0) as usize;
            let lh = (lz1 - lz0) as usize;
            let mut land = vec![Biome::NONE; lw * lh];
            gen_area(stack, p, &mut land, x + lx0, z + lz0, lw, lh);
            map_ocean_mix(&ocean, &land, out, w, h, lx0, lz0, lw, lh);
        }

        LayerOp::OceanMixMod => {
            let p = node.p.expect("OceanMixMod needs a land parent (p)");
            let p2 = node.p2.expect("OceanMixMod needs an ocean parent (p2)");
            let mut ocean = vec![Biome::NONE; w * h];
            gen_area(stack, p2, &mut ocean, x, z, w, h);
            let mut land = vec![Biome::NONE; w * h];
            gen_area(stack, p, &mut land, x, z, w, h);
            map_ocean_mix_mod(&ocean, &land, out, w, h);
        }

        LayerOp::Voronoi114 | LayerOp::Voronoi => {
            let p = node.p.expect("Voronoi needs a parent");
            let sx = x - 2;
            let sz = z - 2;
            let parent_x = sx >> 2;
            let parent_z = sz >> 2;
            let parent_w = (((sx + w as i32) >> 2) - parent_x + 2) as usize;
            let parent_h = (((sz + h as i32) >> 2) - parent_z + 2) as usize;
            let mut parent = vec![Biome::NONE; parent_w * parent_h];
            gen_area(
                stack,
                p,
                &mut parent,
                parent_x,
                parent_z,
                parent_w,
                parent_h,
            );
            if matches!(node.op, LayerOp::Voronoi) {
                map_voronoi(
                    node.start_salt,
                    &parent,
                    parent_x,
                    parent_z,
                    parent_w,
                    parent_h,
                    out,
                    x,
                    z,
                    w,
                    h,
                );
            } else {
                map_voronoi114(
                    node.start_salt,
                    node.start_seed,
                    &parent,
                    parent_x,
                    parent_z,
                    parent_w,
                    parent_h,
                    out,
                    x,
                    z,
                    w,
                    h,
                );
            }
        }
    }
}

/// Walk the layer DAG from `from` upstream, applying each layer's
/// parent-area formula. Returns the area `(x, z, w, h)` that
/// `target` must produce so that `from`'s `(x, z, w, h)` output can
/// be computed. Returns `None` if `target` is not reachable through
/// the primary `p` parent chain. Mirrors what cubiomes' chained
/// `getMap` calls request from each upstream layer.
#[must_use]
pub fn compute_upstream_area(
    stack: &LayerStack,
    from: LayerId,
    target: LayerId,
    x: i32,
    z: i32,
    w: usize,
    h: usize,
) -> Option<(i32, i32, usize, usize)> {
    let mut cur = from;
    let mut cx = x;
    let mut cz = z;
    let mut cw = w as i32;
    let mut ch = h as i32;
    loop {
        if cur == target {
            return Some((cx, cz, cw as usize, ch as usize));
        }
        let node = &stack.layers[cur.as_index()];
        let (px, pz, pw, ph) = match node.op {
            LayerOp::Zoom | LayerOp::ZoomFuzzy => {
                let p_x = cx >> 1;
                let p_z = cz >> 1;
                let p_w = ((cx + cw) >> 1) - p_x + 1;
                let p_h = ((cz + ch) >> 1) - p_z + 1;
                (p_x, p_z, p_w, p_h)
            }
            LayerOp::Land
            | LayerOp::LandB18
            | LayerOp::Land16
            | LayerOp::Snow
            | LayerOp::Snow16
            | LayerOp::Island
            | LayerOp::Cool
            | LayerOp::Heat
            | LayerOp::Mushroom
            | LayerOp::DeepOcean
            | LayerOp::BiomeEdge
            | LayerOp::Shore
            | LayerOp::River
            | LayerOp::Smooth
            | LayerOp::Hills => (cx - 1, cz - 1, cw + 2, ch + 2),
            LayerOp::Voronoi => {
                // Voronoi114 (zoom=4): parent area is roughly (w >> 2) + 2.
                let p_x = (cx - 2) >> 2;
                let p_z = (cz - 2) >> 2;
                let p_w = ((cx + cw - 2) >> 2) - p_x + 2;
                let p_h = ((cz + ch - 2) >> 2) - p_z + 2;
                (p_x, p_z, p_w, p_h)
            }
            LayerOp::OceanMix => (cx - 8, cz - 8, cw + 16, ch + 16),
            // Pass-through layers: Continent, Special, Biome, Bamboo,
            // Noise, SwampRiver, Sunflower, RiverMix, OceanTemp.
            _ => (cx, cz, cw, ch),
        };
        cx = px;
        cz = pz;
        cw = pw;
        ch = ph;
        cur = node.p?;
    }
}

#[cfg(test)]
mod tests {
    use super::super::stack::{LayerStack, set_layer_seed, setup_layer_stack};
    use super::*;
    use crate::mc_version::MCVersion;

    #[test]
    fn gen_area_at_continent4096_runs() {
        let mut stack = Box::new(LayerStack::new());
        setup_layer_stack(&mut stack, MCVersion::V1_18, false);
        let entry = LayerId::Continent4096;
        set_layer_seed(&mut stack, entry, 0xdead_beef);
        let mut out = vec![Biome::NONE; 8 * 8];
        gen_area(&stack, entry, &mut out, 0, 0, 8, 8);
        assert!(out.iter().any(|b| *b != Biome::NONE));
    }
}
