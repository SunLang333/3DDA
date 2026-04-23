# Architecture mapping

This document maps the Rust implementation in `RustRework/` to the same macro / micro architecture implemented in `AA00REWORK/`.

The public workspace surface is now split across:

- `src/worldgen-core`
- `src/worldgen-persistence`
- `src/worldgen-cli`
- `src/worldgen-adapter-godot`
- `tests/worldgen-tests`

## Core rule

> Model the world as a persistent semantic macro lattice, then lazily realize concrete local chunks from that semantic state, persisting only mutated local state.

That rule is implemented by the interaction between:

- `src/worldgen-core`
- `src/worldgen-persistence`
- `src/worldgen-cli`

Internally, the shared implementation files remain rooted under `RustRework/src/*.rs`, while the workspace member crates provide AA00REWORK-like boundaries and public entrypoints.

## Macro / micro split

### Persistent macro layer

Implemented by:

- `WorldProfile`
- `MacroChunk`
- `SemanticParcel`
- `SemanticLayer`
- `ChunkFeatureSummary`
- `ChunkConnectivitySummary`

The macro layer stores semantic truth:

- landcover
- infrastructure
- parcel identity
- layered semantic history
- coarse connectivity
- full `ZBounds(-10, +10)` vertical semantic derivation

Macro chunks store only explicitly-authored or derived semantic parcels in a sparse `BTreeMap<WorldCellCoordinate, SemanticParcel>`. Any unmaterialized coordinate resolves implicitly to the correct default surface / air / subterranean parcel for its z-level.

### Lazy micro layer

Implemented by:

- `LocalChunk`
- `LocalCell`
- `LocalChunkProvenance`
- `LocalRealizer`

The local layer realizes one semantic parcel footprint at a time, keyed by `LocalChunkKey(Macro, X, Y, Z)`. The realization path replays semantic layers in order and adds concrete structure, props, items, fields, and spawns.

## The 9-stage macro pipeline

`MacroGenerationPipeline` uses the same discrete stage order as `AA00REWORK`:

1. `InitializeLayersStage`
2. `CalculateDensityMetricsStage`
3. `PlaceHydrologyStage`
4. `PlaceNaturalFeaturesStage`
5. `SeedSettlementsStage`
6. `BuildInfrastructureStage`
7. `PlaceSpecialsStage`
8. `VerticalDerivationStage`
9. `FinalizeOvermapStage`

## Determinism strategy

Implemented by `worldgen-core` via `src/randomization.rs`.

- `PhaseSeedDeriver` uses a stable 64-bit FNV-1a hash over `worldSeed`, chunk key, and phase name.
- `DeterministicRandomSource` uses a PCG-style generator seeded from the derived 64-bit phase seed.
- Generation phases request independent RNG streams by phase name, and deeper subsystems such as sewer/subway routing fork dedicated substreams to avoid cross-phase coupling.
- Canonical equality is checked via `src/diagnostics.rs`, which serializes macro/local chunks into deterministic JSON byte sequences used by tests and reporting.

## Runtime orchestration

Implemented by `worldgen-core` via `src/chunk_manager.rs`.

Responsibilities:

- register/load the active `WorldProfile`
- lazy-load chunks from persistence
- generate missing macro/local chunks on demand
- enforce single-flight generation per key under concurrency
- maintain LRU caches for macro and local chunks
- persist dirty local chunks when requested or during shutdown

## Persistence boundary

Implemented by `worldgen-persistence` via `src/persistence.rs`.

- `WorldProfile`, `MacroChunk`, and mutated `LocalChunk` payloads are written as binary MessagePack envelopes.
- Human-readable JSON sidecars summarize each stored payload.
- Local chunk elision is policy-driven and z-specific: if a local chunk at a particular `(macro, x, y, z)` is not dirty, it is deleted or omitted so it can be regenerated later.

## Godot boundary

The Rust core contains no Godot runtime references.

`src/worldgen-adapter-godot` exposes the adapter direction by projecting `LocalChunk` into a `GodotChunkSceneBlueprint`. It is intentionally a stub, mirroring the role of `WorldGen.Adapter.Godot` in `AA00REWORK`.
