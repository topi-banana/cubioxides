//! Nether Fortress piece-tree generator. Bit-exact port of
//! cubiomes' `getFortressPieces`, `addFortressPiece`,
//! `extendFortress`, and `extendFortressPiece`.
//!
//! Cubiomes models the pending-queue as a singly-linked-list via
//! `Piece::next`. The Rust port preserves that structure with an
//! `Option<usize>` index field so the queue ordering — and thus the
//! per-step `nextInt(rng, len)` draws — match cubiomes step-for-step.

#![allow(
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    clippy::many_single_char_names,
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation
)]

use crate::finder::end_city::Pos3;
use crate::finder::population_seed::chunk_generate_rng;
use crate::finder::set_attempt_seed;
use crate::mc_version::MCVersion;
use crate::rng::JavaRng;

/// Nether-Fortress piece-type discriminants. Indices match cubiomes'
/// anonymous enum (`FORTRESS_START`, …, `FORTRESS_END`).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum FortressPieceType {
    FortressStart = 0,
    BridgeStraight = 1,
    BridgeCrossing = 2,
    BridgeFortifiedCrossing = 3,
    BridgeStairs = 4,
    BridgeSpawner = 5,
    BridgeCorridorEntrance = 6,
    CorridorStraight = 7,
    CorridorCrossing = 8,
    CorridorTurnRight = 9,
    CorridorTurnLeft = 10,
    CorridorStairs = 11,
    CorridorTCrossing = 12,
    CorridorNetherWart = 13,
    FortressEnd = 14,
}

impl FortressPieceType {
    /// Number of variants (15).
    pub const COUNT: usize = 15;

    const fn from_idx(i: usize) -> Self {
        match i {
            0 => Self::FortressStart,
            1 => Self::BridgeStraight,
            2 => Self::BridgeCrossing,
            3 => Self::BridgeFortifiedCrossing,
            4 => Self::BridgeStairs,
            5 => Self::BridgeSpawner,
            6 => Self::BridgeCorridorEntrance,
            7 => Self::CorridorStraight,
            8 => Self::CorridorCrossing,
            9 => Self::CorridorTurnRight,
            10 => Self::CorridorTurnLeft,
            11 => Self::CorridorStairs,
            12 => Self::CorridorTCrossing,
            13 => Self::CorridorNetherWart,
            _ => Self::FortressEnd,
        }
    }
}

/// Cubiomes' per-type info (`offset`, `size`, `skip`, `repeatable`,
/// `weight`, `max`, `name`). The skip column drives a `skipNextN`
/// after acceptance for the two corridor-turn variants.
struct FortressInfo {
    offset: Pos3,
    size: Pos3,
    skip: u64,
    repeatable: bool,
    weight: i32,
    max: i32,
    name: &'static str,
}

