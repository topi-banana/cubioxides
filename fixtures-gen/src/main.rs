//! Reference fixture generator for cubioxides.
//!
//! Links cubiomes via FFI and emits deterministic input/output records
//! that the `cubioxides-core` test suite consumes for parity checks.
//!
//! This file is the M0 stub. Subcommands for each module (rng, noise,
//! layers, biomenoise, structures) land in their respective milestones.

#![allow(unsafe_code)]

use std::ffi::{CStr, c_char, c_int};
use std::process::ExitCode;

unsafe extern "C" {
    /// `cubiomes/util.h`: returns a static C string such as `"1.18.2"` for a
    /// given `MCVersion` ordinal.
    fn mc2str(mc: c_int) -> *const c_char;
}

// Discriminant of `MC_1_18` (alias for `MC_1_18_2`) in `cubiomes/biomes.h`.
// Update alongside the upstream enum whenever cubiomes inserts new versions.
const MC_1_18: c_int = 22;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map_or("help", String::as_str);

    match cmd {
        "help" | "--help" | "-h" => {
            print_help();
            ExitCode::SUCCESS
        }
        "verify" => verify(),
        "regenerate-all" => {
            eprintln!("fixtures-gen regenerate-all: no fixtures wired up yet (M1+ work)");
            ExitCode::SUCCESS
        }
        unknown => {
            eprintln!("unknown subcommand: {unknown}");
            print_help();
            ExitCode::FAILURE
        }
    }
}

fn verify() -> ExitCode {
    // FFI smoke check: prove the cubiomes link works end-to-end.
    let mc_name = unsafe {
        let ptr = mc2str(MC_1_18);
        if ptr.is_null() {
            eprintln!("cubiomes mc2str returned a null pointer");
            return ExitCode::FAILURE;
        }
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    };
    println!("cubiomes mc2str(MC_1_18) = {mc_name:?}");
    if mc_name != "1.18" {
        eprintln!(
            "verify failed: expected \"1.18\" from cubiomes mc2str, got {mc_name:?}. \
             The MC_1_18 ordinal in fixtures-gen may be out of sync with upstream."
        );
        return ExitCode::FAILURE;
    }
    println!("FFI smoke test passed.");
    ExitCode::SUCCESS
}

fn print_help() {
    eprintln!("Usage: fixtures-gen <subcommand>");
    eprintln!();
    eprintln!("Subcommands:");
    eprintln!("  verify           FFI smoke-test against cubiomes (must print \"1.18\")");
    eprintln!("  regenerate-all   Regenerate every binary fixture under fixtures/ (M1+)");
    eprintln!("  help             Show this help");
}
