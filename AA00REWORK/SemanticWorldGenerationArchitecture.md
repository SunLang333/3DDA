
# Semantic World Generation Architecture

## Formal Design Document for a C# / .NET 10 World Simulation Core

### Oriented for Later Godot 3D Integration

**Document status:** Draft v1
**Audience:** Engine programmers, gameplay programmers, technical designers, tools programmers
**Primary goal:** Recreate a CDDA-like procedural world architecture centered on **persistent semantic macro state** and **lazy micro realization**, using **C# on .NET 10** as the simulation core, while keeping the runtime cleanly adaptable to **Godot 4.x 3D** later. .NET 10 is currently an **LTS** release with **C# 14** support, which makes it a strong baseline for a multi-year engine effort. [\[learn.microsoft.com\]](https://learn.microsoft.com/en-us/dotnet/core/whats-new/dotnet-10/overview), [\[dotnet.microsoft.com\]](https://dotnet.microsoft.com/en-us/platform/support/policy), [\[dotnet.microsoft.com\]](https://dotnet.microsoft.com/en-us/download/dotnet/10.0)

---

# 1. Executive Summary

This architecture models the game world in **two distinct but connected layers**:

1. **Macro semantic world**

   * persistent, abstract, chunked
   * stores landcover, infrastructure, parcels, city graph membership, special instances, vertical semantic layers, and predecessor/overlay history
   * generated once on demand and then persisted
2. **Micro realized world**

   * concrete, local, lazily materialized
   * stores actual geometry-usable tile/voxel/cell data, props, entities, items, simulation state, and damage
   * generated only when needed from macro state and then persisted only when nontrivial or mutated

This separation is the core architectural requirement.
The engine must **never require full-world concrete generation up front**.

The implementation should be split into:

* a **pure .NET 10 simulation core**
* a **persistence layer**
* a **generation pipeline**
* a future **Godot integration adapter** for 3D scene realization, input, rendering, physics, and streaming

Because Godot’s C# workflow requires the **.NET-enabled editor build**, and current official docs note that C# projects support desktop platforms plus experimental Android/iOS support, while **web export is not available for Godot 4 C# projects**, the core world simulation should remain **engine-agnostic** and must not depend on Godot types. [\[docs.godotengine.org\]](https://docs.godotengine.org/en/stable/tutorials/scripting/c_sharp/), [\[docs.godotengine.org\]](https://docs.godotengine.org/en/4.4/tutorials/scripting/c_sharp/index.html), [\[github.com\]](https://github.com/godotengine/godot-docs/blob/master/tutorials/scripting/c_sharp/c_sharp_basics.rst)

---

# 2. Goals

## 2.1 Primary goals

The system shall:

* preserve a **persistent semantic macro lattice**
* generate macro chunks **lazily**
* realize concrete local chunks **lazily**
* preserve **predecessor-aware refinement**
* support **fixed specials** and **mutable socket/join specials**
* support **late regional substitution**
* persist **macro semantic state** separately from **micro mutable state**
* omit persistence for **uniform or trivially reproducible local chunks**
* remain **headless and testable** outside any rendering engine
* support later **Godot 3D visualization and streaming**

## 2.2 Secondary goals

The system should:

* be deterministic under stable content and seed inputs
* support mod/content-driven procedural rules
* be versionable and migratable
* support tooling, introspection, and debug replay
* support future multithreaded generation

---

# 3. Non-Goals

The system shall **not** initially attempt to:

* reproduce any specific legacy 2D tile implementation literally
* encode all semantics into string terrain identifiers
* bind generation logic directly to Godot scenes or nodes
* require that all world data be loaded into memory at once
* solve final rendering, navmesh baking, or physics authoring in this layer

---

# 4. Platform and Technology Baseline

## 4.1 Core platform

The simulation core will target **.NET 10** and use modern C# language features available with that release. .NET 10 is an **LTS** release supported through **November 14, 2028**, and Microsoft documents C# 14 as part of the .NET 10 toolchain. [\[dotnet.microsoft.com\]](https://dotnet.microsoft.com/en-us/platform/support/policy), [\[dotnet.microsoft.com\]](https://dotnet.microsoft.com/en-us/download/dotnet/10.0)

## 4.2 Godot orientation