const FORTRESS_INFO: [FortressInfo; FortressPieceType::COUNT] = [
    FortressInfo {
        offset: Pos3 { x: 0, y: 0, z: 0 },
        size: Pos3 { x: 18, y: 9, z: 18 },
        skip: 0,
        repeatable: false,
        weight: 0,
        max: 0,
        name: "NeStart",
    },
    FortressInfo {
        offset: Pos3 { x: -1, y: -3, z: 0 },
        size: Pos3 { x: 4, y: 9, z: 18 },
        skip: 0,
        repeatable: true,
        weight: 30,
        max: 0,
        name: "NeBS",
    },
    FortressInfo {
        offset: Pos3 { x: -8, y: -3, z: 0 },
        size: Pos3 { x: 18, y: 9, z: 18 },
        skip: 0,
        repeatable: false,
        weight: 10,
        max: 4,
        name: "NeBCr",
    },
    FortressInfo {
        offset: Pos3 { x: -2, y: 0, z: 0 },
        size: Pos3 { x: 6, y: 8, z: 6 },
        skip: 0,
        repeatable: false,
        weight: 10,
        max: 4,
        name: "NeRC",
    },
    FortressInfo {
        offset: Pos3 { x: -2, y: 0, z: 0 },
        size: Pos3 { x: 6, y: 10, z: 6 },
        skip: 0,
        repeatable: false,
        weight: 10,
        max: 3,
        name: "NeSR",
    },
    FortressInfo {
        offset: Pos3 { x: -2, y: 0, z: 0 },
        size: Pos3 { x: 6, y: 7, z: 8 },
        skip: 0,
        repeatable: false,
        weight: 5,
        max: 2,
        name: "NeMT",
    },
    FortressInfo {
        offset: Pos3 { x: -5, y: -3, z: 0 },
        size: Pos3 {
            x: 12,
            y: 13,
            z: 12,
        },
        skip: 0,
        repeatable: false,
        weight: 5,
        max: 1,
        name: "NeCE",
    },
    FortressInfo {
        offset: Pos3 { x: -1, y: 0, z: 0 },
        size: Pos3 { x: 4, y: 6, z: 4 },
        skip: 0,
        repeatable: true,
        weight: 25,
        max: 0,
        name: "NeSC",
    },
    FortressInfo {
        offset: Pos3 { x: -1, y: 0, z: 0 },
        size: Pos3 { x: 4, y: 6, z: 4 },
        skip: 0,
        repeatable: false,
        weight: 15,
        max: 5,
        name: "NeSCSC",
    },
    FortressInfo {
        offset: Pos3 { x: -1, y: 0, z: 0 },
        size: Pos3 { x: 4, y: 6, z: 4 },
        skip: 1,
        repeatable: false,
        weight: 5,
        max: 10,
        name: "NeSCRT",
    },
    FortressInfo {
        offset: Pos3 { x: -1, y: 0, z: 0 },
        size: Pos3 { x: 4, y: 6, z: 4 },
        skip: 1,
        repeatable: false,
        weight: 5,
        max: 10,
        name: "NeSCLT",
    },
    FortressInfo {
        offset: Pos3 { x: -1, y: -7, z: 0 },
        size: Pos3 { x: 4, y: 13, z: 9 },
        skip: 0,
        repeatable: true,
        weight: 10,
        max: 3,
        name: "NeCCS",
    },
    FortressInfo {
        offset: Pos3 { x: -3, y: 0, z: 0 },
        size: Pos3 { x: 8, y: 6, z: 8 },
        skip: 0,
        repeatable: false,
        weight: 7,
        max: 2,
        name: "NeCTB",
    },
    FortressInfo {
        offset: Pos3 { x: -5, y: -3, z: 0 },
        size: Pos3 {
            x: 12,
            y: 13,
            z: 12,
        },
        skip: 0,
        repeatable: false,
        weight: 5,
        max: 2,
        name: "NeCSR",
    },
    FortressInfo {
        offset: Pos3 { x: -1, y: -3, z: 0 },
        size: Pos3 { x: 4, y: 9, z: 7 },
        skip: 1,
        repeatable: false,
        weight: 0,
        max: 0,
        name: "NeBEF",
    },
];

/// A single Nether-Fortress piece in the arena. `next` chains
/// pending pieces in the same singly-linked-list cubiomes uses to
/// drive the random-pop processing loop.
#[derive(Debug, Clone)]
pub struct FortressPiece {
    /// Piece name (e.g. `"NeStart"`, `"NeBS"`).
    pub name: &'static str,
    /// Origin position.
    pub pos: Pos3,
    /// AABB minimum corner.
    pub bb0: Pos3,
    /// AABB maximum corner.
    pub bb1: Pos3,
    /// Facing: 0=north, 1=east, 2=south, 3=west.
    pub rot: u8,
    /// Recursion depth from the start piece.
    pub depth: i32,
    /// Piece type discriminant.
    pub kind: FortressPieceType,
    /// Next pending piece in cubiomes' linked-list queue, or `None`
    /// if not in queue / queue tail.
    pub next: Option<usize>,
}

struct FortressEnv<'a> {
    list: &'a mut Vec<FortressPiece>,
    rng: &'a mut JavaRng,
    ntyp: [i32; FortressPieceType::COUNT],
    typlast: usize,
}

/// Compute the bounding box for a piece given its origin, type, and facing.
fn piece_bbox(typ: FortressPieceType, pos: Pos3, facing: u8) -> (Pos3, Pos3) {
    let info = &FORTRESS_INFO[typ as usize];
    let d0 = info.offset;
    let d1 = info.size;
    let mut b0 = pos;
    let mut b1 = pos;
    b0.y += d0.y;
    b1.y += d0.y + d1.y;
    match facing {
        0 => {
            b0.x += d0.x;
            b0.z += d0.z - d1.z;
            b1.x += d0.x + d1.x;
            b1.z += d0.z;
        }
        1 => {
            b0.x += d0.z;
            b0.z += d0.x;
            b1.x += d0.z + d1.z;
            b1.z += d0.x + d1.x;
        }
        2 => {
            b0.x += d0.x;
            b0.z += d0.z;
            b1.x += d0.x + d1.x;
            b1.z += d0.z + d1.z;
        }
        _ => {
            b0.x += d0.z - d1.z;
            b0.z += d0.x;
            b1.x += d0.z;
            b1.z += d0.x + d1.x;
        }
    }
    (b0, b1)
}

