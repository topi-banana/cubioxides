//! Numeric primitives shared by the RNG, noise, and layer modules.
//!
//! Ports of cubiomes' `lerp*` / `floordiv` / rotate helpers in
//! `cubiomes/rng.h`. The rotation functions delegate to Rust's built-in
//! `rotate_left` / `rotate_right`, which compile to a single instruction
//! on every reasonable target.

/// 64-bit left rotation by `b` bits.
///
/// Matches `rotl64` from cubiomes.
#[inline]
#[must_use]
pub const fn rotl64(x: u64, b: u32) -> u64 {
    x.rotate_left(b)
}

/// 32-bit right rotation by `b` bits.
///
/// Matches `rotr32` from cubiomes.
#[inline]
#[must_use]
pub const fn rotr32(x: u32, b: u32) -> u32 {
    x.rotate_right(b)
}

/// Floor division for signed 32-bit integers.
///
/// Matches cubiomes' `floordiv`: Rust's `/` truncates toward zero, so the
/// quotient is adjusted by `-1` when the operands have opposite signs and
/// the division is not exact.
#[inline]
#[must_use]
pub const fn floordiv(a: i32, b: i32) -> i32 {
    let q = a / b;
    let r = a % b;
    if (a ^ b) < 0 && r != 0 { q - 1 } else { q }
}

/// Linear interpolation between `from` and `to`.
///
/// Matches cubiomes' `lerp`: `from + t * (to - from)`.
#[inline]
#[must_use]
pub fn lerp(t: f64, from: f64, to: f64) -> f64 {
    from + t * (to - from)
}

/// Bilinear interpolation on a unit square.
///
/// Matches cubiomes' `lerp2`.
#[inline]
#[must_use]
pub fn lerp2(dx: f64, dy: f64, v00: f64, v10: f64, v01: f64, v11: f64) -> f64 {
    lerp(dy, lerp(dx, v00, v10), lerp(dx, v01, v11))
}

/// Trilinear interpolation on a unit cube.
///
/// Matches cubiomes' `lerp3`.
#[inline]
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn lerp3(
    dx: f64,
    dy: f64,
    dz: f64,
    v000: f64,
    v100: f64,
    v010: f64,
    v110: f64,
    v001: f64,
    v101: f64,
    v011: f64,
    v111: f64,
) -> f64 {
    let lo = lerp2(dx, dy, v000, v100, v010, v110);
    let hi = lerp2(dx, dy, v001, v101, v011, v111);
    lerp(dz, lo, hi)
}

/// 3D interpolation across 4 vertical "columns" of `[lo, hi]` pairs
/// at the corners of a unit square. Matches cubiomes' static
/// `lerp4` in `biomenoise.c`:
///
/// ```text
/// b00 = a[0] + (a[1] - a[0]) * dy
/// b01 = b[0] + (b[1] - b[0]) * dy
/// b10 = c[0] + (c[1] - c[0]) * dy
/// b11 = d[0] + (d[1] - d[0]) * dy
/// return lerp(dx, lerp(dz, b00, b10), lerp(dz, b01, b11))
/// ```
#[inline]
#[must_use]
pub fn lerp4(
    a: &[f64; 2],
    b: &[f64; 2],
    c: &[f64; 2],
    d: &[f64; 2],
    dy: f64,
    dx: f64,
    dz: f64,
) -> f64 {
    let b00 = a[0] + (a[1] - a[0]) * dy;
    let b01 = b[0] + (b[1] - b[0]) * dy;
    let b10 = c[0] + (c[1] - c[0]) * dy;
    let b11 = d[0] + (d[1] - d[0]) * dy;
    let b0 = b00 + (b10 - b00) * dz;
    let b1 = b01 + (b11 - b01) * dz;
    b0 + (b1 - b0) * dx
}

/// Linear interpolation, clamped to `[from, to]`.
///
/// Matches cubiomes' `clampedLerp`.
#[inline]
#[must_use]
pub fn clamped_lerp(t: f64, from: f64, to: f64) -> f64 {
    if t <= 0.0 {
        from
    } else if t >= 1.0 {
        to
    } else {
        lerp(t, from, to)
    }
}

/// Perlin gradient lookup by 4-bit index.
///
/// Bit-exact port of cubiomes' `indexedLerp`. The 16-way switch matches
/// the table-free dot-product variant used by the modern Perlin
/// implementation. Only the low nibble of `idx` is used. Cases 12 / 13 /
/// 14 / 15 deliberately mirror earlier ones; do not merge them.
#[inline]
#[must_use]
#[allow(clippy::match_same_arms)] // keeps the 1:1 mapping with cubiomes/noise.c
pub const fn indexed_lerp(idx: u8, a: f64, b: f64, c: f64) -> f64 {
    match idx & 0xf {
        0 => a + b,
        1 => -a + b,
        2 => a - b,
        3 => -a - b,
        4 => a + c,
        5 => -a + c,
        6 => a - c,
        7 => -a - c,
        8 => b + c,
        9 => -b + c,
        10 => b - c,
        11 => -b - c,
        12 => a + b,
        13 => -b + c,
        14 => -a + b,
        // 15 is the only remaining case after the mask above.
        _ => -b - c,
    }
}

