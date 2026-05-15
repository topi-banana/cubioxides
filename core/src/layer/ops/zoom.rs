//! `mapZoom` and `mapZoomFuzzy` — 2x upscaling layers.
//!
//! Both functions read a `(parent_w, parent_h)` grid of parent biome IDs
//! and emit a `(w, h)` window into a `(parent_w * 2, parent_h * 2)`
//! zoomed grid. The two differ only in how the centre cell is picked
//! when the surrounding 2x2 corners disagree: `mapZoomFuzzy` picks one
//! of the four corners at random; `mapZoom` picks the majority among
//! the four (`select4`).
//!
//! In cubiomes the layer reuses the caller's `out` buffer as scratch for
//! the upscaled grid (`buf = out + pW * pH`). Rust's borrow checker
//! disagrees with that aliasing, so this port allocates a temporary
//! `Vec` for `buf` and copies the requested window back out at the end.

#![allow(clippy::many_single_char_names)] // names mirror cubiomes/layers.c

use crate::biome::Biome;

/// LCG constants used inside the 2x zoom logic (see `cubiomes/layers.c`).
const ZOOM_MUL: u32 = 1_284_865_837;
const ZOOM_ADD: u32 = 4_150_755_663;

/// `mapZoomFuzzy` — random-corner variant of the 2x upscaling layer.
///
/// `parent[0..parent_w * parent_h]` holds the parent layer's output for
/// the rectangle whose top-left corner is `(parent_x, parent_z)`. The
/// caller's `out[0..w * h]` is filled with the requested zoomed window
/// starting at `(x, z)`.
///
/// Caller must pass `parent_w >= ((x + w) >> 1) - (x >> 1) + 1` (i.e.
/// at least `(w + 2)/2` columns).
#[allow(clippy::too_many_arguments)]
pub fn map_zoom_fuzzy(
    start_salt: u64,
    start_seed: u64,
    parent: &[Biome],
    parent_x: i32,
    parent_z: i32,
    parent_w: usize,
    parent_h: usize,
    out: &mut [Biome],
    x: i32,
    z: i32,
    w: usize,
    h: usize,
) {
    zoom_impl(
        start_salt,
        start_seed,
        parent,
        parent_x,
        parent_z,
        parent_w,
        parent_h,
        out,
        x,
        z,
        w,
        h,
        Zoom::Fuzzy,
    );
}

/// `mapZoom` — majority-vote variant of the 2x upscaling layer.
///
/// Identical to `map_zoom_fuzzy` except that the diagonal centre cell
/// is chosen by the `select4` rule rather than uniformly at random.
#[allow(clippy::too_many_arguments)]
pub fn map_zoom(
    start_salt: u64,
    start_seed: u64,
    parent: &[Biome],
    parent_x: i32,
    parent_z: i32,
    parent_w: usize,
    parent_h: usize,
    out: &mut [Biome],
    x: i32,
    z: i32,
    w: usize,
    h: usize,
) {
    zoom_impl(
        start_salt,
        start_seed,
        parent,
        parent_x,
        parent_z,
        parent_w,
        parent_h,
        out,
        x,
        z,
        w,
        h,
        Zoom::Majority,
    );
}

#[derive(Copy, Clone)]
enum Zoom {
    Fuzzy,
    Majority,
}

