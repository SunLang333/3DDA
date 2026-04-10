# AA00REWORK WorldGen MVP

A production-oriented first pass at the `AA00REWORK` semantic macro / lazy micro world generation architecture.

## What is implemented

- Pure C# `.NET 10` simulation core in `src/WorldGen.Core`
- Deterministic, phase-partitioned macro generation with a 9-stage pipeline
- Full CDDA-style `ZBounds` support from `-10` through `+10`
- Sparse semantic macro storage so implicit air / rock layers are not eagerly materialized
- Data-driven vertical extension rules for bridges, manholes, basements, and skyscrapers
- Independent deterministic sewer and subway networks anchored from surface semantics
- Lazy local realization driven from persisted macro semantics
- Versioned file persistence with MessagePack payloads + JSON metadata sidecars
- Thread-safe chunk manager with single-flight async generation and LRU caches
- CLI tooling for world generation, chunk inspection, and determinism validation
- xUnit v3 unit/integration tests runnable with `dotnet test`
- Optional Godot adapter stub in `src/WorldGen.Adapter.Godot`

## Project layout

- `src/WorldGen.Core` — engine-agnostic simulation types, generators, RNG, and services
- `src/WorldGen.Persistence` — `FileChunkStore`, file format envelope, metadata sidecars
- `src/WorldGen.Cli` — command-line entry point and handlers
- `src/WorldGen.Adapter.Godot` — optional adapter boundary stub, intentionally excluded from the default solution build
- `tests/WorldGen.Tests` — unit + integration tests
- `docs/ARCHITECTURE.md` — implementation mapping to the source-of-truth design docs
- `docs/PERSISTENCE_AND_MIGRATION.md` — file layout, schema versioning, and migration behavior
- `IMPLEMENTATION_REPORT.md` — decisions, trade-offs, evidence, and follow-ups

## Build

```text
dotnet build AA00REWORK/WorldGen.slnx
```

## Test

```text
dotnet test AA00REWORK/WorldGen.slnx
```

## Try it

From the repository root, or from `src/WorldGen.Cli` using the same repo-relative `AA00REWORK/out/...` paths:

```text
dotnet run --project AA00REWORK/src/WorldGen.Cli/WorldGen.Cli.csproj -- generate-world --seed 123456 --out AA00REWORK/out/example_world
```

Dump a macro chunk summary:

```text
dotnet run --project AA00REWORK/src/WorldGen.Cli/WorldGen.Cli.csproj -- dump-macro-chunk --world AA00REWORK/out/example_world --x 0 --y 0
```

Dump a local chunk summary:

```text
dotnet run --project AA00REWORK/src/WorldGen.Cli/WorldGen.Cli.csproj -- dump-local-chunk --world AA00REWORK/out/example_world --x 0 --y 0 --z 0
```

Validate determinism over a small macro grid:

```text
dotnet run --project AA00REWORK/src/WorldGen.Cli/WorldGen.Cli.csproj -- validate-determinism --world AA00REWORK/out/example_world --start-x -1 --start-y -1 --width 2 --height 2
```

## Output layout

A generated world folder contains:

- `world/world-profile.bin` + `world/world-profile.json`
- `macro/<x>_<y>.bin` + `macro/<x>_<y>.json`
- `local/<macroX>_<macroY>/<x>_<y>_<z>.bin` + `local/<macroX>_<macroY>/<x>_<y>_<z>.json` for mutated local chunks only
- generated demo summaries such as `world-summary.json`, `macro-0_0-summary.json`, and `local-0_0_0-summary.json`

## Notes

- The core does **not** reference `Godot.*`.
- Macro chunks are persisted immediately after first generation.
- Local chunks are only persisted when mutated; clean deterministic realizations are regenerated on demand, independently per z-level.
- The current content set is still intentionally modest, but the underlying world profile, persistence layer, and vertical generation model now span the full CDDA-style `-10..10` z-range.