/// Simplex noise gradient contribution.
///
/// Mirrors cubiomes' `simplexGrad`: returns zero outside the squashed
/// radius `sqrt(d)` and otherwise the `(d - r^2)^4`-scaled gradient.
#[inline]
#[must_use]
pub fn simplex_grad(idx: u8, x: f64, y: f64, z: f64, d: f64) -> f64 {
    let con = d - x * x - y * y - z * z;
    if con < 0.0 {
        return 0.0;
    }
    let con = con * con;
    con * con * indexed_lerp(idx, x, y, z)
}

/// Inverse error function via Newton's method on `erf(t) = x`.
/// Mirrors cubiomes' `inverf` from `finders.c`:
///
/// ```text
/// t = x
/// while |dt| > FLT_EPSILON:
///     dt = 0.5 * sqrt(PI) * (erf(t) - x) / exp(-t*t)
///     t -= dt
/// return t
/// ```
///
/// Uses `libm::erf` and `libm::exp` to match cubiomes' use of C
/// `<math.h>`. Results agree to within ~1 ulp on glibc-based hosts;
/// other libm implementations may differ in the last few bits.
///
/// Used by `monteCarloBiomes` to convert a confidence level into a
/// z-score (`z = sqrt(2) * inverf(confidence)`).
#[inline]
#[must_use]
pub fn inverf(x: f64) -> f64 {
    let mut t = x;
    let mut dt: f64 = 1.0;
    let sqrt_pi = core::f64::consts::PI.sqrt();
    while dt.abs() > f64::from(f32::EPSILON) {
        dt = 0.5 * sqrt_pi * (libm::erf(t) - x) / libm::exp(-t * t);
        t -= dt;
    }
    t
}