The project is being oriented for **later Godot 4.x 3D development**, but the simulation core must be independent of Godot. Official Godot docs state that C# support requires downloading the **.NET build of the editor**, and that Godot 4 C# currently supports desktop platforms, with Android/iOS marked experimental and **no web export support** for Godot 4 C# projects. [\[docs.godotengine.org\]](https://docs.godotengine.org/en/stable/tutorials/scripting/c_sharp/), [\[docs.godotengine.org\]](https://docs.godotengine.org/en/4.4/tutorials/scripting/c_sharp/index.html), [\[github.com\]](https://github.com/godotengine/godot-docs/blob/master/tutorials/scripting/c_sharp/c_sharp_basics.rst)

## 4.3 Architectural implication

Therefore:

* **Core simulation libraries** should target `.NET 10`
* **Godot adapter code** should be isolated in a separate integration project
* no core domain type may depend on `Godot.*`
* serialization formats must be engine-neutral
* world generation must be runnable from CLI, tests, or editor tooling without Godot

---

# 5. Core Design Principles

## 5.1 Semantic-first world modeling

The world is modeled first as **semantic state**, not rendered geometry.

A macro cell answers questions such as:

* what kind of place is this?
* what overlays exist here?
* what infrastructure crosses it?
* what special parcel owns it?
* what used to be here?
* what vertical semantic layers exist above/below?

It does **not** directly answer:

* what exact meshes are instantiated?
* which triangles or colliders exist?
* which prop scene file is used?

## 5.2 Lazy realization

Concrete local geometry/state is generated only when required by:

* player proximity
* visibility
* AI/pathfinding demand
* scripted queries
* editor inspection
* simulation activation

## 5.3 Persistence boundary clarity

Persist separately:

* **macro semantic world state**
* **micro realized mutable chunk state**
* **player/world observation state**
* **global procedural state**

## 5.4 Overlay/history-aware generation

Local realization must not treat the current semantic cell as a single flat identity.It must be able to replay or interpret **semantic layer history** such as:

* base landform
* vegetation cover
* hydrology overlay
* road/rail overlay
* parcel/special overlay
* temporary/postgen overlay

## 5.5 Engine decoupling

The core simulation must be equally runnable in:

* tests
* headless command-line tools
* editor utilities
* server-like simulation contexts
* a future Godot runtime

---

# 6. Conceptual Model

## 6.1 World layers

The world is divided into four layers of responsibility.

### Layer A — World Profile

Global configuration and persistent global procedural state.

### Layer B — Macro World

Chunked semantic lattice representing the persistent abstract world.

### Layer C — Feature Graphs

Cross-cell structures such as rivers, highways, rails, and major roads represented in graph/spline form, then compiled into macro chunk occupancy.

### Layer D — Local Chunks

Lazily realized concrete chunks derived from macro state and persisted if nontrivial.

---

# 7. High-Level Runtime Flow

## 7.1 Macro chunk access

When a macro chunk is requested:

1. attempt load from persistence
2. if not present:
   * gather neighbor summaries
   * generate macro chunk deterministically
   * persist macro chunk
3. return macro chunk handle

## 7.2 Local chunk access

When a local chunk is requested:

1. attempt load from persistence
2. if not present:
   * resolve owning macro chunk/cells
   * construct local generation context
   * realize local chunk deterministically from semantic state
   * classify chunk as uniform or non-uniform
   * persist only if policy requires
3. return local chunk handle

## 7.3 Mutation flow

When a realized local chunk is modified:

* mark dirty
* persist full micro state if no longer trivially reproducible
* keep macro semantic state unchanged unless mutation is macro-scale

---

# 8. Spatial Model

## 8.1 Coordinate spaces

The system uses four coordinate spaces:

* **World coordinates**global signed coordinates
* **Macro chunk coordinates**large chunk grid for semantic world storage
* **Macro cell coordinates**semantic cells inside a macro chunk
* **Local chunk coordinates**
  realized concrete chunk grid

## 8.2 Recommended initial dimensions

The exact sizes are configurable, but the architecture assumes:

* macro chunks contain a 2D or 3D lattice of semantic cells
* one semantic cell maps to one local generation parcel footprint
* local chunks are smaller than macro chunks and stream independently

Recommended initial strategy:

* keep macro chunks coarse enough to support region/city/special logic efficiently
* keep local chunks small enough for streaming and mutation persistence

