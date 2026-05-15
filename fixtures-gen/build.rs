//! Build script that compiles cubiomes from its checked-out C sources.
//!
//! Default location is `../cubiomes` relative to the workspace root (i.e.
//! a sibling of the cubioxides workspace). Override by setting the
//! `CUBIOMES_DIR` environment variable. The script links the resulting
//! static archive into the `fixtures-gen` binary.

use std::path::{Path, PathBuf};

fn main() {
    let cubiomes_dir =
        std::env::var("CUBIOMES_DIR").map_or_else(|_| default_cubiomes_dir(), PathBuf::from);

    assert!(
        cubiomes_dir.exists(),
        "cubiomes sources not found at {}. Set CUBIOMES_DIR or clone \
         https://github.com/Cubitect/cubiomes as a sibling of cubioxides.",
        cubiomes_dir.display()
    );

    println!("cargo:rerun-if-env-changed=CUBIOMES_DIR");

    let sources = [
        "util.c",
        "noise.c",
        "biomes.c",
        "layers.c",
        "biomenoise.c",
        "generator.c",
        "finders.c",
        "quadbase.c",
    ];

    let mut build = cc::Build::new();
    for src in &sources {
        let path = cubiomes_dir.join(src);
        assert!(
            path.exists(),
            "expected cubiomes source missing: {}",
            path.display()
        );
        build.file(&path);
        emit_rerun(&path);
    }

    // Non-inline wrappers exposing rng.h's static inline helpers as
    // ordinary FFI symbols.
    let ffi_wrapper = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/cubiomes_rng_ffi.c");
    assert!(ffi_wrapper.exists(), "missing {}", ffi_wrapper.display());
    build.file(&ffi_wrapper);
    emit_rerun(&ffi_wrapper);

    // Headers too — anything that affects ABI when bumped.
    for entry in std::fs::read_dir(&cubiomes_dir).expect("read cubiomes dir") {
        let entry = entry.expect("read dir entry");
        if entry.path().extension().is_some_and(|e| e == "h") {
            emit_rerun(&entry.path());
        }
    }

    build
        .include(&cubiomes_dir)
        .flag_if_supported("-fwrapv")
        .flag_if_supported("-O2")
        .warnings(false)
        .extra_warnings(false)
        .compile("cubiomes");

    if cfg!(target_family = "unix") {
        println!("cargo:rustc-link-lib=pthread");
        println!("cargo:rustc-link-lib=m");
    }
}

fn emit_rerun(path: &Path) {
    println!("cargo:rerun-if-changed={}", path.display());
}

fn default_cubiomes_dir() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_dir.parent().expect("crate sits inside a workspace");
    let project_parent = workspace_root
        .parent()
        .expect("workspace has a parent directory");
    project_parent.join("cubiomes")
}
