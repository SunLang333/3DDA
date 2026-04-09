# Cataclysm: Dark Days Ahead — Headless Real-time Simulation Backend (Fork)

This branch refactors the original Cataclysm: Dark Days Ahead project into a headless, real-time simulation backend targeted at 3D engines (notably Godot). The goal is to separate rendering/UI/tiles logic from the simulation core and provide a deterministic, modular world-generation and simulation service suitable for embedding inside an engine or driving it over the network.

This repository keeps the original code for reference but shifts development toward a headless backend: an engine-agnostic core, semantic macro world generation, chunked persistence, deterministic simulation ticks, and clear integration boundaries. The `AA00REWORK` subproject contains a production-oriented world-generation MVP (.NET / C#) that can serve as a backend prototype or the basis for a Godot adapter.

## Core goals

- Remove/isolate all GUI, tiles, SDL, ncurses, and other rendering dependencies; the core must not depend on rendering libraries.
- Provide a deterministic, fixed-timestep tick model suitable for replay, synchronization, and server-client architectures.
- Support streaming, chunked persistence (macro/local chunks) with on-demand loading to reduce memory usage and scale to large worlds.
- Offer two integration paths for Godot: an embedded native module (GDExtension) or a remote headless service (WebSocket/TCP/UDP).
- Keep the codebase testable and verifiable: reproducible RNG seeds, versioned save formats, and determinism validation tooling.

## What's in this branch (high-level)

- `AA00REWORK/` — a production-quality semantic macro + lazy local world-generation architecture (.NET 10 / C#): a deterministic pipeline, chunk persistence, LRU caches, CLI tools, and an optional Godot adapter stub. This is the recommended starting point for backend use.
- Original `src/`, `data/` and other game components are retained for reference; new backend work should focus on headless components.

See `AA00REWORK/README.md` for implementation details; that module already implements critical backend primitives: chunk management, versioned persistence, and determinism tests.

## Quick start (AA00REWORK)

Prerequisite: install the .NET SDK (recommended: .NET 10 or the version pinned in the project).

Build:

```bash
dotnet build AA00REWORK/WorldGen.slnx
```

Generate an example world:

```bash
dotnet run --project AA00REWORK/src/WorldGen.Cli/WorldGen.Cli.csproj -- generate-world --seed 123456 --out AA00REWORK/out/example_world
```

Dump macro/local summaries and validate determinism:

```bash
dotnet run --project AA00REWORK/src/WorldGen.Cli/WorldGen.Cli.csproj -- dump-macro-chunk --world AA00REWORK/out/example_world --x 0 --y 0

dotnet run --project AA00REWORK/src/WorldGen.Cli/WorldGen.Cli.csproj -- dump-local-chunk --world AA00REWORK/out/example_world --x 0 --y 0 --z 0

dotnet run --project AA00REWORK/src/WorldGen.Cli/WorldGen.Cli.csproj -- validate-determinism --world AA00REWORK/out/example_world --start-x -1 --start-y -1 --width 2 --height 2
```

Run tests:

```bash
dotnet test AA00REWORK/WorldGen.slnx
```

AA00REWORK can be used as a headless backend prototype; you can later wrap it as a native library for Godot or run it as a network service.

## Architecture highlights

- Engine-agnostic simulation core: entities, behaviors, events, and rules separated from rendering and input.
- Macro → local generation: phase-partitioned macro generation with lazy local realization.
- Chunked persistence: macro and local chunks stored separately; binary payload with JSON metadata sidecars for easy migration and debugging.
- Chunk manager: thread-safe single-flight generation, LRU caching, and async generation support.
- Adapter boundary: expose the core as a native library (C ABI / GDExtension) or a network API (HTTP/WebSocket/UDP).

AA00REWORK implements most of these primitives and is the preferred starting point for integrating with Godot or other 3D engines.

## Recommended Godot integration patterns

Two main approaches, chosen based on latency, complexity, and scalability needs:

1. Embedded native module (GDExtension / native)

   - Build the backend core as a shared library (or add a C ABI wrapper) and call it from Godot via GDExtension.
   - Pros: minimal latency, tight data structures, straightforward synchronization.
   - Considerations: memory ownership, thread boundaries, Godot main-thread interaction, and efficient serialization (MessagePack/FlatBuffers recommended).
2. Remote headless service (recommended for rapid iteration and multiplayer)

   - Run the backend as a separate process exposing WebSocket/TCP/HTTP or a custom UDP protocol; Godot clients connect to receive snapshots/deltas and send commands.
   - Pros: independent scaling, easier hot-reload, lightweight clients.
   - Considerations: design a tick-based snapshot/delta protocol, client-side interpolation and prediction, and include versioning + seed pairing for deterministic replays.

Suggested high-level protocol model (remote):

- subscribe_world -> server returns world_profile + initial visible macro/local chunk snapshots + current_tick
- tick_update -> contains tick_id, changed_entities (compressed diff), chunk_events
- command -> client -> { client_id, tick_id, actor_id, action }
- request_chunk -> fetch compressed binary or msgpack for a chunk

All messages should include a tick/timestamp and a protocol version to support replay and debugging.

## Determinism recommendations

- Use a fixed-timestep simulation loop; bind all state updates to a tick id.
- Use reproducible RNG seeds recorded in the world-profile sidecar; any change to generation or simulation logic must bump the schema/version.
- Provide determinism validation tooling (AA00REWORK includes validation tools); add these to CI to detect regressions.

## Persistence & migrations

- Persist macro chunks immediately after generation; local chunks are persisted when mutated. Deterministic, clean local realizations are regenerated on demand.
- Use a versioned file envelope (binary payload + JSON metadata sidecar with version, seed, and generation parameters) to ease forward/backward migration.
- Default AA00REWORK layout: `world/world-profile.*`, `macro/<x>_<y>.*`, `local/<x>_<y>_<z>.*`, plus human-readable summary files for debugging.

## Build examples

> Note: the root repository contains the original C++ game. The examples below focus on AA00REWORK (.NET) and a generic CMake example for C++ headless builds.

AA00REWORK (.NET):

```bash
dotnet build AA00REWORK/WorldGen.slnx
dotnet test AA00REWORK/WorldGen.slnx
```

C++ headless (example):

```bash
cmake -S . -B build -DCMAKE_BUILD_TYPE=Release -DENABLE_TILES=OFF -DENABLE_SDL=OFF -DENABLE_NCURSES=OFF
cmake --build build -j$(nproc)
```

Adjust CMake options to match the actual CMakeLists.txt in the repo.

## Development & contribution guidelines (for this branch)

- New features must prioritize headless/engine-independent design. UI/tiles code should be optional adapter code and not pollute core logic.
- Deterministic behaviors require regression tests; new generation semantics must include repeatable verification cases.
- When changing persistence formats or RNG behavior, add migration docs in `docs/PERSISTENCE_AND_MIGRATION.md`.

## Roadmap

1. Provide a stable RPC protocol spec (JSON/MsgPack/FlatBuffers examples).
2. Implement a GDExtension wrapper example.
3. Add a headless C++ build target and progressively remove tiles/SDL dependencies.
4. Benchmark performance and network sync (latency/throughput/snapshot sizes).