#[allow(clippy::too_many_arguments)]
fn zoom_impl(
    start_salt: u64,
    start_seed: u64,
    parent: &[Biome],
    parent_x: i32,
    parent_z: i32,
    parent_w: usize,
    parent_h: usize,
    out: &mut [Biome],
    x: i32,
    z: i32,
    w: usize,
    h: usize,
    mode: Zoom,
) {
    assert!(
        parent.len() >= parent_w * parent_h,
        "zoom: parent slice too small"
    );
    assert!(out.len() >= w * h, "zoom: output slice too small");

    // Upscaled scratch buffer: (parent_w * 2) × (parent_h * 2) cells.
    let new_w = parent_w * 2;
    let new_h = parent_h * 2;
    let mut buf = vec![Biome::NONE; new_w * new_h];

    let st = start_salt as u32;
    let ss = start_seed as u32;

    // Iterate one cell short of the parent edge: the inner body needs to
    // read (i+1, j+1) and the rightmost / bottom cells of `buf` are
    // truncated by the final copy anyway.
    let j_lim = parent_h.saturating_sub(1);
    let i_lim = parent_w.saturating_sub(1);

    for j in 0..j_lim {
        let mut idx = (j * 2) * new_w;
        let mut v00 = parent[j * parent_w].id();
        let mut v01 = parent[(j + 1) * parent_w].id();

        for i in 0..i_lim {
            let v10 = parent[(i + 1) + j * parent_w].id();
            let v11 = parent[(i + 1) + (j + 1) * parent_w].id();

            if v00 == v01 && v00 == v10 && v00 == v11 {
                let cell = Biome(v00);
                buf[idx] = cell;
                buf[idx + 1] = cell;
                buf[idx + new_w] = cell;
                buf[idx + new_w + 1] = cell;
                idx += 2;
                v00 = v10;
                v01 = v11;
                continue;
            }

            let chunk_x = ((i as i32) + parent_x).wrapping_mul(2);
            let chunk_z = ((j as i32) + parent_z).wrapping_mul(2);

            let mut cs: u32 = ss;
            cs = cs.wrapping_add(chunk_x as u32);
            cs = cs.wrapping_mul(cs.wrapping_mul(ZOOM_MUL).wrapping_add(ZOOM_ADD));
            cs = cs.wrapping_add(chunk_z as u32);
            cs = cs.wrapping_mul(cs.wrapping_mul(ZOOM_MUL).wrapping_add(ZOOM_ADD));
            cs = cs.wrapping_add(chunk_x as u32);
            cs = cs.wrapping_mul(cs.wrapping_mul(ZOOM_MUL).wrapping_add(ZOOM_ADD));
            cs = cs.wrapping_add(chunk_z as u32);

            buf[idx] = Biome(v00);
            buf[idx + new_w] = Biome(if ((cs >> 24) & 1) != 0 { v01 } else { v00 });
            idx += 1;

            cs = cs.wrapping_mul(cs.wrapping_mul(ZOOM_MUL).wrapping_add(ZOOM_ADD));
            cs = cs.wrapping_add(st);
            buf[idx] = Biome(if ((cs >> 24) & 1) != 0 { v10 } else { v00 });

            buf[idx + new_w] = Biome(match mode {
                Zoom::Fuzzy => {
                    cs = cs.wrapping_mul(cs.wrapping_mul(ZOOM_MUL).wrapping_add(ZOOM_ADD));
                    cs = cs.wrapping_add(st);
                    fuzzy_pick(cs, v00, v01, v10, v11)
                }
                Zoom::Majority => select4(cs, st, v00, v01, v10, v11),
            });
            idx += 1;

            v00 = v10;
            v01 = v11;
        }
    }

    // Final window copy: out[j*w + i] = buf[(j + z_off) * new_w + i + x_off].
    let z_off = (z & 1) as usize;
    let x_off = (x & 1) as usize;
    for j in 0..h {
        let src_start = (j + z_off) * new_w + x_off;
        let dst_start = j * w;
        out[dst_start..dst_start + w].copy_from_slice(&buf[src_start..src_start + w]);
    }
}

#[inline]
fn fuzzy_pick(cs: u32, v00: i32, v01: i32, v10: i32, v11: i32) -> i32 {
    let r = (cs >> 24) & 3;
    match r {
        0 => v00,
        1 => v10,
        2 => v01,
        _ => v11,
    }
}

#[inline]
fn select4(cs: u32, st: u32, v00: i32, v01: i32, v10: i32, v11: i32) -> i32 {
    let cv00 = i32::from(v00 == v10) + i32::from(v00 == v01) + i32::from(v00 == v11);
    let cv10 = i32::from(v10 == v01) + i32::from(v10 == v11);
    let cv01 = i32::from(v01 == v11);
    if cv00 > cv10 && cv00 > cv01 {
        v00
    } else if cv10 > cv00 {
        v10
    } else if cv01 > cv00 {
        v01
    } else {
        let mut cs = cs;
        cs = cs.wrapping_mul(cs.wrapping_mul(ZOOM_MUL).wrapping_add(ZOOM_ADD));
        cs = cs.wrapping_add(st);
        fuzzy_pick(cs, v00, v01, v10, v11)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uniform_parent(value: i32, w: usize, h: usize) -> Vec<Biome> {
        vec![Biome(value); w * h]
    }

    #[test]
    fn uniform_parent_yields_uniform_output_fuzzy() {
        let parent = uniform_parent(5, 4, 4);
        let mut out = vec![Biome::NONE; 4 * 4];
        map_zoom_fuzzy(0, 0, &parent, 0, 0, 4, 4, &mut out, 0, 0, 4, 4);
        for &b in &out {
            assert_eq!(b, Biome(5));
        }
    }

    #[test]
    fn uniform_parent_yields_uniform_output_majority() {
        let parent = uniform_parent(7, 4, 4);
        let mut out = vec![Biome::NONE; 4 * 4];
        map_zoom(0, 0, &parent, 0, 0, 4, 4, &mut out, 0, 0, 4, 4);
        for &b in &out {
            assert_eq!(b, Biome(7));
        }
    }

    #[test]
    fn select4_picks_majority_when_three_agree() {
        // v00 == v10 == v01, v11 differs → cv00 = 2, cv10 = 1, cv01 = 0
        // → cv00 is unique max → v00 selected without consulting cs.
        let v = select4(0, 0, 1, 1, 1, 2);
        assert_eq!(v, 1);
    }

    #[test]
    fn select4_falls_back_to_random_on_all_distinct() {
        // All four different → cv00 = cv10 = cv01 = 0 → random pick from cs.
        let v = select4(0x0000_0000, 0, 1, 2, 3, 4);
        // r = 0 → v00 = 1
        assert_eq!(v, 1);
    }
}
