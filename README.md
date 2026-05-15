# cubioxides

A Rust port of [cubiomes](https://github.com/Cubitect/cubiomes), the
fast Minecraft world generation library by Cubitect.

This crate aims for **bit-for-bit numerical compatibility** with cubiomes
for Minecraft Java Edition versions Beta 1.7 through 1.21+. Every biome
lookup, noise sample, and structure position calculation matches the
reference C implementation when given the same seed and coordinates.

## Status

Work in progress. See the milestone plan in
[`.claude/plans/`](https://github.com/topi-banana/cubioxides) for
current scope and progress.

## Crates

| Crate           | Purpose                                                        |
| --------------- | -------------------------------------------------------------- |
| `cubioxides`    | The library itself. `wasm32-unknown-unknown` compatible.       |
| `fixtures-gen`  | Dev tool: links cubiomes via FFI and dumps reference fixtures. |
| `ffi-tests`     | Dev tool: differential tests against cubiomes via bindgen.     |

## Building

```sh
cargo build -p cubioxides-core
cargo test -p cubioxides-core
cargo build -p cubioxides-core --target wasm32-unknown-unknown
```

The `fixtures-gen` and `ffi-tests` crates require a C compiler (gcc or
clang) and a checkout of cubiomes at `../cubiomes`. They are not part
of `default-members`, so a plain `cargo build` and the CI workflow do
not need a C toolchain.

## Features

| Feature    | Default | wasm32 | Description                                         |
| ---------- | ------- | ------ | --------------------------------------------------- |
| `parallel` | off     | no-op  | Use rayon for the 48-bit quad-base seed search.     |
| `colors`   | off     | no-op  | PPM image output of biome maps.                     |

## License

MIT, matching cubiomes upstream.
