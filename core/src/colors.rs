//! Biome → RGB color tables for visualisation. Bit-exact port of
//! cubiomes' `initBiomeColors` / `initBiomeTypeColors` from `util.c`.
//!
//! Color scheme inspired by the AMIDST project
//! (<https://github.com/toolbox4minecraft/amidst/wiki/Biome-Color-Table>)
//! with cubiomes' additions for 1.18+ and a few contrast tweaks.

#![allow(clippy::missing_panics_doc, clippy::unreadable_literal)]

/// Build the AMIDST-style 256-entry biome RGB color table. Biomes
/// not present in cubiomes' table return `[0, 0, 0]` (matching the
/// `memset(0)` initialisation in cubiomes).
///
/// Bit-exact port of cubiomes' `initBiomeColors`.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn init_biome_colors() -> [[u8; 3]; 256] {
    let mut colors = [[0u8; 3]; 256];
    let entries: &[(usize, u32)] = &[
        (0, 0x000070),   // ocean
        (1, 0x8db360),   // plains
        (2, 0xfa9418),   // desert
        (3, 0x606060),   // mountains / windswept_hills
        (4, 0x056621),   // forest
        (5, 0x0b6a5f),   // taiga
        (6, 0x07f9b2),   // swamp
        (7, 0x0000ff),   // river
        (8, 0x572526),   // nether_wastes
        (9, 0x8080ff),   // the_end
        (10, 0x7070d6),  // frozen_ocean
        (11, 0xa0a0ff),  // frozen_river
        (12, 0xffffff),  // snowy_plains / snowy_tundra
        (13, 0xa0a0a0),  // snowy_mountains
        (14, 0xff00ff),  // mushroom_fields
        (15, 0xa000ff),  // mushroom_field_shore
        (16, 0xfade55),  // beach
        (17, 0xd25f12),  // desert_hills
        (18, 0x22551c),  // wooded_hills
        (19, 0x163933),  // taiga_hills
        (20, 0x72789a),  // mountain_edge
        (21, 0x507b0a),  // jungle
        (22, 0x2c4205),  // jungle_hills
        (23, 0x60930f),  // sparse_jungle / jungle_edge
        (24, 0x000030),  // deep_ocean
        (25, 0xa2a284),  // stony_shore / stone_shore
        (26, 0xfaf0c0),  // snowy_beach
        (27, 0x307444),  // birch_forest
        (28, 0x1f5f32),  // birch_forest_hills
        (29, 0x40511a),  // dark_forest
        (30, 0x31554a),  // snowy_taiga
        (31, 0x243f36),  // snowy_taiga_hills
        (32, 0x596651),  // old_growth_pine_taiga / giant_tree_taiga
        (33, 0x454f3e),  // giant_tree_taiga_hills
        (34, 0x5b7352),  // windswept_forest / wooded_mountains
        (35, 0xbdb25f),  // savanna
        (36, 0xa79d64),  // savanna_plateau
        (37, 0xd94515),  // badlands
        (38, 0xb09765),  // wooded_badlands / wooded_badlands_plateau
        (39, 0xca8c65),  // badlands_plateau
        (40, 0x4b4bab),  // small_end_islands
        (41, 0xc9c959),  // end_midlands
        (42, 0xb5b536),  // end_highlands
        (43, 0x7070cc),  // end_barrens
        (44, 0x0000ac),  // warm_ocean
        (45, 0x000090),  // lukewarm_ocean
        (46, 0x202070),  // cold_ocean
        (47, 0x000050),  // deep_warm_ocean
        (48, 0x000040),  // deep_lukewarm_ocean
        (49, 0x202038),  // deep_cold_ocean
        (50, 0x404090),  // deep_frozen_ocean
        (51, 0x2f560f),  // seasonal_forest
        (52, 0x47840e),  // rainforest
        (53, 0x789e31),  // shrubland
        (127, 0x000000), // the_void
        (129, 0xb5db88), // sunflower_plains
        (130, 0xffbc40), // desert_lakes
        (131, 0x888888), // windswept_gravelly_hills / gravelly_mountains
        (132, 0x2d8e49), // flower_forest
        (133, 0x339287), // taiga_mountains
        (134, 0x2fffda), // swamp_hills
        (140, 0xb4dcdc), // ice_spikes
        (149, 0x78a332), // modified_jungle
        (151, 0x88bb37), // modified_jungle_edge
        (155, 0x589c6c), // old_growth_birch_forest / tall_birch_forest
        (156, 0x47875a), // tall_birch_hills
        (157, 0x687942), // dark_forest_hills
        (158, 0x597d72), // snowy_taiga_mountains
        (160, 0x818e79), // old_growth_spruce_taiga / giant_spruce_taiga
        (161, 0x6d7766), // giant_spruce_taiga_hills
        (162, 0x839b7a), // modified_gravelly_mountains
        (163, 0xe5da87), // windswept_savanna / shattered_savanna
        (164, 0xcfc58c), // shattered_savanna_plateau
        (165, 0xff6d3d), // eroded_badlands
        (166, 0xd8bf8d), // modified_wooded_badlands_plateau
        (167, 0xf2b48d), // modified_badlands_plateau
        (168, 0x849500), // bamboo_jungle
        (169, 0x5c6c04), // bamboo_jungle_hills
        (170, 0x4d3a2e), // soul_sand_valley
        (171, 0x981a11), // crimson_forest
        (172, 0x49907b), // warped_forest
        (173, 0x645f63), // basalt_deltas
        (174, 0x4e3012), // dripstone_caves
        (175, 0x283c00), // lush_caves
        (177, 0x60a445), // meadow
        (178, 0x47726c), // grove
        (179, 0xc4c4c4), // snowy_slopes
        (180, 0xdcdcc8), // jagged_peaks
        (181, 0xb0b3ce), // frozen_peaks
        (182, 0x7b8f74), // stony_peaks
        (183, 0x031f29), // deep_dark
        (184, 0x2ccc8e), // mangrove_swamp
        (185, 0xff91c8), // cherry_grove
        (186, 0x696d95), // pale_garden
    ];
    for &(id, hex) in entries {
        colors[id] = [
            ((hex >> 16) & 0xff) as u8,
            ((hex >> 8) & 0xff) as u8,
            (hex & 0xff) as u8,
        ];
    }
    colors
}

