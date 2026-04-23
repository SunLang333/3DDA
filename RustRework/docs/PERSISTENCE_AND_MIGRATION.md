# Persistence and migration guide

## Storage goals

The persistence layer implements the same MVP requirements as `AA00REWORK`:

- engine-neutral binary chunk payloads
- human-inspectable JSON summaries
- explicit schema version tracking
- optional compression
- fail-fast migration behavior for unsupported schema versions

The persistence API is now surfaced through the `worldgen-persistence` crate, while orchestration and persistence policy live in `worldgen-core`.

## File layout

A world directory is rooted at the CLI `--out` / `--world` path.

- `<world-root>/world/world-profile.bin`
- `<world-root>/world/world-profile.json`
- `<world-root>/macro/<x>_<y>.bin`
- `<world-root>/macro/<x>_<y>.json`
- `<world-root>/local/<macroX>_<macroY>/<x>_<y>_<z>.bin`
- `<world-root>/local/<macroX>_<macroY>/<x>_<y>_<z>.json`

## Binary envelope

Each `.bin` file contains:

1. 4-byte ASCII magic: `WGEN`
2. 4-byte little-endian schema version
3. 1-byte chunk kind
4. 1-byte compression mode
5. 4-byte little-endian payload length
6. MessagePack payload bytes

Current schema version: `2`

## Compression

The Rust MVP stores MessagePack payloads and optionally compresses them with LZ4 before writing the envelope.

## JSON sidecars

Each persisted payload also writes a `.json` file with:

- schema version
- chunk kind
- key
- compression mode
- content hash
- canonical hash
- mutation/uniform flags
- persisted timestamp
- concise human-readable summary fields

## Local chunk persistence policy

Implemented by `worldgen-core::ChunkManager::persist_local_chunk`.

Rules:

- macro chunks are always persisted after first generation
- local chunks are only persisted when dirty/mutated
- local chunk persistence is fully z-specific: one dirty `(macro, x, y, z)` slice is saved independently of the levels above or below it
- a clean local chunk is deleted or omitted so it can be regenerated deterministically

## Migration behavior

Unsupported schema versions are not silently upgraded automatically.

If a stored payload’s schema version does not match the current implementation:

- loading fails with `UnsupportedSchemaVersion`
- the error explicitly says migration is required
- the load operation fails fast rather than risking incorrect deserialization

## Recommended migration flow

When schema `2+` evolves, the recommended flow remains:

1. detect the old schema from the header
2. deserialize using the old DTO/reader
3. transform to the new in-memory shape
4. reserialize with the new schema version
5. update the JSON sidecar metadata

## Related workspace members

- `src/worldgen-core` — persistence contracts and dirty-local save policy
- `src/worldgen-persistence` — file-backed `FileChunkStore`
- `src/worldgen-cli` — CLI commands that read/write world roots
- `data/items.csv` — migrated auxiliary asset set mirrored from `AA00REWORK/data`
