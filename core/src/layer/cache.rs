//! Layer-DAG cache sizing.
//!
//! Ports cubiomes' `getMaxArea` / `getMinLayerCacheSize` from
//! `generator.c`. Walks parent pointers (`p`, `p2`) recursively to
//! determine the worst-case buffer size and array extents needed
//! to evaluate a given (sx, sz) query at the entry layer.

use crate::layer::stack::{LayerId, LayerStack};

/// Recursive helper — accumulates the buffer size needed for
/// temporary copies, plus the maximum `(area_x, area_z)` extent
/// ever touched up the DAG.
fn get_max_area(
    stack: &LayerStack,
    layer: Option<LayerId>,
    mut area_x: i32,
    mut area_z: i32,
    max_x: &mut i32,
    max_z: &mut i32,
    siz: &mut usize,
) {
    let Some(layer_id) = layer else { return };
    let node = &stack.layers[layer_id as usize];

    area_x += i32::from(node.edge);
    area_z += i32::from(node.edge);

    // multi-layers and zoom-layers use a temporary copy of their parent area
    if node.p2.is_some() || node.zoom != 1 {
        *siz += (area_x as usize) * (area_z as usize);
    }

    if area_x > *max_x {
        *max_x = area_x;
    }
    if area_z > *max_z {
        *max_z = area_z;
    }

    if node.zoom == 2 {
        area_x >>= 1;
        area_z >>= 1;
    } else if node.zoom == 4 {
        area_x >>= 2;
        area_z >>= 2;
    }

    get_max_area(stack, node.p, area_x, area_z, max_x, max_z, siz);
    if node.p2.is_some() {
        get_max_area(stack, node.p2, area_x, area_z, max_x, max_z, siz);
    }
}

/// Returns the minimum cache size (in `i32` slots) needed to evaluate
/// a `size_x × size_z` query at `entry` through the layer DAG.
///
/// Bit-exact port of cubiomes' `getMinLayerCacheSize`.
#[must_use]
pub fn get_min_layer_cache_size(
    stack: &LayerStack,
    entry: LayerId,
    size_x: i32,
    size_z: i32,
) -> usize {
    let mut max_x = size_x;
    let mut max_z = size_z;
    let mut bufsiz: usize = 0;
    get_max_area(
        stack,
        Some(entry),
        size_x,
        size_z,
        &mut max_x,
        &mut max_z,
        &mut bufsiz,
    );
    bufsiz + (max_x as usize) * (max_z as usize)
}
