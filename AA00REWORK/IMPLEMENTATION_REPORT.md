# Implementation report

## Summary

This MVP implements the `AA00REWORK` semantic macro / lazy micro architecture as an isolated `.NET 10` solution with:

- pure C# core world model and generators
- MessagePack + JSON sidecar persistence
- deterministic, phase-partitioned RNG
- async single-flight chunk management with LRU caches
- CLI tooling
- xUnit v3 test coverage
- optional Godot adapter stub kept outside the default solution build

## Decisions and trade-offs

### Serializer

**Chosen:** MessagePack `3.1.4` with LZ4 compression and JSON metadata sidecars.

Why:

- compact binary storage
- mature .NET ecosystem support
- straightforward file-backed persistence
- JSON sidecars preserve debuggability

Alternative considered:

- protobuf: stronger schema contract, but more boilerplate for the MVP
- custom binary format: maximum control, higher implementation cost and test burden

### RNG and seed derivation

**Chosen:** FNV-1a 64-bit seed derivation + PCG-style deterministic RNG.

Why:

- stable and explicit `Hash64(worldSeed, chunkKey, phaseName)` implementation
- cheap independent streams per stage
- simple to audit in tests and docs

Trade-off:

- not as statistically ambitious as a larger PRNG suite, but more than adequate for the MVP’s deterministic generation patterns

### Cache policy

**Chosen:** in-memory LRU caches with single-flight task deduplication in `ChunkManager`.

Why:

- prevents duplicate work when multiple callers request the same chunk concurrently
- bounds memory usage
- keeps async access simple and predictable

Trade-off:

- no background prefetch heuristics yet
- eviction persistence is intentionally conservative and synchronous in the current code path

### Vertical scope

**Chosen:** z-aware types from day one, with bounded content for `z = -1, 0, +1`.

Why:

- satisfies the architectural requirement without exploding MVP complexity
- exercises vertical derivation, bridge spans, and underground service/basement semantics early

Trade-off:

- not a full deep multi-level underground stack yet

## Implemented deliverables

- `src/WorldGen.Core`
  - domain model (`WorldProfile`, `MacroChunk`, `LocalChunk`, `SemanticLayer`, keys, summaries)
  - deterministic RNG + seed derivation
  - macro pipeline + local realizer
  - feature graph
  - chunk manager + LRU cache
- `src/WorldGen.Persistence`
  - `FileChunkStore`
  - schema/versioned binary envelope
  - JSON metadata sidecars
- `src/WorldGen.Cli`
  - `generate-world`
  - `dump-macro-chunk`
  - `dump-local-chunk`
  - `validate-determinism`
- `src/WorldGen.Adapter.Godot`
  - safe adapter stub demonstrating chunk-to-scene blueprint projection
- `tests/WorldGen.Tests`
  - determinism, stage isolation, feature graph, persistence, mutation, concurrency, CLI tests
- docs
  - README
  - architecture mapping
  - persistence/migration guide

## Test evidence from this session

### Build

```text
dotnet build WorldGen.slnx
```

Result: success

### Tests

```text
dotnet test WorldGen.slnx
```

Result: success

Observed summary from this session:

```text
Total tests: 9
Passed: 9
Failed: 0
Skipped: 0
```

### Key test coverage

- `SeedDerivationTests.Phase_seed_derivation_is_stable_and_phase_partitioned`
- `LinearFeatureGraphTests.Rasterize_segment_creates_expected_diagonal_points`
- `MacroStageIsolationTests.Density_stage_populates_metric_arrays_without_creating_features`
- `MacroStageIsolationTests.Hydrology_stage_marks_watery_cells_and_emits_feature_segments`
- `MacroDeterminismIntegrationTests.Macro_generation_is_bitwise_stable_for_the_same_seed_and_key`
- `ChunkPersistenceTests.Macro_chunk_persistence_roundtrips_canonical_state`
- `ChunkPersistenceTests.Local_chunk_mutation_roundtrips_without_modifying_macro_state`
- `ConcurrencyTests.Parallel_macro_generation_completes_without_deadlock_and_is_consistent`
- `CliCommandTests.Generate_world_and_validate_determinism_commands_succeed`

## Important assumptions

- one world output directory corresponds to one active `WorldProfile`
- one local chunk maps to one semantic parcel footprint
- the first pass focuses on determinism and extensibility over content breadth
- persisted macro chunks are authoritative for the schema/content version they were written with

## Deferred follow-ups

1. add explicit content registry loading from authored JSON instead of relying on the built-in default world profile
2. add a dedicated migration CLI for schema upgrades
3. split `WorldGen.Core` into additional assemblies (`Generation`, `Content`) if assembly boundaries become useful
4. add richer local simulation state (entities, damage, construction, field evolution)
5. add true Godot 4.x runtime integration with real mesh/material/collision generation
6. add background chunk prefetch and more advanced eviction tuning