---

# 9. Domain Model

## 9.1 WorldProfile

`WorldProfile` stores global world metadata and cross-chunk procedural state.

### Responsibilities

* world seed
* generation version
* active region stack
* content/mod manifest hash
* unique special counts/decks
* world-global highway/routing state
* world-global hydrology counters
* serialization schema version

### Example type sketch

```csharp
public sealed record WorldProfile(
    Guid WorldId,
    ulong WorldSeed,
    int GenerationVersion,
    string ActiveRegionId,
    IReadOnlyList<string> OverlayRegionIds,
    ContentManifestHash ContentHash,
    GlobalSpecialState SpecialState,
    GlobalGraphState GraphState,
    DateTimeOffset CreatedAtUtc);
```

---

## 9.2 MacroChunk

`MacroChunk` is the authoritative semantic state for a region of world space.

### Responsibilities

* stores semantic cells for all supported z-layers
* stores local feature occupancy compiled from graph features
* stores city memberships, parcel claims, special references
* stores predecessor/layer history
* stores summarized pathing/support data
* stores chunk generation provenance/version

### Example type sketch

```csharp
public sealed class MacroChunk
{
    public required MacroChunkKey Key { get; init; }
    public required int GenerationVersion { get; init; }
    public required RegionRef Region { get; init; }

    public required MacroCell[,,] Cells { get; init; }

    public required ChunkFeatureSummary FeatureSummary { get; init; }
    public required ChunkConnectivitySummary Connectivity { get; init; }
    public required ChunkGenerationMetadata Metadata { get; init; }
}
```

---

## 9.3 MacroCell

A `MacroCell` is the semantic unit of world meaning.

### Responsibilities

* landcover classification
* infrastructure classification
* parcel or facility assignment
* elevation intent
* rotation/orientation
* semantic layer stack
* special instance association
* city association
* mapgen/localgen argument reference
* predecessor history

### Example type sketch

```csharp
public sealed record MacroCell(
    LandcoverType Landcover,
    InfrastructureType Infrastructure,
    ParcelType Parcel,
    Orientation Rotation,
    ElevationMode ElevationMode,
    ImmutableArray<SemanticLayer> Layers,
    SpecialInstanceId? SpecialInstanceId,
    CityId? CityId,
    ArgRef? GenerationArgs,
    CellFlags Flags);
```

---

## 9.4 SemanticLayer

A `SemanticLayer` replaces the idea of a plain predecessor stack with a richer, typed composition history.

### Responsibilities

* identify source system
* define semantic intent
* define overlay priority
* define transform/orientation
* expose local realization hints
* record whether it supersedes or refines prior state

### Example type sketch

```csharp
public sealed record SemanticLayer(
    SemanticLayerKind Kind,
    string SemanticId,
    int Priority,
    Orientation Orientation,
    LayerBlendMode BlendMode,
    LayerSource Source,
    ImmutableDictionary<string, string> Hints);
```

### Typical layer ordering

1. base landform
2. water/coast modifier
3. vegetation or biome cover
4. infrastructure overlay
5. parcel/special overlay
6. postgen overlay

---

## 9.5 FeatureGraph

`FeatureGraph` stores graph-first structures that later compile into macro chunk occupancy.

### Used for

* rivers
* highways
* rail
* arterial roads
* underground lines
* ravine corridors

### Responsibilities

* topology
* continuity across chunk borders
* splines/polyline geometry
* lane/width/bridge metadata
* compilation to macro cell reservations

---

## 9.6 LocalChunk

`LocalChunk` is the authoritative realized state for a streamable local area.

### Responsibilities

* concrete terrain cells / structural cells
* props/furniture equivalents
* items/resources
* entities/spawns
* environmental fields/volumes
* damage and repair state
* lighting/reversion metadata
* authored runtime deltas

### Example type sketch

```csharp
public sealed class LocalChunk
{
    public required LocalChunkKey Key { get; init; }
    public required int GenerationVersion { get; init; }
    public required LocalCell[,,] Cells { get; init; }

    public required LocalEntitySet Entities { get; init; }
    public required LocalItemSet Items { get; init; }
    public required LocalFieldSet Fields { get; init; }

    public required bool IsUniform { get; init; }
    public required bool IsDirty { get; set; }
    public required LocalChunkProvenance Provenance { get; init; }
}
```

