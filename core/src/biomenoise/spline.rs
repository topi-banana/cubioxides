//! 1.18+ continentalness / erosion / ridges / weirdness spline stack.
//!
//! Bit-exact Rust port of cubiomes' spline machinery from
//! `biomenoise.c` (`createSpline_38219`, `createFlatOffsetSpline`,
//! `createLandSpline`, `initBiomeNoise`-internal `createFixSpline` /
//! `addSplineVal`, and `getSpline`). The tree is stored as a `Vec`
//! of [`SplineNode`]s with child references encoded as indices into
//! that vector — cubiomes uses two fixed-size arenas (`stack[42]`,
//! `fstack[151]`) and intrusive `Spline *` pointers, but the
//! topology is identical.

#![allow(clippy::many_single_char_names)]

/// Climate axis tag carried by every branch node, matching cubiomes'
/// `enum { SP_CONTINENTALNESS, SP_EROSION, SP_RIDGES, SP_WEIRDNESS }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SplineAxis {
    /// Continentalness — driven by `vals[0]`.
    Continentalness = 0,
    /// Erosion — driven by `vals[1]`.
    Erosion = 1,
    /// Ridges (i.e. `-3 * (|w| - 2/3| - 1/3)`) — driven by `vals[2]`.
    Ridges = 2,
    /// Weirdness — driven by `vals[3]`.
    Weirdness = 3,
}

/// A branch (non-leaf) spline node. Carries up to 12 child entries
/// in three parallel arrays.
#[derive(Debug, Clone)]
pub struct SplineBranch {
    /// Climate axis indexed into the caller's `vals[4]`.
    pub typ: SplineAxis,
    /// Number of populated children (`<= 12`).
    pub len: u8,
    /// Knot locations along the chosen axis.
    pub loc: [f32; 12],
    /// Tangents at each knot.
    pub der: [f32; 12],
    /// Child node indices into [`SplineStack::nodes`].
    pub val: [u32; 12],
}

/// Either a fixed-value leaf or a branch. Mirrors cubiomes' polymorphic
/// `(Spline*)` / `(FixSpline*)` pointers via a sum type.
#[derive(Debug, Clone)]
pub enum SplineNode {
    /// Constant value leaf.
    Fix(f32),
    /// Recursive branch.
    Branch(SplineBranch),
}

/// All spline nodes for a single MC-version build, owned by value.
#[derive(Debug, Clone, Default)]
pub struct SplineStack {
    /// Flat node arena. Index `0` is reserved for the root branch.
    pub nodes: Vec<SplineNode>,
}

impl SplineStack {
    /// Empty stack.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a fixed-value leaf and return its index.
    fn push_fix(&mut self, val: f32) -> u32 {
        let id = self.nodes.len() as u32;
        self.nodes.push(SplineNode::Fix(val));
        id
    }

    /// Append an empty branch with axis `typ` and return its index.
    /// Children must be added later with [`Self::add_child`].
    fn push_branch(&mut self, typ: SplineAxis) -> u32 {
        let id = self.nodes.len() as u32;
        self.nodes.push(SplineNode::Branch(SplineBranch {
            typ,
            len: 0,
            loc: [0.0; 12],
            der: [0.0; 12],
            val: [0; 12],
        }));
        id
    }

    /// Append `(loc, val, der)` to branch `branch_id`'s child list.
    /// Panics if the target is not a branch or already full.
    fn add_child(&mut self, branch_id: u32, loc: f32, val: u32, der: f32) {
        let SplineNode::Branch(sp) = &mut self.nodes[branch_id as usize] else {
            panic!("add_child target must be a branch");
        };
        let i = sp.len as usize;
        assert!(i < 12, "spline branch overflow");
        sp.loc[i] = loc;
        sp.der[i] = der;
        sp.val[i] = val;
        sp.len += 1;
    }
}

/// `getSpline(sp, vals)` — recursively evaluate the spline tree
/// rooted at `id` against the four climate inputs in `vals`.
/// Numerical behaviour matches cubiomes' mixed-precision arithmetic:
/// internal `lerp` calls go through `f64` while pointwise additions
/// stay `f32`.
#[must_use]
pub fn sample_spline(stack: &SplineStack, id: u32, vals: &[f32; 4]) -> f32 {
    match &stack.nodes[id as usize] {
        SplineNode::Fix(v) => *v,
        SplineNode::Branch(sp) => sample_branch(stack, sp, vals),
    }
}

