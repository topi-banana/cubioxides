//! Print the first few End City pieces for a known seed so we
//! can diff against cubiomes' raw output.

#![allow(clippy::missing_panics_doc, clippy::doc_markdown)]

use cubioxides::finder::end_city::get_end_city_pieces;

#[test]
#[ignore = "diagnostic helper for End City piece-tree divergence"]
fn dump_pieces() {
    let seed: u64 = 0xdead_beef_0000;
    let pieces = get_end_city_pieces(seed, 0, 0);
    println!("count = {}", pieces.len());
    for (i, p) in pieces.iter().enumerate() {
        println!(
            "  [{}] kind={:?} rot={} pos=({},{},{}) bb0=({},{},{}) bb1=({},{},{}) depth={}",
            i,
            p.kind,
            p.rot,
            p.pos.x,
            p.pos.y,
            p.pos.z,
            p.bb0.x,
            p.bb0.y,
            p.bb0.z,
            p.bb1.x,
            p.bb1.y,
            p.bb1.z,
            p.depth,
        );
    }
}