/// `initBiomeTypeColors` — 5-entry palette keyed by the climate
/// category (`Oceanic`, `Warm`, `Lush`, `Cold`, `Freezing`).
/// Mirrors cubiomes' helper of the same name.
#[must_use]
pub fn init_biome_type_colors() -> [[u8; 3]; 256] {
    let mut colors = [[0u8; 3]; 256];
    let entries: &[(usize, u32)] = &[
        // cubiomes' BiomeTempCategory enum maps to small ids.
        (0, 0x0000a0), // Oceanic
        (1, 0xffc000), // Warm
        (2, 0x00a000), // Lush
        (3, 0x606060), // Cold
        (4, 0xffffff), // Freezing
    ];
    for &(id, hex) in entries {
        colors[id] = [
            ((hex >> 16) & 0xff) as u8,
            ((hex >> 8) & 0xff) as u8,
            (hex & 0xff) as u8,
        ];
    }
    colors
}

/// `biomesToImage(pixels, biomeColors, biomes, sx, sy, pixscale, flip)`
/// — render a `(sx, sy)` biome-id grid into an RGB pixel buffer
/// using `biome_colors` as the palette. Bit-exact port of cubiomes'
/// helper of the same name.
///
/// Behavior:
/// - Each cell is replicated into a `pixscale × pixscale` square.
/// - When `flip == false`, row 0 lands at the *bottom* of the
///   output (cubiomes default for PPM output, where the natural Z+
///   direction is up). With `flip == true`, row 0 is at the top.
/// - Invalid IDs (`< 0` or `>= 256`) get the palette entry at
///   `id & 0x7f` darkened by 40 per channel (saturating to 0),
///   and the function returns `true` to flag them.
///
/// `pixels` must be at least `3 * (sx * pixscale) * (sy * pixscale)`
/// bytes long. Panics if too small.
#[allow(clippy::many_single_char_names, clippy::too_many_arguments)]
pub fn biomes_to_image(
    pixels: &mut [u8],
    biome_colors: &[[u8; 3]; 256],
    biomes: &[i32],
    sx: u32,
    sy: u32,
    pixscale: u32,
    flip: bool,
) -> bool {
    let sxu = sx as usize;
    let syu = sy as usize;
    let ps = pixscale as usize;
    assert!(biomes.len() >= sxu * syu, "biomes slice too small");
    assert!(
        pixels.len() >= 3 * sxu * syu * ps * ps,
        "pixels buffer too small"
    );
    let mut contains_invalid = false;
    for j in 0..syu {
        for i in 0..sxu {
            let id = biomes[j * sxu + i];
            let (r, g, b) = if (0..256).contains(&id) {
                let c = biome_colors[id as usize];
                (c[0], c[1], c[2])
            } else {
                contains_invalid = true;
                // cubiomes: `id & 0x7f` even for negative ids.
                let idx = (id & 0x7f) as usize;
                let c = biome_colors[idx];
                (
                    c[0].saturating_sub(40),
                    c[1].saturating_sub(40),
                    c[2].saturating_sub(40),
                )
            };
            for m in 0..ps {
                for n in 0..ps {
                    let col = ps * i + n;
                    let row = if flip {
                        ps * j + m
                    } else {
                        ps * (syu - 1 - j) + m
                    };
                    let idx = sxu * ps * row + col;
                    let pix = &mut pixels[3 * idx..3 * idx + 3];
                    pix[0] = r;
                    pix[1] = g;
                    pix[2] = b;
                }
            }
        }
    }
    contains_invalid
}