fn sample_branch(stack: &SplineStack, sp: &SplineBranch, vals: &[f32; 4]) -> f32 {
    let f = vals[sp.typ as usize];
    let len = sp.len as usize;
    debug_assert!(len > 0 && len < 12, "branch len out of range");

    // i = first index with loc[i] >= f, mirroring cubiomes' `for ... if`.
    let mut i = len;
    for j in 0..len {
        if sp.loc[j] >= f {
            i = j;
            break;
        }
    }

    if i == 0 || i == len {
        let j = i.saturating_sub(1);
        let v = sample_spline(stack, sp.val[j], vals);
        return v + sp.der[j] * (f - sp.loc[j]);
    }

    let sp1 = sp.val[i - 1];
    let sp2 = sp.val[i];
    let g = sp.loc[i - 1];
    let h = sp.loc[i];
    let k = (f - g) / (h - g);
    let l = sp.der[i - 1];
    let m = sp.der[i];
    let n = sample_spline(stack, sp1, vals);
    let o = sample_spline(stack, sp2, vals);
    let p = l * (h - g) - (o - n);
    let q = -m * (h - g) + (o - n);
    // cubiomes' formula: lerp(k, n, o) + k * (1.0F - k) * lerp(k, p, q),
    // where lerp() returns double and the multiplication mixes f32 / f64.
    let lerp_no = lerp_f64(k, n, o);
    let lerp_pq = lerp_f64(k, p, q);
    let kk = k * (1.0_f32 - k);
    (lerp_no + f64::from(kk) * lerp_pq) as f32
}

#[inline]
fn lerp_f64(t: f32, a: f32, b: f32) -> f64 {
    f64::from(a) + f64::from(t) * (f64::from(b) - f64::from(a))
}

fn get_offset_value(weirdness: f32, continentalness: f32) -> f32 {
    let f0 = 1.0_f32 - (1.0_f32 - continentalness) * 0.5_f32;
    let f1 = 0.5_f32 * (1.0_f32 - continentalness);
    let f2 = (weirdness + 1.17_f32) * 0.460_829_47_f32;
    let off = f2 * f0 - f1;
    if weirdness < -0.7_f32 {
        if off > -0.2222_f32 { off } else { -0.2222_f32 }
    } else if off > 0.0 {
        off
    } else {
        0.0
    }
}

fn create_spline_38219(ss: &mut SplineStack, f: f32, bl: bool) -> u32 {
    let sp = ss.push_branch(SplineAxis::Ridges);

    let i = get_offset_value(-1.0, f);
    let k = get_offset_value(1.0, f);
    let l_init = 1.0_f32 - (1.0_f32 - f) * 0.5_f32;
    let u_init = 0.5_f32 * (1.0_f32 - f);
    let l = u_init / (0.460_829_47_f32 * l_init) - 1.17_f32;

    if -0.65_f32 < l && l < 1.0_f32 {
        let u = get_offset_value(-0.65, f);
        let p = get_offset_value(-0.75, f);
        let q = (p - i) * 4.0;
        let r = get_offset_value(l, f);
        let s = (k - r) / (1.0 - l);

        let fix_i = ss.push_fix(i);
        ss.add_child(sp, -1.0, fix_i, q);
        let fix_p = ss.push_fix(p);
        ss.add_child(sp, -0.75, fix_p, 0.0);
        let fix_u = ss.push_fix(u);
        ss.add_child(sp, -0.65, fix_u, 0.0);
        let fix_r1 = ss.push_fix(r);
        ss.add_child(sp, l - 0.01, fix_r1, 0.0);
        let fix_r2 = ss.push_fix(r);
        ss.add_child(sp, l, fix_r2, s);
        let fix_k = ss.push_fix(k);
        ss.add_child(sp, 1.0, fix_k, s);
    } else {
        let u = (k - i) * 0.5;
        if bl {
            let v_i = if i > 0.2 { i } else { 0.2 };
            let fix_i = ss.push_fix(v_i);
            ss.add_child(sp, -1.0, fix_i, 0.0);
            // `lerp(0.5F, i, k)` returns double in cubiomes; narrow to f32 at store.
            let mid = lerp_f64(0.5, i, k) as f32;
            let fix_mid = ss.push_fix(mid);
            ss.add_child(sp, 0.0, fix_mid, u);
        } else {
            let fix_i = ss.push_fix(i);
            ss.add_child(sp, -1.0, fix_i, u);
        }
        let fix_k = ss.push_fix(k);
        ss.add_child(sp, 1.0, fix_k, u);
    }
    sp
}

fn create_flat_offset_spline(
    ss: &mut SplineStack,
    f: f32,
    g: f32,
    h: f32,
    i: f32,
    j: f32,
    k: f32,
) -> u32 {
    let sp = ss.push_branch(SplineAxis::Ridges);

    let l_raw = 0.5_f32 * (g - f);
    let l = if l_raw < k { k } else { l_raw };
    let m = 5.0_f32 * (h - g);

    let fix_f = ss.push_fix(f);
    ss.add_child(sp, -1.0, fix_f, l);
    let fix_g = ss.push_fix(g);
    let der_g = if l < m { l } else { m };
    ss.add_child(sp, -0.4, fix_g, der_g);
    let fix_h = ss.push_fix(h);
    ss.add_child(sp, 0.0, fix_h, m);
    let fix_i = ss.push_fix(i);
    ss.add_child(sp, 0.4, fix_i, 2.0 * (i - h));
    let fix_j = ss.push_fix(j);
    ss.add_child(sp, 1.0, fix_j, 0.7 * (j - i));
    sp
}

