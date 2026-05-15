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
}
