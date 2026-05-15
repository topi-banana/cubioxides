//! Build script for `ffi-tests`.
//!
//! Compiles cubiomes through `cc` and generates Rust bindings for the
//! headers via `bindgen`. The bindings live in `$OUT_DIR/bindings.rs`
//! and are pulled in by `src/lib.rs`.

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

    compile_cubiomes(&cubiomes_dir);
    generate_bindings(&cubiomes_dir);

    if cfg!(target_family = "unix") {
        println!("cargo:rustc-link-lib=pthread");
        println!("cargo:rustc-link-lib=m");
    }
}

fn compile_cubiomes(cubiomes_dir: &Path) {
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

    for entry in std::fs::read_dir(cubiomes_dir).expect("read cubiomes dir") {
        let entry = entry.expect("read dir entry");
        if entry.path().extension().is_some_and(|e| e == "h") {
            emit_rerun(&entry.path());
        }
    }

    build
        .include(cubiomes_dir)
        .flag_if_supported("-fwrapv")
        .flag_if_supported("-O2")
        .warnings(false)
        .extra_warnings(false)
        .compile("cubiomes");
}

fn generate_bindings(cubiomes_dir: &Path) {
    let wrapper_h = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("wrapper.h");
    emit_rerun(&wrapper_h);

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is set by cargo"));

    let bindings = bindgen::Builder::default()
        .header(wrapper_h.to_str().expect("wrapper path is UTF-8"))
        .clang_arg(format!("-I{}", cubiomes_dir.display()))
        // Limit the surface to what differential tests actually call. Expand
        // these allowlists as more cubiomes-side functions are needed.
        .allowlist_function("mc2str")
        .allowlist_function("str2mc")
        .allowlist_function("biome2str")
        .allowlist_function("setSeed")
        .allowlist_function("next.*")
        .allowlist_function("setupGenerator")
        .allowlist_function("applySeed")
        .allowlist_function("getBiomeAt")
        .allowlist_function("genBiomes")
        .allowlist_function("samplePerlin")
        .allowlist_function("xPerlinInit")
        .allowlist_function("perlinInit")
        .allowlist_type("Generator")
        .allowlist_type("MCVersion")
        .allowlist_type("Dimension")
        .allowlist_type("Range")
        .allowlist_type("PerlinNoise")
        .layout_tests(false)
        .generate()
        .expect("bindgen failed to generate cubiomes bindings");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("failed to write bindings.rs");
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
