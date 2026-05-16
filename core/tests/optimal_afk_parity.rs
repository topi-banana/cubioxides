//! Parity test for `get_optimal_afk` vs cubiomes' `getOptimalAfk`.

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::finder::{Pos, get_optimal_afk};

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 63;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Header {
    magic: [u8; 4],
    format_version: u16,
    kind: u16,
    record_count: u64,
    reserved: [u64; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct OptimalAfkRecord {
    p0x: i32,
    p0z: i32,
    p1x: i32,
    p1z: i32,
    p2x: i32,
    p2z: i32,
    p3x: i32,
    p3z: i32,
    ax: i32,
    ay: i32,
    az: i32,
    spcnt: i32,
    afk_x: i32,
    afk_z: i32,
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("optimal_afk.bin")
}

#[test]
fn optimal_afk_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[OptimalAfkRecord] = bytemuck::cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    for (i, r) in recs.iter().enumerate() {
        let p = [
            Pos { x: r.p0x, z: r.p0z },
            Pos { x: r.p1x, z: r.p1z },
            Pos { x: r.p2x, z: r.p2z },
            Pos { x: r.p3x, z: r.p3z },
        ];
        let mut spcnt = 0_i32;
        let got = get_optimal_afk(&p, r.ax, r.ay, r.az, Some(&mut spcnt));
        assert!(
            got.x == r.afk_x && got.z == r.afk_z && spcnt == r.spcnt,
            "optimal_afk mismatch at record {i} (p={:?}, ax={}, ay={}, az={}): got ({}, {}) spcnt={}, want ({}, {}) spcnt={}",
            p,
            r.ax,
            r.ay,
            r.az,
            got.x,
            got.z,
            spcnt,
            r.afk_x,
            r.afk_z,
            r.spcnt,
        );
    }
}