---

# 10. Data Ownership Rules

## 10.1 Macro owns semantics

Macro state owns:

* terrain class meaning
* city placement
* road/rail/highway continuity
* special placements
* vertical semantic structure
* parcel ownership
* graph-derived occupancy
* predecessor/overlay history

## 10.2 Local owns concrete mutation

Local state owns:

* exact cell materialization
* breakage
* item depletion
* prop state
* entity state
* active simulation deltas
* temperature/phase consequences
* repairs or player modifications

## 10.3 Observation data is separate

Player-facing discovery data must be stored separately from both macro and local simulation state.

---

# 11. Generation Pipeline

## 11.1 Macro generation phases

Macro generation should run in stable ordered phases.

### Phase 0 — Bootstrap

* create empty semantic cells from region defaults
* initialize chunk metadata
* read neighbor continuity summaries

### Phase 1 — Macro field synthesis

* urbanity field
* forest/biome density field
* hydrology influence field
* topological constraints

### Phase 2 — Hydrology and terrain corridors

* rivers
* lakes
* coast/ocean edges
* ravines
* swamp/wetland derivation

### Phase 3 — Settlement seeding

* city seeds
* settlement metadata
* urban district rough assignment

### Phase 4 — Graph infrastructure

* roads
* rail
* highways
* bridge candidate reservation
* border continuity exits

### Phase 5 — Parcel placement

* fixed specials
* city building parcels
* reserved parcels
* mandatory placements first
* optional placements after

### Phase 6 — Mutable assemblies

* socket/join special growth
* unresolved join tracking
* phased parcel expansion

### Phase 7 — Vertical semantic derivation

* basements
* sewers
* subway/utility layers
* roofs/overpasses
* supports/bridgeheads
* ravine depth propagation

### Phase 8 — World finishing

* monster/ecology seeds
* signals/radios if applicable
* cache summaries
* metadata condensation

### Phase 9 — Persist macro chunk

* serialize semantic cells
* serialize summaries
* serialize chunk provenance

---

## 11.2 Local generation phases

Local generation uses semantic context from macro world.

### Phase A — Context assembly

Build `LocalGenerationContext` from:

* current macro cell
* neighboring macro cells
* semantic layers/history
* special instance
* region profile
* generation arguments
* time/environment flags

### Phase B — Base realization

Replay semantic layers in order:

1. landform
2. hydrology/coast
3. vegetation
4. infrastructure
5. parcel/facility
6. postgen overlays

### Phase C — Object and structure pass

* place walls/floors/volumes
* place props
* place fixtures
* place items/spawns/containers
* place interaction systems

### Phase D — Regional substitution

Resolve region-specific placeholders late.

### Phase E — Local finishing

* environment extras
* weather/temperature transforms
* ruin/decay overlays
* deterministic clutter
* AI/nav markers

### Phase F — Uniformity classification

Determine whether the chunk is:

* uniform and reproducible
* non-uniform but deterministic
* mutated/persistent

### Phase G — Persistence decision

Persist according to local persistence policy.

---

# 12. Determinism and RNG Policy

This is a required production feature.

## 12.1 Determinism principles

Each generation scope must use an explicit seed derivation strategy.

### Required seed scopes

* world seed
* macro chunk seed
* local chunk seed
* special instance seed
* mutable assembly seed
* postgen seed

### Example rule

```text
MacroChunkSeed = Hash(WorldSeed, MacroChunkKey, GenerationVersion, ContentHash)
LocalChunkSeed = Hash(WorldSeed, LocalChunkKey, MacroChunkSemanticFingerprint)
SpecialSeed    = Hash(WorldSeed, SpecialInstanceId)
```

## 12.2 RNG partitioning

Each phase must use independent streams to avoid cascade instability.

Example:

* hydrology stream
* settlement stream
* road graph stream
* parcel placement stream
* local clutter stream

## 12.3 Versioning

Every macro and local chunk stores:

* generation version
* content hash
* source region ids
* optional migration marker

If content changes incompatibly, old generated chunks should either:

* remain authoritative if already persisted, or
* be migrated with explicit tooling, or
* be invalidated and regenerated according to defined policy

---

# 13. Region and Content Policy System

## 13.1 Region profiles