/// AABB overlap (cubiomes' inclusive `>=`/`<=`).
#[inline]
const fn fortress_overlap(a_bb0: &Pos3, a_bb1: &Pos3, b_bb0: &Pos3, b_bb1: &Pos3) -> bool {
    a_bb1.x >= b_bb0.x
        && a_bb0.x <= b_bb1.x
        && a_bb1.z >= b_bb0.z
        && a_bb0.z <= b_bb1.z
        && a_bb1.y >= b_bb0.y
        && a_bb0.y <= b_bb1.y
}

/// `addFortressPiece` — try to add a piece at `(x, y, z)`. Returns
/// `Some(index)` on accept (and updates the queue when `pending`
/// is true), `None` on collision.
#[allow(clippy::too_many_arguments)]
fn add_fortress_piece(
    env: &mut FortressEnv<'_>,
    typ: FortressPieceType,
    x: i32,
    y: i32,
    z: i32,
    depth: i32,
    facing: u8,
    pending: bool,
) -> Option<usize> {
    let pos = Pos3 { x, y, z };
    let (bb0, bb1) = piece_bbox(typ, pos, facing);

    // Collision against every previously-accepted piece.
    for q in env.list.iter() {
        if fortress_overlap(&bb0, &bb1, &q.bb0, &q.bb1) {
            return None;
        }
    }

    let info = &FORTRESS_INFO[typ as usize];

    if info.skip > 0 {
        env.rng.skip_n(info.skip);
    }

    if !pending {
        // Cubiomes still "writes" the piece into `env->list[*env->n]`
        // but does NOT increment `*env->n`, so the slot is overwritten
        // by the next accepted piece. From the caller's perspective
        // (which discards the return value for non-pending pieces),
        // this is equivalent to not committing the piece at all.
        return Some(usize::MAX);
    }

    let piece = FortressPiece {
        name: info.name,
        pos,
        bb0,
        bb1,
        rot: facing,
        depth,
        kind: typ,
        next: None,
    };
    let idx = env.list.len();
    env.list.push(piece);

    env.ntyp[typ as usize] += 1;
    if typ != FortressPieceType::FortressEnd {
        env.typlast = typ as usize;
    }
    // Append to the linked-list queue: walk from list[0] to the tail.
    let mut cursor: usize = 0;
    while let Some(next) = env.list[cursor].next {
        cursor = next;
    }
    env.list[cursor].next = Some(idx);

    Some(idx)
}

