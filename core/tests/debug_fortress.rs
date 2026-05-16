//! Print pieces for V1_12 fortress seed 0xdeadbeef0000 chunk (0, 0)
//! so we can diff against cubiomes.

#![allow(clippy::missing_panics_doc, clippy::doc_markdown)]

use cubioxides::finder::fortress::get_fortress_pieces;
use cubioxides::mc_version::MCVersion;

#[test]
#[ignore = "diagnostic helper for fortress divergence"]
fn dump_pieces() {
    let pieces = get_fortress_pieces(MCVersion::V1_12, 0xdead_beef_0000, 0, 0, 512);
    println!("count = {}", pieces.len());
    for (i, p) in pieces.iter().enumerate() {
        println!(
            "  [{}] type={} rot={} pos=({},{},{}) bb0=({},{},{}) bb1=({},{},{})",
            i,
            p.kind as i32,
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
        );
    }
}