A region profile defines worldgen policy, not concrete rendered geometry.

### Region responsibilities

* default landcover by depth/height band
* city size/spacing policy
* hydrology bias
* forest density
* swamp/ravine rules
* road/rail/highway parameters
* building distribution bins
* regional substitutions
* feature flags
* local extras

### Example type sketch

```csharp
public sealed record RegionProfile(
    string RegionId,
    DefaultTerrainProfile DefaultTerrain,
    CityPolicy CityPolicy,
    HydrologyPolicy Hydrology,
    VegetationPolicy Vegetation,
    InfrastructurePolicy Infrastructure,
    SpecialPolicy Specials,
    SubstitutionProfile Substitutions,
    ExtraProfile Extras);
```

## 13.2 Overlays/mods

A region may be modified by overlay profiles that patch:

* weights
* substitutions
* feature flags
* parcel availability
* local realization styles

The engine should merge region overlays into an immutable compiled region profile at startup.

---

# 14. Specials System

## 14.1 Fixed specials

Fixed specials are constrained parcels with explicit occupancy and placement rules.

### Responsibilities

* footprint definition
* location constraints
* city distance rules
* uniqueness policy
* required connections
* generation arguments
* vertical derivation hooks

## 14.2 Mutable specials

Mutable specials are **socketed parcel grammars**.

### Responsibilities

* root parcel placement
* sockets per face and optionally vertical faces
* phased expansion rules
* join compatibility
* unresolved join tracking
* argument propagation

### Example type sketch

```csharp
public sealed record MutableSpecialDefinition(
    string Id,
    RootPlacementRule RootRule,
    ImmutableArray<ParcelDefinition> ParcelSet,
    ImmutableArray<JoinRule> JoinRules,
    ImmutableArray<ExpansionPhase> Phases,
    UniquePolicy UniquePolicy);
```

## 14.3 Why this matters for 3D

A socketed parcel grammar maps naturally onto 3D parcelized content:

* rooms
* corridors
* wings
* outdoor compounds
* cave modules
* utility branches
* stacked vertical facilities

---

# 15. Persistence Architecture

## 15.1 Persistence categories

Persist the world in four categories.

### A. World profile persistence

Stores:

* world seed
* generation version
* active regions
* content hashes
* global unique state

### B. Macro chunk persistence

Stores:

* semantic cells
* connectivity summaries
* feature occupancy
* city data
* special instance placement
* vertical semantic layers
* predecessor/layer history

### C. Local chunk persistence

Stores:

* realized local state
* runtime mutations
* items, props, entities
* environmental state
* damage and construction

### D. Observation/player persistence

Stores:

* visibility
* exploration
* map notes
* discovered annotations

## 15.2 Uniform chunk elision

A local chunk should be omitted from disk if all of the following are true:

* it is uniform or trivially reproducible
* it has no player/world mutation
* it has no unique runtime state
* it can be regenerated from macro semantic state plus deterministic local rules

This is a mandatory optimization.

## 15.3 Recommended serialization format

Use an engine-neutral storage format.

Recommended options:

* binary for chunk payloads
* JSON or YAML for content definitions
* indexed chunk manifests for save lookup

A practical first pass is:

* JSON for design-time content
* binary chunk persistence for runtime
* optional compression for local chunk payloads

---

# 16. Assembly and Project Structure

Recommended solution structure:

```text
Game.World.sln
 ├─ Game.World.Core
 ├─ Game.World.Generation
 ├─ Game.World.Persistence
 ├─ Game.World.Content
 ├─ Game.World.Tools
 ├─ Game.World.Tests
 ├─ Game.World.DebugCli
 └─ Game.World.GodotAdapter
```

## 16.1 `Game.World.Core`

Contains:

* domain model
* coordinates
* chunk keys
* region models
* semantic layers
* feature graph contracts
* chunk service interfaces

## 16.2 `Game.World.Generation`

Contains:

* macro generators
* local generators
* determinism services
* special placement
* mutable assembly logic
* substitution resolution

## 16.3 `Game.World.Persistence`

Contains:

* repositories
* serializers
* chunk indexing
* save versioning
* migration hooks

## 16.4 `Game.World.Content`

Contains:

* loaders
* validators
* schema definitions
* compiled content registries

## 16.5 `Game.World.Tools`

