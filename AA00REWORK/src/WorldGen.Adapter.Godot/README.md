# WorldGen.Adapter.Godot

This project is an optional adapter stub that demonstrates the boundary between the pure simulation core and a future Godot 4.x visualization layer.

## Intent

- `WorldGen.Core` owns the authoritative world state.
- A Godot-specific project can translate `LocalChunk` instances into scene nodes, meshes, materials, and collision objects.
- This stub intentionally avoids `Godot.*` references so the default AA00REWORK solution builds cleanly on machines without the Godot .NET toolchain installed.

## Suggested next step

Replace `GodotChunkSceneBlueprint` with real Godot-facing types once the target editor/runtime version is chosen. A typical next iteration would map:

- `LocalChunk.Cells` -> `ArrayMesh` / `MeshInstance3D`
- `LocalChunk.Props` -> instanced scenes or pooled `Node3D`s
- `LocalChunk.Fields` -> particles / decals / gameplay proxies
