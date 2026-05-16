//! End City piece-tree generator. Bit-exact port of cubiomes'
//! `getEndCityPieces`, `addEndCityPiece`, `genPiecesRecusively`
//! and the four piece-specific generators (`genTower`, `genBridge`,
//! `genHouseTower`, `genFatTower`).
//!
//! The arena-based design uses indices instead of pointers so the
//! piece list is owned by the caller and never aliased mutably. The
//! collision check still walks all previously-emitted pieces — this
//! mirrors cubiomes' `q->bb1.x >= p->bb0.x && q->bb0.x <= p->bb1.x`
//! AABB overlap test exactly.
//!
//! End City piece-types (mirrors `finders.h`'s anonymous enum):
//! `BaseFloor` (0) through `TowerTop` (19) for a total of 20 types
//! plus `END_CITY_PIECES_MAX = 421` as the suggested arena capacity.

#![allow(clippy::many_single_char_names)]

use crate::finder::population_seed::chunk_generate_rng;
use crate::rng::JavaRng;

/// 3D integer vector. Mirrors cubiomes' `Pos3`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Pos3 {
    /// Block-x.
    pub x: i32,
    /// Block-y.
    pub y: i32,
    /// Block-z.
    pub z: i32,
}

/// End City piece type discriminant. Indices match cubiomes' enum
/// ordering exactly — do not reorder.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum EndCityPieceType {
    BaseFloor = 0,
    BaseRoof = 1,
    BridgeEnd = 2,
    BridgeGentleStairs = 3,
    BridgePiece = 4,
    BridgeSteepStairs = 5,
    FatTowerBase = 6,
    FatTowerMiddle = 7,
    FatTowerTop = 8,
    SecondFloor1 = 9,
    SecondFloor2 = 10,
    SecondRoof = 11,
    EndShip = 12,
    ThirdFloor1 = 13,
    ThirdFloor2 = 14,
    ThirdRoof = 15,
    TowerBase = 16,
    /// Unused by the generators but kept so cubiomes' indices line up.
    TowerFloor = 17,
    TowerPiece = 18,
    TowerTop = 19,
}

impl EndCityPieceType {
    /// Convert a 0..20 raw index into the corresponding piece type.
    /// Indices outside the range collapse to `TowerTop`.
    #[must_use]
    pub const fn from_idx(i: u8) -> Self {
        // Safety: only used internally with `idx in 0..20`.
        match i {
            0 => Self::BaseFloor,
            1 => Self::BaseRoof,
            2 => Self::BridgeEnd,
            3 => Self::BridgeGentleStairs,
            4 => Self::BridgePiece,
            5 => Self::BridgeSteepStairs,
            6 => Self::FatTowerBase,
            7 => Self::FatTowerMiddle,
            8 => Self::FatTowerTop,
            9 => Self::SecondFloor1,
            10 => Self::SecondFloor2,
            11 => Self::SecondRoof,
            12 => Self::EndShip,
            13 => Self::ThirdFloor1,
            14 => Self::ThirdFloor2,
            15 => Self::ThirdRoof,
            16 => Self::TowerBase,
            17 => Self::TowerFloor,
            18 => Self::TowerPiece,
            _ => Self::TowerTop,
        }
    }
}

/// Suggested upper bound on the piece-list arena. Mirrors cubiomes'
/// `END_CITY_PIECES_MAX`.
pub const END_CITY_PIECES_MAX: usize = 421;

/// `(sx, sy, sz, name)` for each piece type, indexed by
/// [`EndCityPieceType`].
const PIECE_INFO: [(i32, i32, i32, &str); 20] = [
    (9, 3, 9, "base_floor"),
    (11, 1, 11, "base_roof"),
    (4, 5, 1, "bridge_end"),
    (4, 6, 7, "bridge_gentle_stairs"),
    (4, 5, 3, "bridge_piece"),
    (4, 6, 3, "bridge_steep_stairs"),
    (12, 3, 12, "fat_tower_base"),
    (12, 7, 12, "fat_tower_middle"),
    (16, 5, 16, "fat_tower_top"),
    (11, 7, 11, "second_floor_1"),
    (11, 7, 11, "second_floor_2"),
    (13, 1, 13, "second_roof"),
    (12, 23, 28, "ship"),
    (13, 7, 13, "third_floor_1"),
    (13, 7, 13, "third_floor_2"),
    (15, 1, 15, "third_roof"),
    (6, 6, 6, "tower_base"),
    (6, 3, 6, "tower_floor"),
    (6, 3, 6, "tower_piece"),
    (8, 4, 8, "tower_top"),
];