Contains:

* inspectors
* migration utilities
* world diff tools
* replay tools
* chunk visualization exports

## 16.6 `Game.World.GodotAdapter`

Contains:

* Godot-facing scene realization
* mesh/material lookup
* chunk streaming bridge
* Godot coordinate conversion
* debug overlays in-engine

Because Godot C# projects require the .NET-enabled editor/runtime path, isolating the adapter prevents the simulation core from inheriting engine constraints unnecessarily. [\[docs.godotengine.org\]](https://docs.godotengine.org/en/stable/tutorials/scripting/c_sharp/), [\[github.com\]](https://github.com/godotengine/godot-docs/blob/master/tutorials/scripting/c_sharp/c_sharp_basics.rst)

---

# 17. Core Interfaces

## 17.1 World access façade

```csharp
public interface IWorldService
{
    ValueTask<MacroChunk> GetMacroChunkAsync(MacroChunkKey key, CancellationToken ct = default);
    ValueTask<LocalChunk> GetLocalChunkAsync(LocalChunkKey key, CancellationToken ct = default);

    ValueTask SaveDirtyAsync(CancellationToken ct = default);
}
```

## 17.2 Macro generation

```csharp
public interface IMacroChunkGenerator
{
    ValueTask<MacroChunk> GenerateAsync(
        MacroChunkKey key,
        MacroNeighborContext neighbors,
        WorldProfile world,
        CancellationToken ct = default);
}
```

## 17.3 Local generation

```csharp
public interface ILocalChunkGenerator
{
    ValueTask<LocalChunk> GenerateAsync(
        LocalChunkKey key,
        LocalGenerationContext context,
        CancellationToken ct = default);
}
```

## 17.4 Persistence repositories

```csharp
public interface IMacroChunkRepository
{
    ValueTask<MacroChunk?> LoadAsync(MacroChunkKey key, CancellationToken ct = default);
    ValueTask SaveAsync(MacroChunk chunk, CancellationToken ct = default);
}

public interface ILocalChunkRepository
{
    ValueTask<LocalChunk?> LoadAsync(LocalChunkKey key, CancellationToken ct = default);
    ValueTask SaveAsync(LocalChunk chunk, CancellationToken ct = default);
    ValueTask DeleteAsync(LocalChunkKey key, CancellationToken ct = default);
}
```

## 17.5 Content registry

```csharp
public interface IContentRegistry
{
    RegionProfile GetRegion(string regionId);
    FixedSpecialDefinition GetFixedSpecial(string id);
    MutableSpecialDefinition GetMutableSpecial(string id);
    LocalGeneratorDefinition GetLocalGenerator(string id);
}
```

---

# 18. Recommended Internal Patterns

## 18.1 Use immutable records for definitions

Use immutable `record` types for:

* content definitions
* compiled region profiles
* generation policies
* special definitions
* semantic layer descriptors

## 18.2 Use mutable runtime objects for active chunks

Use mutable classes for:

* loaded macro chunk instances
* realized local chunks
* dirty tracking
* runtime caches

## 18.3 Prefer explicit IDs over stringly logic in runtime core

Content may still be authored using symbolic IDs, but the runtime should compile them into:

* strongly typed enums where stable
* interned identifiers where extensible
* validated references rather than ad hoc string branching

## 18.4 Use composition over inheritance for generators

Prefer:

* phase services
* rule objects
* registries
* pluggable post-processors

Avoid monolithic inheritance-heavy generator trees.

---

# 19. Godot 3D Integration Strategy

## 19.1 Boundary rule

Godot does **not** own world truth.
Godot renders and simulates the presentation layer of truth owned by the world core.

## 19.2 Adapter responsibilities

The Godot adapter should:

* request local chunks from `IWorldService`
* translate local chunk data into:
  * meshes
  * scene instances
  * collision bodies
  * navigation sources
  * interaction proxies
* subscribe to chunk load/unload events
* translate player actions back into world mutations

## 19.3 Suggested streaming model

### In the core:

* stream macro chunks and local chunks by radius / relevance

### In Godot:

* instantiate scene containers per local chunk
* pool/reuse scene nodes where practical
* keep mesh generation and asset binding outside the semantic layer

## 19.4 Strong recommendation

Do **not** generate procedural truth directly inside Godot scenes.Instead:

1. world core produces authoritative local chunk data
2. Godot adapter consumes local chunk data
3. Godot scenes are disposable views over chunk state

This avoids locking procedural logic to engine lifecycle quirks.

---

# 20. Tooling Requirements

A production-ready version of this architecture requires tooling from the start.

## 20.1 Mandatory tools

* chunk inspector
* semantic layer history viewer
* special placement failure tracer
* mutable join/socket visualizer
* deterministic regeneration replay tool
* region substitution previewer
* local chunk provenance viewer
* save diff tool

## 20.2 Debug export

Support exporting chunk state to human-readable debug files:

* macro semantic dump
* local realization dump
* feature graph dump

This is especially important before Godot-side rendering is mature.

---

# 21. Minimum Viable Implementation Order

Recommended implementation order:

## Phase 1 — Foundations

1. coordinate system
2. world profile
3. macro chunk model
4. persistence interfaces
5. content registry and compiled region profile

## Phase 2 — Macro world

6. lazy macro chunk load/generate service
7. base terrain/landcover initialization
8. hydrology pass
9. vegetation pass
10. settlement seeding
11. road/rail/highway graph pass

## Phase 3 — Specials

12. fixed parcel placement
13. mutable socket/join system
14. vertical semantic derivation

## Phase 4 — Local realization

15. local generation context
16. semantic layer replay
17. local chunk realization
18. regional substitution
19. uniform chunk classification
20. local persistence policy

## Phase 5 — Runtime integration

21. headless debug CLI
22. chunk inspectors
23. save/load validation tests
24. Godot adapter
25. 3D visualization of streamed local chunks

---

# 22. Risks and Mitigations

## Risk 1 — Over-coupling to Godot early

**Mitigation:** keep the core simulation completely engine-neutral.

## Risk 2 — Loss of modding flexibility

**Mitigation:** preserve symbolic content IDs in authoring data even if runtime uses compiled references.

## Risk 3 — Nondeterministic generation drift

**Mitigation:** explicit seed partitioning, stable phase order, generation fingerprints.

## Risk 4 — Overly implicit overlay history

**Mitigation:** use typed `SemanticLayer` records instead of raw predecessor strings alone.

## Risk 5 — Local chunk bloat

**Mitigation:** classify uniform chunks aggressively and elide their persistence.

## Risk 6 — Godot/.NET version mismatch pain

Godot’s current C# workflow depends on its .NET integration path and engine-specific support matrix, so the simulation core should avoid assuming every latest .NET feature maps cleanly into the Godot runtime/editor toolchain immediately. Official docs emphasize using the dedicated .NET editor build and describe platform/export limitations for C# projects. [\[docs.godotengine.org\]](https://docs.godotengine.org/en/stable/tutorials/scripting/c_sharp/), [\[docs.godotengine.org\]](https://docs.godotengine.org/en/4.4/tutorials/scripting/c_sharp/index.html), [\[github.com\]](https://github.com/godotengine/godot-docs/blob/master/tutorials/scripting/c_sharp/c_sharp_basics.rst)

---

# 23. Acceptance Criteria

This design is considered successfully implemented when:

1. a world can be created from a seed and region profile
2. macro chunks generate only on first access
3. macro chunks persist and reload without regeneration
4. local chunks realize only on demand
5. uniform local chunks can be omitted and regenerated
6. non-uniform local chunks persist mutations correctly
7. specials and mutable assemblies are reproducible
8. regional substitutions resolve late and correctly
9. the simulation core runs headless without Godot
10. a Godot adapter can request and visualize local chunks without becoming the source of truth

---

# 24. Single-Sentence Architectural Rule

> **Model the world as a persistent semantic macro lattice with graph features and parcel overlays, then lazily realize concrete local chunks from that semantic state, persisting only the parts that stop being trivially reproducible.**

---

# 25. Recommended Next Step

If you want, I can now turn this into one of three concrete follow-ups:

1. **a real solution skeleton** (`Game.World.Core`, `Generation`, `Persistence`, etc. with starter C# code),
2. **a UML-style class/sequence design**, or
3. **a Godot-oriented adapter spec** showing how `LocalChunk` becomes streamed 3D scenes.

My recommendation would be **#1 next** — build the actual .NET 10 solution skeleton first, then hang Godot off it later.