#[allow(clippy::similar_names, clippy::too_many_arguments)]
fn create_land_spline(
    ss: &mut SplineStack,
    f: f32,
    g: f32,
    h: f32,
    i: f32,
    j: f32,
    k: f32,
    bl: bool,
) -> u32 {
    let sp1 = create_spline_38219(ss, lerp_f64(i, 0.6, 1.5) as f32, bl);
    let sp2 = create_spline_38219(ss, lerp_f64(i, 0.6, 1.0) as f32, bl);
    let sp3 = create_spline_38219(ss, i, bl);
    let ih = 0.5_f32 * i;
    let sp4 = create_flat_offset_spline(ss, f - 0.15, ih, ih, ih, i * 0.6, 0.5);
    let sp5 = create_flat_offset_spline(ss, f, j * i, g * i, ih, i * 0.6, 0.5);
    let sp6 = create_flat_offset_spline(ss, f, j, j, g, h, 0.5);
    let sp7 = create_flat_offset_spline(ss, f, j, j, g, h, 0.5);

    let sp8 = ss.push_branch(SplineAxis::Ridges);
    let fix_f = ss.push_fix(f);
    ss.add_child(sp8, -1.0, fix_f, 0.0);
    ss.add_child(sp8, -0.4, sp6, 0.0);
    let fix_hp = ss.push_fix(h + 0.07);
    ss.add_child(sp8, 0.0, fix_hp, 0.0);

    let sp9 = create_flat_offset_spline(ss, -0.02, k, k, g, h, 0.0);

    let sp = ss.push_branch(SplineAxis::Erosion);
    ss.add_child(sp, -0.85, sp1, 0.0);
    ss.add_child(sp, -0.7, sp2, 0.0);
    ss.add_child(sp, -0.4, sp3, 0.0);
    ss.add_child(sp, -0.35, sp4, 0.0);
    ss.add_child(sp, -0.1, sp5, 0.0);
    ss.add_child(sp, 0.2, sp6, 0.0);
    if bl {
        ss.add_child(sp, 0.4, sp7, 0.0);
        ss.add_child(sp, 0.45, sp8, 0.0);
        ss.add_child(sp, 0.55, sp8, 0.0);
        ss.add_child(sp, 0.58, sp7, 0.0);
    }
    ss.add_child(sp, 0.7, sp9, 0.0);
    sp
}

/// Build the canonical 1.18+ Overworld spline stack and return
/// `(stack, root_id)`. Mirrors `initBiomeNoise(bn, mc)` modulo the
/// per-`bn` storage (the spline shape is independent of MC version).
#[must_use]
pub fn build_overworld_spline() -> (SplineStack, u32) {
    let mut ss = SplineStack::new();
    let sp = ss.push_branch(SplineAxis::Continentalness);

    let sp1 = create_land_spline(&mut ss, -0.15, 0.00, 0.0, 0.1, 0.00, -0.03, false);
    let sp2 = create_land_spline(&mut ss, -0.10, 0.03, 0.1, 0.1, 0.01, -0.03, false);
    let sp3 = create_land_spline(&mut ss, -0.10, 0.03, 0.1, 0.7, 0.01, -0.03, true);
    let sp4 = create_land_spline(&mut ss, -0.05, 0.03, 0.1, 1.0, 0.01, 0.01, true);

    let fix_044 = ss.push_fix(0.044);
    ss.add_child(sp, -1.10, fix_044, 0.0);
    let fix_neg_a = ss.push_fix(-0.2222);
    ss.add_child(sp, -1.02, fix_neg_a, 0.0);
    let fix_neg_b = ss.push_fix(-0.2222);
    ss.add_child(sp, -0.51, fix_neg_b, 0.0);
    let fix_neg_c = ss.push_fix(-0.12);
    ss.add_child(sp, -0.44, fix_neg_c, 0.0);
    let fix_neg_d = ss.push_fix(-0.12);
    ss.add_child(sp, -0.18, fix_neg_d, 0.0);
    ss.add_child(sp, -0.16, sp1, 0.0);
    ss.add_child(sp, -0.15, sp1, 0.0);
    ss.add_child(sp, -0.10, sp2, 0.0);
    ss.add_child(sp, 0.25, sp3, 0.0);
    ss.add_child(sp, 1.00, sp4, 0.0);

    (ss, sp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_overworld_produces_a_tree() {
        let (ss, root) = build_overworld_spline();
        assert_eq!(root, 0);
        assert!(matches!(&ss.nodes[0], SplineNode::Branch(_)));
        // cubiomes' `Spline stack[42]` and `FixSpline fstack[151]` give
        // a hard upper bound of 193 total slots; the actual count must
        // stay below that.
        assert!(ss.nodes.len() < 200, "spline node count out of range");
    }

    #[test]
    fn sample_at_negative_extreme_is_a_fixed_value() {
        let (ss, root) = build_overworld_spline();
        // continentalness = -2.0 falls before the first knot (-1.10),
        // so cubiomes hits the i == 0 branch and returns
        // val(-1.10) + der(-1.10) * (-2.0 - (-1.10)) = 0.044 + 0 = 0.044.
        let r = sample_spline(&ss, root, &[-2.0, 0.0, 0.0, 0.0]);
        assert!((r - 0.044).abs() < 1e-6, "expected 0.044, got {r}");
    }
}