/// One generated End City piece. Bit-exact port of cubiomes' `Piece`.
#[derive(Debug, Clone)]
pub struct Piece {
    /// Piece name (static string, e.g. `"base_floor"`).
    pub name: &'static str,
    /// Anchor position.
    pub pos: Pos3,
    /// AABB minimum corner.
    pub bb0: Pos3,
    /// AABB maximum corner.
    pub bb1: Pos3,
    /// Rotation: 0=0°, 1=90°, 2=180°, 3=270°.
    pub rot: u8,
    /// Collision-check group (modified during piece-tree generation).
    pub depth: i8,
    /// Piece type discriminant.
    pub kind: EndCityPieceType,
}

struct PieceEnv<'a> {
    list: &'a mut Vec<Piece>,
    rng: &'a mut JavaRng,
    ship: bool,
    y: i32,
}

/// Translate `(px, py, pz)` by `prev_rot`. Helper for the
/// rotation-aware offset in `add_end_city_piece`.
#[inline]
const fn rotated_offset(prev_rot: u8, px: i32, py: i32, pz: i32) -> (i32, i32, i32) {
    let (dx, dz) = match prev_rot {
        0 => (px, pz),
        1 => (-pz, px),
        2 => (-px, -pz),
        _ => (pz, -px),
    };
    (dx, py, dz)
}

/// Append a new piece anchored on `prev` (or at absolute `(px, py, pz)`
/// when `prev` is `None`), returning its index in `env.list`. Bit-exact
/// port of cubiomes' `addEndCityPiece`.
#[allow(clippy::too_many_arguments)]
fn add_end_city_piece(
    env: &mut PieceEnv<'_>,
    prev: Option<usize>,
    rot: u8,
    px: i32,
    py: i32,
    pz: i32,
    kind: EndCityPieceType,
) -> usize {
    let (sx, sy, sz, name) = PIECE_INFO[kind as usize];

    let mut pos = if let Some(i) = prev {
        env.list[i].pos
    } else {
        Pos3 {
            x: px,
            y: py,
            z: pz,
        }
    };
    let mut bb0 = pos;
    let mut bb1 = pos;
    bb1.y += sy;
    match rot {
        0 => {
            bb1.x += sx;
            bb1.z += sz;
        }
        1 => {
            bb0.x -= sz;
            bb1.z += sx;
        }
        2 => {
            bb0.x -= sx;
            bb0.z -= sz;
        }
        _ => {
            bb1.x += sz;
            bb0.z -= sx;
        }
    }

    if let Some(i) = prev {
        let prev_rot = env.list[i].rot;
        let (dx, dy, dz) = rotated_offset(prev_rot, px, py, pz);
        pos.x += dx;
        pos.y += dy;
        pos.z += dz;
        bb0.x += dx;
        bb0.y += dy;
        bb0.z += dz;
        bb1.x += dx;
        bb1.y += dy;
        bb1.z += dz;
    }

    env.list.push(Piece {
        name,
        pos,
        bb0,
        bb1,
        rot,
        depth: 0,
        kind,
    });
    env.list.len() - 1
}

/// AABB-overlap test matching cubiomes' inclusive `>=` / `<=` comparison.
#[inline]
const fn pieces_overlap(a: &Piece, b: &Piece) -> bool {
    a.bb1.x >= b.bb0.x
        && a.bb0.x <= b.bb1.x
        && a.bb1.z >= b.bb0.z
        && a.bb0.z <= b.bb1.z
        && a.bb1.y >= b.bb0.y
        && a.bb0.y <= b.bb1.y
}

/// Speculatively run a piece generator from `current`. Mirrors
/// cubiomes' `genPiecesRecusively`: emits pieces into a scratch
/// window, assigns them the same `depth` value (drawn from
/// `next(rng, 32)`), then commits them only when none collide with
/// the *immediate parent's* emitted pieces. Returns `true` on commit.
///
/// `parent_scope_start` is the index where the caller (parent
/// wrapper) began emitting its pieces; the collision range is
/// `[parent_scope_start..n_before]`. Cubiomes encodes this via the
/// `env_local.list = env->list + *env->n` pointer arithmetic — the
/// inner wrapper only sees pieces from its immediate parent's scope.
fn gen_pieces_recursively(
    generator: for<'b> fn(&mut PieceEnv<'b>, usize, i32, usize) -> bool,
    env: &mut PieceEnv<'_>,
    current: usize,
    depth: i32,
    parent_scope_start: usize,
) -> bool {
    if depth > 8 {
        return false;
    }
    let n_before = env.list.len();
    if !generator(env, current, depth, n_before) {
        // Drop any pieces the failed generator emitted.
        env.list.truncate(n_before);
        return false;
    }
    let gendepth = env.rng.next(32) as i8;
    let current_depth = env.list[current].depth;
    let n_after = env.list.len();
    // Each newly emitted piece gets the same depth tag (matches
    // cubiomes' assignment loop).
    for i in n_before..n_after {
        env.list[i].depth = gendepth;
    }
    // Collision check: only against the immediate parent's emitted
    // pieces (indices [parent_scope_start..n_before]) — cubiomes'
    // `env_local.list` pointer skips everything before that.
    for i in n_before..n_after {
        for j in parent_scope_start..n_before {
            // Borrow disjoint slices to avoid mutable aliasing.
            let (p, q) = if i < j {
                let (l, r) = env.list.split_at(j);
                (&l[i], &r[0])
            } else {
                let (l, r) = env.list.split_at(i);
                (&r[0], &l[j])
            };
            if pieces_overlap(p, q) {
                if current_depth != q.depth {
                    env.list.truncate(n_before);
                    return false;
                }
                break;
            }
        }
    }
    true
}

