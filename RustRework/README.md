# RustRework WorldGen MVP

A Rust reimplementation of the `AA00REWORK` semantic macro / lazy micro world generation MVP.

## What is implemented

- Cargo workspace split into AA00REWORK-like member crates under `src/` and `tests/`
- Core Rust simulation APIs exposed by `src/worldgen-core`
- File persistence exposed by `src/worldgen-persistence`
- CLI entrypoint exposed by `src/worldgen-cli`
- Deterministic, phase-partitioned macro generation with the same 9-stage pipeline
- Full CDDA-style `ZBounds` support from `-10` through `+10`
- Sparse semantic macro storage with implicit air / rock resolution
- Declarative vertical extension rules for bridges, manholes, basements, and skyscrapers
- Independent deterministic sewer and subway networks anchored from surface semantics
- Lazy local realization from persisted macro semantics
- Versioned file persistence with MessagePack payloads + JSON sidecars
- Thread-safe chunk manager with single-flight generation and LRU caches
- CLI tooling for world generation, chunk inspection, item counting, and determinism validation
- Rust test coverage for determinism, persistence, concurrency, vertical derivation, and CLI behavior
- Optional Godot adapter stub in `src/worldgen-adapter-godot`
- Migrated support asset set in `data/items.csv`

## Build

Run `cargo build --workspace` from `RustRework/`.

## Test

Run `cargo test --workspace` from `RustRework/`.

## Try it

From the repository root, or from `RustRework/` using the same repo-relative `RustRework/out/...` paths:

- `cargo run -p worldgen-cli -- generate-world --seed 123456 --out RustRework/out/example_world`
- `cargo run -p worldgen-cli -- dump-macro-chunk --world RustRework/out/example_world --x 0 --y 0`
- `cargo run -p worldgen-cli -- dump-local-chunk --world RustRework/out/example_world --x 0 --y 0 --z 0`
- `cargo run -p worldgen-cli -- validate-determinism --world RustRework/out/example_world --start-x -1 --start-y -1 --width 2 --height 2`
- `cargo run -p worldgen-cli -- count-items --world RustRework/out/example_world --surface-only`

## Workspace layout

- `src/worldgen-core` — domain model, macro generation, local realization, diagnostics, cache/orchestration contracts
- `src/worldgen-persistence` — MessagePack + LZ4 persistence boundary and file-backed chunk store
- `src/worldgen-cli` — command handlers and binary entrypoint
- `src/worldgen-adapter-godot` — Godot-facing adapter stub
- `tests/worldgen-tests` — integration tests migrated from the original single-crate MVP
- `data/items.csv` — migrated support asset from `AA00REWORK/data`

## Output layout

A generated world folder contains:

- `world/world-profile.bin` + `world/world-profile.json`
- `macro/<x>_<y>.bin` + `macro/<x>_<y>.json`
- `local/<macroX>_<macroY>/<x>_<y>_<z>.bin` + `local/<macroX>_<macroY>/<x>_<y>_<z>.json` for mutated local chunks only
- generated demo summaries such as `world-summary.json`, `macro-0_0-summary.json`, and `local-0_0_0-summary.json`

## Notes

- The Rust core does not reference Godot runtime types.
- Macro chunks are persisted immediately after first generation.
- Local chunks are only persisted when mutated; clean deterministic realizations are regenerated on demand, independently per z-level.
- The content set is intentionally modest, but the world profile, persistence layer, vertical model, and CLI contract match the MVP behavior implemented in `AA00REWORK`.