/// `savePPM(path, pixels, sx, sy)` — write an RGB pixel buffer
/// to disk as a binary [PPM P6] file. Bit-exact port of cubiomes'
/// helper of the same name.
///
/// `pixels` must contain `3 * sx * sy` bytes in row-major
/// (red, green, blue) order. Returns `Ok(())` on full success.
/// File handling errors propagate via `std::io::Error`.
///
/// Gated behind `not(wasm32)` since wasm32-unknown-unknown has no
/// filesystem; callers on wasm should use [`biomes_to_image`] and
/// pass the resulting buffer to their host environment instead.
///
/// [PPM P6]: https://en.wikipedia.org/wiki/Netpbm
#[cfg(not(target_arch = "wasm32"))]
pub fn save_ppm(path: &std::path::Path, pixels: &[u8], sx: u32, sy: u32) -> std::io::Result<()> {
    use std::io::Write;
    let expected = 3 * sx as usize * sy as usize;
    assert!(
        pixels.len() >= expected,
        "save_ppm: pixels buffer too small ({} < {})",
        pixels.len(),
        expected
    );
    let mut file = std::fs::File::create(path)?;
    write!(file, "P6\n{sx} {sy}\n255\n")?;
    file.write_all(&pixels[..expected])?;
    Ok(())
}

/// Reverse-lookup `biome name → biome id`. Mirrors cubiomes' `_str2id`:
/// scan every biome ID, compare both `MC_NEWEST` and `MC_1_17` names
/// against `s` via substring match, and return the longest match.
///
/// Returns `None` when no biome name appears in the input.
#[must_use]
pub fn str_to_biome_id(s: &str) -> Option<i32> {
    use crate::biome::Biome;
    use crate::mc_version::MCVersion;
    if s.is_empty() {
        return None;
    }
    let mut best_len = 0_usize;
    let mut best_id: Option<i32> = None;
    for id in 0..256_i32 {
        let p = Biome::name(MCVersion::NEWEST, id);
        if let Some(name) = p {
            if name.len() > best_len && s.contains(name) {
                best_len = name.len();
                best_id = Some(id);
            }
        }
        let t = Biome::name(MCVersion::V1_17, id);
        if let Some(name) = t {
            if t != p && name.len() > best_len && s.contains(name) {
                best_len = name.len();
                best_id = Some(id);
            }
        }
    }
    best_id
}