fn gen_tower(env: &mut PieceEnv<'_>, current: usize, depth: i32, my_scope: usize) -> bool {
    let rot = env.list[current].rot;
    let x = 3 + env.rng.next_int(2);
    let z = 3 + env.rng.next_int(2);
    let mut base = current;
    base = add_end_city_piece(env, Some(base), rot, x, -3, z, EndCityPieceType::TowerBase);
    base = add_end_city_piece(env, Some(base), rot, 0, 7, 0, EndCityPieceType::TowerPiece);
    let mut floor = if env.rng.next_int(3) == 0 { Some(base) } else { None };
    let floorcnt = 1 + env.rng.next_int(3);
    for i in 0..floorcnt {
        base = add_end_city_piece(env, Some(base), rot, 0, 4, 0, EndCityPieceType::TowerPiece);
        if i < floorcnt - 1 && env.rng.next(1) != 0 {
            floor = Some(base);
        }
    }
    if floor.is_some() {
        const BINFO: [[i32; 4]; 4] = [
            [0, 1, -1, 0],
            [1, 6, -1, 1],
            [3, 0, -1, 5],
            [2, 5, -1, 6],
        ];
        for entry in BINFO {
            if env.rng.next(1) == 0 {
                continue;
            }
            let brot = ((rot as i32 + entry[0]) & 3) as u8;
            let bridge = add_end_city_piece(
                env,
                Some(base),
                brot,
                entry[1],
                entry[2],
                entry[3],
                EndCityPieceType::BridgeEnd,
            );
            gen_pieces_recursively(gen_bridge, env, bridge, depth + 1, my_scope);
        }
    } else if depth != 7 {
        return gen_pieces_recursively(gen_fat_tower, env, base, depth + 1, my_scope);
    }

    add_end_city_piece(env, Some(base), rot, -1, 4, -1, EndCityPieceType::TowerTop);
    true
}

fn gen_bridge(env: &mut PieceEnv<'_>, current: usize, depth: i32, my_scope: usize) -> bool {
    let rot = env.list[current].rot;
    let floorcnt = 1 + env.rng.next_int(4);
    let mut base = current;
    base = add_end_city_piece(env, Some(base), rot, 0, 0, -4, EndCityPieceType::BridgePiece);
    env.list[base].depth = -1;
    let mut y = 0_i32;
    for _ in 0..floorcnt {
        if env.rng.next(1) != 0 {
            base = add_end_city_piece(env, Some(base), rot, 0, y, -4, EndCityPieceType::BridgePiece);
            y = 0;
            continue;
        }
        if env.rng.next(1) != 0 {
            base = add_end_city_piece(
                env,
                Some(base),
                rot,
                0,
                y,
                -4,
                EndCityPieceType::BridgeSteepStairs,
            );
        } else {
            base = add_end_city_piece(
                env,
                Some(base),
                rot,
                0,
                y,
                -8,
                EndCityPieceType::BridgeGentleStairs,
            );
        }
        y = 4;
    }
    if !env.ship && env.rng.next_int(10 - depth) == 0 {
        let x = -8 + env.rng.next_int(8);
        let z = -70 + env.rng.next_int(10);
        base = add_end_city_piece(env, Some(base), rot, x, y, z, EndCityPieceType::EndShip);
        env.ship = true;
    } else {
        env.y = y + 1;
        if !gen_pieces_recursively(gen_house_tower, env, base, depth + 1, my_scope) {
            return false;
        }
    }
    base = add_end_city_piece(
        env,
        Some(base),
        (rot + 2) & 3,
        4,
        y,
        0,
        EndCityPieceType::BridgeEnd,
    );
    env.list[base].depth = -1;
    true
}