#[allow(clippy::too_many_lines, clippy::needless_range_loop)]
fn extend_fortress(
    env: &mut FortressEnv<'_>,
    parent_idx: usize,
    offh: i32,
    offv: i32,
    turn: i32,
    corridor: bool,
) {
    let p_facing = env.list[parent_idx].rot;
    let p_depth = env.list[parent_idx].depth;
    let p_bb0 = env.list[parent_idx].bb0;
    let p_bb1 = env.list[parent_idx].bb1;
    let depth = p_depth + 1;
    let typ0_idx: usize = if corridor {
        FortressPieceType::CorridorStraight as usize
    } else {
        FortressPieceType::BridgeStraight as usize
    };
    let typ1_idx: usize = typ0_idx + if corridor { 7 } else { 6 };

    let y = p_bb0.y + offv;
    let (x, z, facing): (i32, i32, u8);

    if turn == 0 {
        match p_facing {
            0 => {
                x = p_bb0.x + offh;
                z = p_bb0.z - 1;
            }
            1 => {
                x = p_bb1.x + 1;
                z = p_bb0.z + offh;
            }
            2 => {
                x = p_bb0.x + offh;
                z = p_bb1.z + 1;
            }
            _ => {
                x = p_bb0.x - 1;
                z = p_bb0.z + offh;
            }
        }
        facing = p_facing;
    } else if turn == -1 {
        if p_facing & 1 == 1 {
            x = p_bb0.x + offh;
            z = p_bb0.z - 1;
            facing = 0;
        } else {
            x = p_bb0.x - 1;
            z = p_bb0.z + offh;
            facing = 3;
        }
    } else {
        // turn == +1
        if p_facing & 1 == 1 {
            x = p_bb0.x + offh;
            z = p_bb1.z + 1;
            facing = 2;
        } else {
            x = p_bb1.x + 1;
            z = p_bb0.z + offh;
            facing = 1;
        }
    }

    let start_bb0 = env.list[0].bb0;
    if (x - start_bb0.x).abs() > 112 || (z - start_bb0.z).abs() > 112 {
        // valid = -1 in cubiomes — pending=false for FORTRESS_END.
        add_fortress_piece(
            env,
            FortressPieceType::FortressEnd,
            x,
            y,
            z,
            depth,
            facing,
            false,
        );
        return;
    }

    let mut valid = false;
    let mut weight_tot: i32 = 0;
    for t in typ0_idx..typ1_idx {
        let info = &FORTRESS_INFO[t];
        if info.max > 0 && env.ntyp[t] >= info.max {
            continue;
        }
        if info.max > 0 {
            valid = true;
        }
        weight_tot += info.weight;
    }

    if !valid || weight_tot <= 0 || depth > 30 {
        add_fortress_piece(
            env,
            FortressPieceType::FortressEnd,
            x,
            y,
            z,
            depth,
            facing,
            true,
        );
        return;
    }

    for _ in 0..5 {
        let mut n = env.rng.next_int(weight_tot);
        let mut placed = false;
        for t in typ0_idx..typ1_idx {
            let info = &FORTRESS_INFO[t];
            if info.max > 0 && env.ntyp[t] >= info.max {
                continue;
            }
            n -= info.weight;
            if n >= 0 {
                continue;
            }
            if env.typlast == t && !info.repeatable {
                break;
            }
            if add_fortress_piece(
                env,
                FortressPieceType::from_idx(t),
                x,
                y,
                z,
                depth,
                facing,
                true,
            )
            .is_some()
            {
                placed = true;
                break;
            }
        }
        if placed {
            return;
        }
    }
    add_fortress_piece(
        env,
        FortressPieceType::FortressEnd,
        x,
        y,
        z,
        depth,
        facing,
        true,
    );
}

#[allow(clippy::match_same_arms)]
fn extend_fortress_piece(env: &mut FortressEnv<'_>, p_idx: usize) {
    use FortressPieceType::*;
    let typ = env.list[p_idx].kind;
    let rot = env.list[p_idx].rot;
    match typ {
        BridgeStraight => extend_fortress(env, p_idx, 1, 3, 0, false),
        BridgeCrossing | FortressStart => {
            extend_fortress(env, p_idx, 8, 3, 0, false);
            extend_fortress(env, p_idx, 8, 3, -1, false);
            extend_fortress(env, p_idx, 8, 3, 1, false);
        }
        BridgeFortifiedCrossing => {
            extend_fortress(env, p_idx, 2, 0, 0, false);
            extend_fortress(env, p_idx, 2, 0, -1, false);
            extend_fortress(env, p_idx, 2, 0, 1, false);
        }
        BridgeStairs => extend_fortress(env, p_idx, 2, 6, 1, false),
        BridgeCorridorEntrance => extend_fortress(env, p_idx, 5, 3, 0, true),
        CorridorStraight => extend_fortress(env, p_idx, 1, 0, 0, true),
        CorridorCrossing => {
            extend_fortress(env, p_idx, 1, 0, 0, true);
            extend_fortress(env, p_idx, 1, 0, -1, true);
            extend_fortress(env, p_idx, 1, 0, 1, true);
        }
        CorridorTurnRight => extend_fortress(env, p_idx, 1, 0, 1, true),
        CorridorTurnLeft => extend_fortress(env, p_idx, 1, 0, -1, true),
        CorridorStairs => extend_fortress(env, p_idx, 1, 0, 0, true),
        CorridorTCrossing => {
            let h: i32 = if rot == 0 || rot == 3 { 5 } else { 1 };
            let left = env.rng.next_int(8) != 0;
            extend_fortress(env, p_idx, h, 0, -1, left);
            let right = env.rng.next_int(8) != 0;
            extend_fortress(env, p_idx, h, 0, 1, right);
        }
        CorridorNetherWart => {
            extend_fortress(env, p_idx, 5, 3, 0, true);
            extend_fortress(env, p_idx, 5, 11, 0, true);
        }
        _ => {}
    }
}