/// Bit-exact port of cubiomes' `parseBiomeColors`. Parses a config
/// blob of `biome_name #RRGGBB` / `biome_name R G B` / `id R G B`
/// lines (semicolon-separated also accepted) and updates the palette.
///
/// Returns the number of accepted color entries.
#[allow(clippy::too_many_lines, clippy::similar_names)]
pub fn parse_biome_colors(biome_colors: &mut [[u8; 3]; 256], buf: &str) -> i32 {
    let bytes = buf.as_bytes();
    let mut p: usize = 0;
    let mut n: i32 = 0;
    while p < bytes.len() {
        let mut bstr = String::with_capacity(64);
        let mut col: [i64; 4] = [0; 4];
        let mut ic: usize = 0;
        while p < bytes.len() && bytes[p] != b'\n' && bytes[p] != b';' {
            let c = bytes[p];
            // Accumulate biome name characters: lowercase, '_'.
            if bstr.len() + 1 < 64 {
                if c.is_ascii_lowercase() || c == b'_' {
                    bstr.push(c as char);
                } else if c.is_ascii_uppercase() {
                    bstr.push(((c - b'A') + b'a') as char);
                }
            }
            // Try color literal: "#hex", "0xhex", decimal.
            if ic < 4 && (c == b'#' || (c == b'0' && p + 1 < bytes.len() && bytes[p + 1] == b'x')) {
                // Skip '#' or '0x' prefix (single char beyond the leading '0').
                let start = p + 1 + usize::from(c == b'0');
                let (value, consumed) = parse_hex(&bytes[start..]);
                col[ic] = value;
                ic += 1;
                p = start + consumed;
                continue;
            } else if ic < 4 && c.is_ascii_digit() {
                let (value, consumed) = parse_dec(&bytes[p..]);
                col[ic] = value;
                ic += 1;
                p += consumed;
                continue;
            }
            p += 1;
        }
        // Skip to end of line.
        while p < bytes.len() && bytes[p] != b'\n' {
            p += 1;
        }
        while p < bytes.len() && bytes[p] == b'\n' {
            p += 1;
        }

        let id = str_to_biome_id(&bstr);
        if let Some(id) = id {
            if (0..256).contains(&id) {
                let idx = id as usize;
                if ic == 3 {
                    biome_colors[idx][0] = (col[0] & 0xff) as u8;
                    biome_colors[idx][1] = (col[1] & 0xff) as u8;
                    biome_colors[idx][2] = (col[2] & 0xff) as u8;
                    n += 1;
                    continue;
                } else if ic == 1 {
                    biome_colors[idx][0] = ((col[0] >> 16) & 0xff) as u8;
                    biome_colors[idx][1] = ((col[0] >> 8) & 0xff) as u8;
                    biome_colors[idx][2] = (col[0] & 0xff) as u8;
                    n += 1;
                    continue;
                }
            }
        }
        // Fallthrough: "id R G B" or "id #rgb" (no name).
        if ic == 4 {
            let idx = (col[0] & 0xff) as usize;
            biome_colors[idx][0] = (col[1] & 0xff) as u8;
            biome_colors[idx][1] = (col[2] & 0xff) as u8;
            biome_colors[idx][2] = (col[3] & 0xff) as u8;
            n += 1;
        } else if ic == 2 {
            let idx = (col[0] & 0xff) as usize;
            biome_colors[idx][0] = ((col[1] >> 16) & 0xff) as u8;
            biome_colors[idx][1] = ((col[1] >> 8) & 0xff) as u8;
            biome_colors[idx][2] = (col[1] & 0xff) as u8;
            n += 1;
        }
    }
    n
}

