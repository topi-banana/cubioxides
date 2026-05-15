//! Truncated SHA-256 used to seed the 1.15+ Voronoi layer.
//!
//! Bit-exact port of cubiomes' `getVoronoiSHA` (`layers.c`). Hashes the
//! 8 bytes of a `u64` world seed with a single SHA-256 compression
//! round and returns the byte-swapped first 64 bits of the result.
//! Despite the name, this is **not** a full SHA-256 implementation —
//! the message is a single padded block and only `a0` and `a1` are
//! returned. Use a vetted crate for any real cryptography.

const K: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

const B: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];

/// Hash an 8-byte world seed with a single SHA-256 compression round
/// and return the truncated, byte-swapped 64-bit digest, matching
/// cubiomes' `getVoronoiSHA`.
#[must_use]
pub const fn voronoi_sha(seed: u64) -> u64 {
    let mut m = [0u32; 64];
    m[0] = (seed as u32).swap_bytes();
    m[1] = ((seed >> 32) as u32).swap_bytes();
    m[2] = 0x8000_0000;
    // m[3..15] is zero-initialized.
    m[15] = 0x0000_0040;

    let mut i = 16;
    while i < 64 {
        let s0_x = m[i - 15];
        let s0 = s0_x.rotate_right(7) ^ s0_x.rotate_right(18) ^ (s0_x >> 3);
        let s1_x = m[i - 2];
        let s1 = s1_x.rotate_right(17) ^ s1_x.rotate_right(19) ^ (s1_x >> 10);
        // Mirrors cubiomes' folded form:
        //   m[i] = m[i-7] + m[i-16] + s0(m[i-15]) + s1(m[i-2])
        m[i] = m[i - 7]
            .wrapping_add(m[i - 16])
            .wrapping_add(s0)
            .wrapping_add(s1);
        i += 1;
    }

    let mut a0 = B[0];
    let mut a1 = B[1];
    let mut a2 = B[2];
    let mut a3 = B[3];
    let mut a4 = B[4];
    let mut a5 = B[5];
    let mut a6 = B[6];
    let mut a7 = B[7];

    let mut i = 0;
    while i < 64 {
        // Note: cubiomes' compression deliberately matches its own
        // hand-rolled layout and is NOT the textbook SHA-256 schedule.
        let x = a7
            .wrapping_add(K[i])
            .wrapping_add(m[i])
            .wrapping_add(a4.rotate_right(6) ^ a4.rotate_right(11) ^ a4.rotate_right(25))
            .wrapping_add((a4 & a5) ^ (!a4 & a6));
        let y = (a0.rotate_right(2) ^ a0.rotate_right(13) ^ a0.rotate_right(22))
            .wrapping_add((a0 & a1) ^ (a0 & a2) ^ (a1 & a2));

        a7 = a6;
        a6 = a5;
        a5 = a4;
        a4 = a3.wrapping_add(x);
        a3 = a2;
        a2 = a1;
        a1 = a0;
        a0 = x.wrapping_add(y);
        i += 1;
    }

    a0 = a0.wrapping_add(B[0]);
    a1 = a1.wrapping_add(B[1]);

    (a0.swap_bytes() as u64) | ((a1.swap_bytes() as u64) << 32)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `voronoi_sha(0)` and `voronoi_sha(1)` must differ — a sanity
    /// check that the message schedule isn't collapsing inputs. The
    /// cross-check against cubiomes' reference values lives in the
    /// fixture-driven `voronoi_sha_parity.rs` integration test, which
    /// covers thousands of seeds.
    #[test]
    fn distinct_seeds_yield_distinct_digests() {
        assert_ne!(voronoi_sha(0), voronoi_sha(1));
        assert_ne!(voronoi_sha(0), voronoi_sha(2));
        assert_ne!(voronoi_sha(0), voronoi_sha(u64::MAX));
    }

    #[test]
    fn deterministic() {
        for seed in [0u64, 1, 2, 0xdead_beef, u64::MAX].iter().copied() {
            assert_eq!(voronoi_sha(seed), voronoi_sha(seed));
        }
    }
}