fn gen_house_tower(env: &mut PieceEnv<'_>, current: usize, depth: i32, my_scope: usize) -> bool {
    if depth > 8 {
        return false;
    }
    let rot = env.list[current].rot;
    let mut base = current;
    base = add_end_city_piece(
        env,
        Some(base),
        rot,
        -3,
        env.y,
        -11,
        EndCityPieceType::BaseFloor,
    );
    let size = env.rng.next_int(3);
    if size == 0 {
        add_end_city_piece(env, Some(base), rot, -1, 4, -1, EndCityPieceType::BaseRoof);
        return true;
    }
    base = add_end_city_piece(
        env,
        Some(base),
        rot,
        -1,
        0,
        -1,
        EndCityPieceType::SecondFloor2,
    );
    if size == 1 {
        base = add_end_city_piece(
            env,
            Some(base),
            rot,
            -1,
            8,
            -1,
            EndCityPieceType::SecondRoof,
        );
    } else {
        base = add_end_city_piece(
            env,
            Some(base),
            rot,
            -1,
            4,
            -1,
            EndCityPieceType::ThirdFloor2,
        );
        base = add_end_city_piece(env, Some(base), rot, -1, 8, -1, EndCityPieceType::ThirdRoof);
    }
    gen_pieces_recursively(gen_tower, env, base, depth + 1, my_scope);
    true
}

fn gen_fat_tower(env: &mut PieceEnv<'_>, current: usize, depth: i32, my_scope: usize) -> bool {
    const BINFO: [[i32; 4]; 4] = [
        [0, 4, -1, 0],
        [1, 12, -1, 4],
        [3, 0, -1, 8],
        [2, 8, -1, 12],
    ];
    let rot = env.list[current].rot;
    let mut base = current;
    base = add_end_city_piece(env, Some(base), rot, -3, 4, -3, EndCityPieceType::FatTowerBase);
    base = add_end_city_piece(
        env,
        Some(base),
        rot,
        0,
        4,
        0,
        EndCityPieceType::FatTowerMiddle,
    );
    let mut j = 0;
    while j < 2 && env.rng.next_int(3) != 0 {
        base = add_end_city_piece(
            env,
            Some(base),
            rot,
            0,
            8,
            0,
            EndCityPieceType::FatTowerMiddle,
        );
        for entry in BINFO {
            if env.rng.next(1) == 0 {
                continue;
            }
            let brot = ((rot as i32 + entry[0]) & 3) as u8;
            let bridge = add_end_city_piece(
                env,
                Some(base),
                brot,
                entry[1],
                entry[2],
                entry[3],
                EndCityPieceType::BridgeEnd,
            );
            gen_pieces_recursively(gen_bridge, env, bridge, depth + 1, my_scope);
        }
        j += 1;
    }
    add_end_city_piece(env, Some(base), rot, -2, 8, -2, EndCityPieceType::FatTowerTop);
    true
}

/// Generate the End City piece tree for `(chunk_x, chunk_z)` in
/// world `seed`. Returns the list of pieces in the order cubiomes
/// emits them. Bit-exact port of `getEndCityPieces`.
#[must_use]
pub fn get_end_city_pieces(seed: u64, chunk_x: i32, chunk_z: i32) -> Vec<Piece> {
    let mut rng = chunk_generate_rng(seed, chunk_x, chunk_z);
    let rot = rng.next_int(4) as u8;
    let mut list: Vec<Piece> = Vec::with_capacity(END_CITY_PIECES_MAX);
    let mut env = PieceEnv {
        list: &mut list,
        rng: &mut rng,
        ship: false,
        y: 0,
    };
    let x = chunk_x * 16 + 8;
    let z = chunk_z * 16 + 8;
    let base = add_end_city_piece(&mut env, None, rot, x, 0, z, EndCityPieceType::BaseFloor);
    let base = add_end_city_piece(
        &mut env,
        Some(base),
        rot,
        -1,
        0,
        -1,
        EndCityPieceType::SecondFloor1,
    );
    let base = add_end_city_piece(
        &mut env,
        Some(base),
        rot,
        -1,
        4,
        -1,
        EndCityPieceType::ThirdFloor1,
    );
    let base = add_end_city_piece(&mut env, Some(base), rot, -1, 8, -1, EndCityPieceType::ThirdRoof);
    gen_pieces_recursively(gen_tower, &mut env, base, 1, 0);
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn piece_count_matches_cubiomes_arena_bound() {
        // Smoke test that a known small seed stays well under the
        // arena bound. The real parity check lives in
        // tests/end_city_pieces_parity.rs.
        let pieces = get_end_city_pieces(0xdead_beef, 0, 0);
        assert!(pieces.len() <= END_CITY_PIECES_MAX);
    }

    #[test]
    fn from_idx_round_trips() {
        for i in 0_u8..20 {
            assert_eq!(EndCityPieceType::from_idx(i) as u8, i);
        }
    }
}