/// Generate the Nether-Fortress piece tree for `(seed, chunk_x, chunk_z)`
/// on Minecraft version `mc`. Mirrors cubiomes' `getFortressPieces`.
/// `max_pieces` bounds the arena (recommended ~400; cubiomes uses
/// `n` as a soft limit but does not actually enforce it).
///
/// # Example
///
/// ```
/// use cubioxides::MCVersion;
/// use cubioxides::finder::get_fortress_pieces;
///
/// // After locating a candidate Nether Fortress chunk via
/// // `get_structure_pos`, expand its piece tree. The first element
/// // is always the `FortressStart` arena root. Pre-1.16.1 uses
/// // `setAttemptSeed`; 1.16.1+ uses `chunkGenerateRnd` (the doc
/// // example uses 1.18).
/// let pieces = get_fortress_pieces(MCVersion::V1_18, 0xdead_beef, 16, 16, 400);
/// assert!(!pieces.is_empty());
/// ```
#[must_use]
pub fn get_fortress_pieces(
    mc: MCVersion,
    seed: u64,
    chunk_x: i32,
    chunk_z: i32,
    max_pieces: usize,
) -> Vec<FortressPiece> {
    let mut rng = if mc.is_at_least(MCVersion::V1_16_1) {
        chunk_generate_rng(seed, chunk_x, chunk_z)
    } else {
        // Pre-1.16: setAttemptSeed + 3 disposable draws.
        let mut r = set_attempt_seed(seed, chunk_x, chunk_z);
        r.next_int(3);
        r.next_int(8);
        r.next_int(8);
        r
    };

    let mut list: Vec<FortressPiece> = Vec::with_capacity(max_pieces);
    // Initial FORTRESS_START piece. Cubiomes builds it manually
    // (no addFortressPiece call); collision check is unnecessary
    // since the arena is empty.
    let info0 = &FORTRESS_INFO[FortressPieceType::FortressStart as usize];
    let start_pos = Pos3 {
        x: chunk_x * 16 + 2,
        y: 64,
        z: chunk_z * 16 + 2,
    };
    let bb0 = start_pos;
    let mut bb1 = start_pos;
    bb1.x += info0.size.x;
    bb1.y += info0.size.y;
    bb1.z += info0.size.z;
    let rot0 = rng.next_int(4) as u8;
    list.push(FortressPiece {
        name: info0.name,
        pos: start_pos,
        bb0,
        bb1,
        rot: rot0,
        depth: 0,
        kind: FortressPieceType::FortressStart,
        next: None,
    });

    let mut ntyp = [0_i32; FortressPieceType::COUNT];
    ntyp[FortressPieceType::FortressStart as usize] = 1;

    let mut env = FortressEnv {
        list: &mut list,
        rng: &mut rng,
        ntyp,
        typlast: FortressPieceType::FortressStart as usize,
    };
    extend_fortress_piece(&mut env, 0);

    // Drain the pending queue (cubiomes pops a random element per iteration).
    while let Some(_head_next) = env.list[0].next {
        // Count queue length (walk from list[0]).
        let mut len = 0;
        let mut cursor = env.list[0].next;
        while let Some(c) = cursor {
            len += 1;
            cursor = env.list[c].next;
        }
        let i = env.rng.next_int(len);
        // Walk to the i-th element and unlink it.
        let mut prev: usize = 0;
        let mut cur = env.list[0].next.expect("queue is non-empty");
        let mut steps = i;
        while steps > 0 {
            prev = cur;
            cur = env.list[cur].next.expect("queue walk underrun");
            steps -= 1;
        }
        let cur_next = env.list[cur].next;
        env.list[prev].next = cur_next;
        env.list[cur].next = None;
        extend_fortress_piece(&mut env, cur);
    }

    list.shrink_to_fit();
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fortress_generates_non_empty_tree() {
        let pieces = get_fortress_pieces(MCVersion::V1_18, 0xdead_beef, 0, 0, 512);
        assert!(!pieces.is_empty());
        assert_eq!(pieces[0].kind, FortressPieceType::FortressStart);
    }

    #[test]
    fn from_idx_round_trips() {
        for i in 0..FortressPieceType::COUNT {
            assert_eq!(FortressPieceType::from_idx(i) as usize, i);
        }
    }
}