/// Wilson score interval for a binomial proportion. Returns
/// `(lo, hi)` where `n` is the total trial count, `p` is the
/// observed success ratio (`successes / n`), and `z` is the
/// confidence z-score (e.g. 1.96 for 95%).
///
/// Bit-exact port of `cubiomes/finders.c::wilson`. Uses the same
/// `+ FLT_EPSILON` margin cubiomes adds to the radius `d`. Used by
/// the upstream `monteCarloBiomes` sampling helper.
///
/// [Wilson score interval]: https://en.wikipedia.org/wiki/Binomial_proportion_confidence_interval#Wilson_score_interval
#[inline]
#[must_use]
#[allow(clippy::many_single_char_names)]
pub fn wilson(n: f64, p: f64, z: f64) -> (f64, f64) {
    let s = z * z / n;
    let t = 1.0 / (1.0 + s);
    let w = t * (p + 0.5 * s);
    // cubiomes uses C's `FLT_EPSILON` (single-precision epsilon
    // ≈ 1.192e-7), not `DBL_EPSILON`, even though `d` is a `double`.
    let d = t * z * ((p * (1.0 - p) + 0.25 * s) / n).sqrt() + f64::from(f32::EPSILON);
    (w - d, w + d)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn lerp_hits_endpoints_exactly() {
        assert_eq!(lerp(0.0, 3.0, 7.0), 3.0);
        assert_eq!(lerp(1.0, 3.0, 7.0), 7.0);
    }

    #[test]
    fn lerp_midpoint_is_average() {
        assert!((lerp(0.5, 0.0, 10.0) - 5.0).abs() < 1e-12);
    }

    #[test]
    fn lerp2_separable_components() {
        // f(dx,dy) = dx + dy when v00=0, v10=1, v01=1, v11=2.
        assert!((lerp2(0.25, 0.75, 0.0, 1.0, 1.0, 2.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn lerp3_at_corner() {
        let v = lerp3(
            0.0, 0.0, 0.0, // dx, dy, dz
            7.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        );
        assert_eq!(v, 7.0);
    }

    #[test]
    fn clamped_lerp_saturates_outside_unit_interval() {
        assert_eq!(clamped_lerp(-1.0, 3.0, 7.0), 3.0);
        assert_eq!(clamped_lerp(2.0, 3.0, 7.0), 7.0);
        assert_eq!(clamped_lerp(0.0, 3.0, 7.0), 3.0);
        assert_eq!(clamped_lerp(1.0, 3.0, 7.0), 7.0);
        assert!((clamped_lerp(0.5, 3.0, 7.0) - 5.0).abs() < 1e-12);
    }

    #[test]
    fn floordiv_matches_c_semantics() {
        // Same operand sign: identical to truncating division.
        assert_eq!(floordiv(7, 3), 2);
        assert_eq!(floordiv(-7, -3), 2);
        // Opposite signs with non-zero remainder: rounds toward -infinity.
        assert_eq!(floordiv(-7, 3), -3);
        assert_eq!(floordiv(7, -3), -3);
        // Zero dividend.
        assert_eq!(floordiv(0, 3), 0);
        // Exact division with mixed signs: no adjustment.
        assert_eq!(floordiv(-9, 3), -3);
        assert_eq!(floordiv(9, -3), -3);
    }

    #[test]
    fn rotl64_rotates_by_bits() {
        assert_eq!(rotl64(0x1234_5678_9abc_def0, 8), 0x3456_789a_bcde_f012);
        assert_eq!(rotl64(1, 1), 2);
        assert_eq!(rotl64(0x8000_0000_0000_0000, 1), 1);
        // Rotating by a full word width is a no-op.
        assert_eq!(rotl64(0xdead_beef_dead_beef, 64), 0xdead_beef_dead_beef);
    }

    #[test]
    fn rotr32_rotates_by_bits() {
        assert_eq!(rotr32(0x1234_5678, 8), 0x7812_3456);
        assert_eq!(rotr32(1, 1), 0x8000_0000);
    }

    #[test]
    fn indexed_lerp_masks_to_low_nibble() {
        // Differs only on the unmasked bits; the low nibble decides.
        let a = indexed_lerp(0b0001, 1.0, 2.0, 3.0);
        let b = indexed_lerp(0b1111_0001, 1.0, 2.0, 3.0);
        assert_eq!(a, b);
    }

    #[test]
    fn indexed_lerp_covers_every_case() {
        // Each of the 16 indices yields a distinct linear combination of
        // a, b, c (with a = 1, b = 10, c = 100 the sums are unique).
        let mut seen = std::collections::HashSet::new();
        for idx in 0..16u8 {
            let v = indexed_lerp(idx, 1.0, 10.0, 100.0);
            seen.insert(v.to_bits());
        }
        // Cases 0 / 12 yield `a + b`, 1 / 14 yield `-a + b`, 9 / 13 yield
        // `-b + c`, and 11 / 15 yield `-b - c`, so we expect 12 distinct
        // values rather than 16.
        assert_eq!(seen.len(), 12);
    }

    #[test]
    fn simplex_grad_is_zero_outside_radius() {
        // d = 0.5 means radius^2 = 0.5; any (x, y, z) outside that should
        // yield zero contribution.
        assert_eq!(simplex_grad(0, 1.0, 1.0, 1.0, 0.5), 0.0);
        assert_eq!(simplex_grad(0, 5.0, 0.0, 0.0, 0.5), 0.0);
    }

    #[test]
    fn inverf_known_values() {
        // inverf(0) = 0
        assert!(inverf(0.0).abs() < 1e-10);
        // inverf(0.5) ≈ 0.4769
        assert!((inverf(0.5) - 0.4769).abs() < 1e-3);
        // inverf(0.95) ≈ 1.3859
        assert!((inverf(0.95) - 1.3859).abs() < 1e-3);
    }

    #[test]
    fn inverf_round_trip_via_erf() {
        // erf(inverf(x)) should equal x.
        for &x in &[-0.5, -0.1, 0.0, 0.3, 0.7, 0.9] {
            let t = inverf(x);
            let back = libm::erf(t);
            assert!((back - x).abs() < 1e-7, "x={x}, inverf={t}, erf(t)={back}");
        }
    }

    #[test]
    fn wilson_known_value() {
        // For n=100, p=0.5, z=1.96 (95% CI) the Wilson score
        // interval is approximately (0.4038, 0.5962).
        let (lo, hi) = wilson(100.0, 0.5, 1.96);
        assert!((lo - 0.4038).abs() < 1e-3, "lo = {lo}");
        assert!((hi - 0.5962).abs() < 1e-3, "hi = {hi}");
    }

    #[test]
    fn wilson_extremes_bounded() {
        // p=0 means lo should be near 0 (small positive),
        // hi should be < 1.
        let (lo, hi) = wilson(50.0, 0.0, 2.0);
        assert!(lo >= 0.0 - 1e-6, "lo = {lo}");
        assert!(hi < 1.0, "hi = {hi}");
    }

    #[test]
    fn simplex_grad_nonzero_inside_radius() {
        // Inside the radius the contribution is non-zero unless the gradient
        // dotted with (x, y, z) happens to vanish.
        let v = simplex_grad(0, 0.1, 0.2, 0.0, 0.5);
        assert!(v != 0.0);
    }
}
