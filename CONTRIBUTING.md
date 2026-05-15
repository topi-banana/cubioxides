# Contributing to cubioxides

cubioxides is a port-in-progress. The goal is bit-for-bit parity with
cubiomes; until that goal is reached, every Rust function should have a
fixture-based test that compares its output against the C reference.

## Porting workflow

1. **Pick a function** from cubiomes (e.g. `mapZoom` in `layers.c`).
2. **Add a fixture generator** in `fixtures-gen/src/bin/<module>.rs`
   that calls the cubiomes function over a deterministic range of
   inputs and writes the results to `fixtures/<module>/<name>.bin`.
3. **Port the function** to `core/src/<module>/<file>.rs`. Stick to
   `wrapping_mul` / `wrapping_add` for every site that maps to a C
   `int64_t` or `uint64_t` operation. Do not introduce a `Wrapping<T>`
   newtype.
4. **Add a parity test** in `core/tests/<module>_parity.rs` that loads
   the fixture, runs the Rust function on the same inputs, and asserts
   bit-exact equality (use `f64::to_bits` for floats).
5. **Run the full check suite locally** before committing:
   ```sh
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets --all-features -- -D warnings
   cargo test --workspace
   cargo test --workspace --release
   cargo build -p cubioxides-core --target wasm32-unknown-unknown
   cargo machete
   typos
   taplo fmt --check
   ```

## Commit guidance

- One commit per ported function or per fixture set. Keep diffs small.
- Use Conventional Commits prefixes: `feat:`, `fix:`, `test:`,
  `chore:`, `ci:`, `docs:`.
- Each commit should leave the tree green (CI checks above all pass).

## wasm32 compatibility

`cubioxides-core` must build on `wasm32-unknown-unknown`. Do not pull
in `std::fs`, `std::thread`, `std::process`, `std::env`, or
`std::time::SystemTime`. Host-only features must be gated on
`not(target_arch = "wasm32")`.

## Updating fixtures

Reference fixtures live under `fixtures/`. They are committed binary
files. Regenerate them with:

```sh
cargo run -p fixtures-gen --release -- regenerate-all
```

This step requires gcc / clang and a checkout of cubiomes at
`../cubiomes`. CI does not run regeneration; a weekly job runs the
`verify` subcommand to catch drift.