fn parse_dec(buf: &[u8]) -> (i64, usize) {
    let mut value: i64 = 0;
    let mut i = 0;
    while i < buf.len() && buf[i].is_ascii_digit() {
        value = value * 10 + i64::from(buf[i] - b'0');
        i += 1;
    }
    (value, i)
}

fn parse_hex(buf: &[u8]) -> (i64, usize) {
    let mut value: i64 = 0;
    let mut i = 0;
    while i < buf.len() {
        let c = buf[i];
        let d = match c {
            b'0'..=b'9' => i64::from(c - b'0'),
            b'a'..=b'f' => i64::from(c - b'a' + 10),
            b'A'..=b'F' => i64::from(c - b'A' + 10),
            _ => break,
        };
        value = value * 16 + d;
        i += 1;
    }
    (value, i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ocean_color_matches_amidst() {
        let c = init_biome_colors();
        assert_eq!(c[0], [0x00, 0x00, 0x70]);
    }

    #[test]
    fn high_ids_default_to_black() {
        let c = init_biome_colors();
        assert_eq!(c[200], [0, 0, 0]);
        assert_eq!(c[255], [0, 0, 0]);
    }

    #[test]
    fn pale_garden_present() {
        let c = init_biome_colors();
        assert_eq!(c[186], [0x69, 0x6d, 0x95]);
    }

    #[test]
    fn biomes_to_image_simple() {
        let palette = init_biome_colors();
        // 2x2 grid of ocean / plains / desert / forest at pixscale=1,
        // flip=true so row 0 is at top.
        let biomes = vec![0, 1, 2, 4]; // row 0 = (ocean, plains); row 1 = (desert, forest)
        let mut pixels = vec![0u8; 3 * 4];
        let invalid = biomes_to_image(&mut pixels, &palette, &biomes, 2, 2, 1, true);
        assert!(!invalid);
        // row 0 col 0 = ocean = 0x00, 0x00, 0x70
        assert_eq!(&pixels[0..3], &[0x00, 0x00, 0x70]);
        // row 0 col 1 = plains
        assert_eq!(&pixels[3..6], &[0x8d, 0xb3, 0x60]);
        // row 1 col 0 = desert
        assert_eq!(&pixels[6..9], &[0xfa, 0x94, 0x18]);
        // row 1 col 1 = forest
        assert_eq!(&pixels[9..12], &[0x05, 0x66, 0x21]);
    }

    #[test]
    fn biomes_to_image_invalid_id_darkens() {
        let palette = init_biome_colors();
        let biomes = vec![-1_i32]; // -1 & 0x7f = 0x7f = 127 (the_void = 0)
        let mut pixels = vec![0u8; 3];
        let invalid = biomes_to_image(&mut pixels, &palette, &biomes, 1, 1, 1, true);
        assert!(invalid);
        // the_void palette is [0, 0, 0]; saturating_sub(40) keeps it 0.
        assert_eq!(pixels, vec![0, 0, 0]);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn save_ppm_writes_p6_header_and_pixels() {
        // 2x1 image: red, green.
        let pixels: [u8; 6] = [0xff, 0x00, 0x00, 0x00, 0xff, 0x00];
        let dir = std::env::temp_dir();
        let path = dir.join("cubioxides_save_ppm_test.ppm");
        save_ppm(&path, &pixels, 2, 1).expect("write");
        let bytes = std::fs::read(&path).expect("read");
        // Header: "P6\n2 1\n255\n" = 11 bytes, then 6 pixel bytes.
        assert_eq!(&bytes[..11], b"P6\n2 1\n255\n");
        assert_eq!(&bytes[11..], &pixels);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn biome_type_palette_size() {
        let c = init_biome_type_colors();
        // First 5 entries set, rest zero.
        assert_eq!(c[0], [0x00, 0x00, 0xa0]);
        assert_eq!(c[4], [0xff, 0xff, 0xff]);
        assert_eq!(c[5], [0, 0, 0]);
    }
}
