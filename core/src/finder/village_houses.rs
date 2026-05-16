//! Pre-1.14 Village house-count generator. Bit-exact port of
//! cubiomes' `getHouseList`.
//!
//! The function counts the number of each house type a Village in
//! `(chunk_x, chunk_z)` will be assigned during world generation
//! (MC 1.10 – 1.13 only — 1.14+ uses the `StructureVariant` path).
//! It also returns the post-call RNG state so callers can chain it
//! with subsequent draws.

#![allow(
    clippy::missing_panics_doc,
    clippy::identity_op,
    clippy::erasing_op
)]

use crate::finder::population_seed::chunk_generate_rng;

/// House-type indices into the output array. Mirrors the anonymous
/// enum in `finders.h` (`HouseSmall` = 0, …, `HouseLarge` = 8).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum HouseType {
    HouseSmall = 0,
    Church = 1,
    Library = 2,
    WoodHut = 3,
    Butcher = 4,
    FarmLarge = 5,
    FarmSmall = 6,
    Blacksmith = 7,
    HouseLarge = 8,
}

/// Total number of house-type slots. Mirrors `HOUSE_NUM`.
pub const HOUSE_NUM: usize = 9;

/// Build a Village house-count list for `(seed, chunk_x, chunk_z)`.
/// Returns the post-call `JavaRng` state's raw seed (for callers
/// that want to chain further draws), and writes nine counts into
/// `houses` indexed by [`HouseType`].
///
/// Bit-exact port of cubiomes' `getHouseList`. Note the per-type
/// `(min, max)` bounds (inclusive) — cubiomes uses
/// `nextInt(rng, max - min + 1) + min`.
pub fn get_house_list(seed: u64, chunk_x: i32, chunk_z: i32) -> ([i32; HOUSE_NUM], u64) {
    let mut rng = chunk_generate_rng(seed, chunk_x, chunk_z);
    rng.skip_n(1);
    let mut houses = [0_i32; HOUSE_NUM];
    houses[HouseType::HouseSmall as usize] = rng.next_int(4 - 2 + 1) + 2;
    houses[HouseType::Church as usize] = rng.next_int(1 - 0 + 1);
    houses[HouseType::Library as usize] = rng.next_int(2 - 0 + 1);
    houses[HouseType::WoodHut as usize] = rng.next_int(5 - 2 + 1) + 2;
    houses[HouseType::Butcher as usize] = rng.next_int(2 - 0 + 1);
    houses[HouseType::FarmLarge as usize] = rng.next_int(4 - 1 + 1) + 1;
    houses[HouseType::FarmSmall as usize] = rng.next_int(4 - 2 + 1) + 2;
    houses[HouseType::Blacksmith as usize] = rng.next_int(1 - 0 + 1);
    houses[HouseType::HouseLarge as usize] = rng.next_int(3 - 0 + 1);
    (houses, rng.raw_seed())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn house_counts_in_expected_ranges() {
        // Spot-check that counts stay within the documented bounds.
        let (houses, _) = get_house_list(0xdead_beef, 0, 0);
        assert!((2..=4).contains(&houses[HouseType::HouseSmall as usize]));
        assert!((0..=1).contains(&houses[HouseType::Church as usize]));
        assert!((0..=2).contains(&houses[HouseType::Library as usize]));
        assert!((2..=5).contains(&houses[HouseType::WoodHut as usize]));
        assert!((0..=2).contains(&houses[HouseType::Butcher as usize]));
        assert!((1..=4).contains(&houses[HouseType::FarmLarge as usize]));
        assert!((2..=4).contains(&houses[HouseType::FarmSmall as usize]));
        assert!((0..=1).contains(&houses[HouseType::Blacksmith as usize]));
        assert!((0..=3).contains(&houses[HouseType::HouseLarge as usize]));
    }
}
