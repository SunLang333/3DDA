
---

# Architecture Decision Record: Pure C# Semantic Macro & Lazy Micro World Generation

**Status:** Accepted

**Context Area:** Core Engine / World Generation

**Primary Technologies:** C# 14, .NET 10, Godot 4.x

## 1. Context and Problem Statement

To recreate a highly detailed, procedurally generated, and infinite-style world architecture akin to  *Cataclysm: DDA* , the engine must handle massive amounts of spatial data without overwhelming system memory or storage. Tying the procedural generation logic directly to a game engine's scene graph (e.g., Godot's node system) limits testability, hinders headless server deployments, and binds the simulation to engine-specific lifecycles and export limitations.

The primary challenge is establishing a system that can generate complex regional topologies while allowing for dynamic, local-level player modifications, without requiring the entire world's concrete geometry to be generated or loaded up front.

## 2. Decision

We will implement a strictly decoupled, two-layered world generation architecture using a pure **C# / .NET 10** simulation core. The system will rely on a **persistent semantic macro world** and a  **lazy micro realization pipeline** .

Godot 4.x will be utilized strictly as a visualization and interaction adapter, meaning Godot does not own the world's ground truth; it merely renders and streams the presentation layer of the truth owned by the C# core.

### 2.1 The Macro-Micro Dichotomy

The world is strictly divided into two operational layers:

* **Macro Semantic World (Persistent):** Represents the "what" of the world (abstract identity, topology, infrastructure networks, vertical semantic layers). This layer is chunked, generated once upon regional access, and persisted permanently.
* **Micro Realized World (Lazy):** Represents the "how" of the world (concrete geometry, props, items, damage states). This layer is instantiated lazily only when required by player proximity, AI, or scripts.

### 2.2 Persistence Policy and Elision

To enable efficient streaming and manageable save sizes, persistence treats the layers differently:

* **Macro State:** Saved to disk upon initial generation.
* **Micro State:** Persisted *only* if the chunk has been mutated (e.g., broken windows, dropped items). Uniform or trivially reproducible chunks are discarded from memory and regenerated from the macro semantic state using deterministic rules when re-entered.

### 2.3 Determinism and RNG

To support uniform chunk regeneration, each generation phase must rely on explicit seed derivation strategies (e.g., hashing the world seed with chunk keys). Random number generation must be partitioned per phase (e.g., independent streams for hydrology vs. settlement placement) to prevent cascading instability when rules are patched.

## 3. Consequences

### Positive

* **Engine Agnosticism:** The core simulation can be run in tests, headless command-line tools, and editor utilities without booting Godot.
* **Scalability:** Lazy micro realization ensures memory and CPU are only spent on localized, active play areas.
* **Storage Efficiency:** The uniform chunk elision policy drastically reduces save file bloat.

### Negative/Risks

* **Translation Overhead:** Requires writing a dedicated Godot adapter to translate C# data structures into instanced Godot 3D scenes and meshes.
* **Complexity:** Managing predecessor-aware generation, socketed specials, and phase-ordered RNG requires strict architectural discipline to prevent non-deterministic drift.

---

# Appendix: Implementation Details & Godot Integration

This appendix outlines the structural mechanisms and specific integration patterns required to execute the ADR, utilizing C# models and Godot 4.x APIs.

## A. Core Data Modeling

Avoid string-heavy, implicitly structured representations. Use explicit, type-safe C# models to improve debuggability and performance:

* `WorldProfile`: The global configuration rulebook (seeds, global counters, region definitions).
* `MacroChunk`: Contains `SemanticParcel` records. Each parcel explicitly defines its base landcover, infrastructure overlays, elevation, and predecessor stack.
* `LocalChunk`: Contains dense, low-level data suitable for Godot translation (voxel grids, prop collections, field volumes, destructible states).

## B. The Macro Generation Lifecycle

Macro chunks are created via a strict 9-stage dependency graph to guarantee topological consistency:

| **Stage** | **Operation**  | **Logic / Output**                                                 |
| --------------- | -------------------- | ------------------------------------------------------------------------ |
| **1**     | Initialize Layers    | Fill empty OMT grid with default landcover.                              |
| **2**     | Calculate Metrics    | Determine `urbanity`and `forestosity`.                               |
| **3**     | Place Hydrology      | Rivers, lakes, and oceans (infrastructure must route around/over these). |
| **4**     | Place Nature         | Forests, swamps, ravines.                                                |
| **5**     | Seed Settlements     | Establish city centers based on density metrics.                         |
| **6**     | Build Infrastructure | Connect points of interest with road, rail, and highway graphs.          |
| **7**     | Place Specials       | Insert unique locations (ruins, camps) into unclaimed space.             |
| **8**     | Vertical Derivation  | Build z-axis connections (subways, basements, bridge overpasses).        |
| **9**     | Finalize             | Output the immutable `MacroChunk`object.                               |

## C. The Micro Realization Pipeline

Translating abstract semantics to concrete data requires a context-sensitive generator:

* **Predecessor-Aware Refinement:** When placing features (like a road over a field), the generator queries the `SemanticParcel` history to apply the underlying field logic first, blending the road into the terrain rather than erasing it.
* **Late-Binding Regional Substitution:** Mapgen files should utilize "pseudo" identifiers (e.g., `"regional_floor_interior"`). After the initial pass, these placeholders are scanned and replaced with concrete IDs based on the active region's palette, enabling diverse biomes from single blueprints.

## D. Godot 4.x Streaming and Procedural Meshing

Godot acts purely as the visualization layer. The translation from `LocalChunk` to 3D scene must be asynchronous and programmatic.

* **Direct Binary I/O:** Do not use Godot's native `ResourceFormatLoader` for chunk saves. Instead, utilize Godot's `FileAccess` class to read/write C# structures directly to custom-binary streams, enabling fast serialization, compression, and fine-tuned layout control.
* **Procedural Meshing:** Bypass external 3D modeling for terrain. Feed the parsed `LocalChunk` data into Godot's `SurfaceTool` to construct meshes programmatically (extruding quads/triangles based on voxel/cell states). Use `MeshDataTool` for advanced post-processing or height deformation.
* **Background Threading:** To prevent frame rate stutter during vast open-world streaming, background threads (using mechanisms like `ResourceLoader.load_threaded_request`) must handle file I/O, deserialization, and the C# generation pipeline before returning the data to the main thread to instantiate the `MeshInstance3D` nodes.
