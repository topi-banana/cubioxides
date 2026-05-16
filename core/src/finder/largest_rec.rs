//! `getLargestRec` — find the largest axis-aligned rectangle of a
//! given `match` value in a 2D `ids` array. Bit-exact port of
//! cubiomes' classic histogram-based algorithm.
//!
//! Each row is processed as a per-column run-length histogram; for
//! each column we pop the rolling stack and track the maximum-area
//! rectangle ending at that column. Total: `O(sx * sz)`.

#![allow(clippy::missing_panics_doc, clippy::many_single_char_names)]

/// Result of [`get_largest_rec`]: the area of the largest
/// `match`-only rectangle plus its corner coordinates (inclusive
/// at both ends in cubiomes' convention).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LargestRec {
    /// `(p0.x, p0.z)` — top-left corner (smallest x, smallest z).
    pub p0: (i32, i32),
    /// `(p1.x, p1.z)` — bottom-right corner (largest x, largest z).
    pub p1: (i32, i32),
    /// Area (`(p1.x - p0.x + 1) * (p1.z - p0.z + 1)`) — 0 if no match.
    pub area: i32,
}

#[derive(Clone, Copy, Default)]
struct Entry {
    n: i32,
    j: i32,
    w: i32,
}

/// Find the largest axis-aligned rectangle of `match` values in
/// the `ids` grid (interpreted as `(sx, sz)` row-major, row j-th
/// row starts at index `j * sx`). Returns the rectangle and area.
///
/// Bit-exact port of cubiomes' `getLargestRec`.
///
/// **Note on cubiomes parity**: cubiomes' inner loop only commits
/// rectangles when the run-length *shrinks* at some `j`. Rectangles
/// whose right edge is the last row (`j == sz - 1`) and whose run
/// extends to the row boundary never trigger a commit and are
/// missed. This Rust port mirrors that behavior verbatim so the
/// (area, p0, p1) tuple matches cubiomes. Callers that need a
/// "true" largest-rectangle should pad the input with a non-match
/// border on the high-j and high-i sides.
#[must_use]
pub fn get_largest_rec(target: i32, ids: &[i32], sx: i32, sz: i32) -> LargestRec {
    let sx_u = sx as usize;
    let sz_u = sz as usize;
    assert!(
        ids.len() >= sx_u * sz_u,
        "get_largest_rec: ids slice too small"
    );
    let stack_size = sx_u.max(sz_u);
    let mut meta: Vec<Entry> = vec![Entry::default(); stack_size];
    let mut ret: i32 = 0;
    let mut m: usize = 0;
    let mut best = LargestRec::default();

    // Process columns right to left.
    for i in (0..sx).rev() {
        // Update per-row run lengths.
        for j in 0..sz {
            if ids[(j as usize) * sx_u + (i as usize)] == target {
                meta[j as usize].n += 1;
            } else {
                meta[j as usize].n = 0;
            }
        }
        // Scan rows accumulating widths, popping when shrinking.
        let mut w: i32 = 0;
        let mut j: i32 = 0;
        while j < sz {
            let n = meta[j as usize].n;
            if n > w {
                meta[m].j = j;
                meta[m].w = w;
                m += 1;
                w = n;
            }
            if n != w {
                loop {
                    m -= 1;
                    let e = meta[m];
                    let area = w * (j - e.j);
                    if area > ret {
                        best.p0 = (i, e.j);
                        best.p1 = (i + w - 1, j - 1);
                        best.area = area;
                        ret = area;
                    }
                    w = e.w;
                    if n >= w {
                        break;
                    }
                }
                w = n;
                if w != 0 {
                    m += 1;
                }
            }
            j += 1;
        }
        // Cubiomes does NOT drain remaining stack entries at the
        // end of the inner loop — see the doc comment about the
        // missed-right-edge bug. We deliberately match that.
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_grid_returns_zero() {
        let ids = vec![0; 9];
        let r = get_largest_rec(1, &ids, 3, 3);
        assert_eq!(r.area, 0);
    }

    #[test]
    fn full_grid_misses_due_to_cubiomes_bug() {
        // Per the doc comment: cubiomes never commits rectangles at
        // the rightmost column. A fully-matching grid hits no
        // shrink point and returns area=0.
        let ids = vec![5; 12]; // 4x3 all 5s
        let r = get_largest_rec(5, &ids, 4, 3);
        assert_eq!(r.area, 0, "cubiomes parity quirk");
    }

    #[test]
    fn finds_inner_block() {
        // 5x4 grid, the largest 1-block is a 3x2 rectangle inside.
        // ids layout (row-major, sx=5):
        //   row 0: 0 0 0 0 0
        //   row 1: 0 1 1 1 0
        //   row 2: 0 1 1 1 0
        //   row 3: 0 0 0 0 0
        #[rustfmt::skip]
        let ids = vec![
            0, 0, 0, 0, 0,
            0, 1, 1, 1, 0,
            0, 1, 1, 1, 0,
            0, 0, 0, 0, 0,
        ];
        let r = get_largest_rec(1, &ids, 5, 4);
        assert_eq!(r.area, 6);
        assert_eq!(r.p0, (1, 1));
        assert_eq!(r.p1, (3, 2));
    }
}
