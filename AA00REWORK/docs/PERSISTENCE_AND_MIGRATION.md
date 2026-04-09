# Persistence and migration guide

## Storage goals

The persistence layer implements the MVP requirements:

- engine-neutral binary chunk payloads
- human-inspectable JSON summaries
- explicit schema version tracking
- optional compression
- clear migration-required failure behavior for unsupported schema versions

## File layout

A world directory is rooted at the CLI `--out` / `--world` path.

```text
<world-root>/
  world/
    world-profile.bin
    world-profile.json
  macro/
    <x>_<y>.bin
    <x>_<y>.json
  local/
    <x>_<y>_<z>.bin
    <x>_<y>_<z>.json
```

## Binary envelope

Implemented in `src/WorldGen.Persistence/FileChunkStore.cs`.

Each `.bin` file contains:

1. 4-byte ASCII magic: `WGEN`
2. 4-byte little-endian schema version
3. 1-byte chunk kind
4. 1-byte compression mode
5. 4-byte little-endian payload length
6. MessagePack payload bytes

Current schema version: `1`

## Compression

The MVP uses MessagePack `Lz4BlockArray` compression for chunk payloads.

Why this choice:

- good balance between load latency and disk footprint
- mature .NET implementation
- no engine coupling

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

These sidecars are intended for:

- debugging
- diffing persisted state during development
- manually inspecting generated content without decoding MessagePack

## Local chunk persistence policy

Implemented by `ChunkManager.PersistLocalChunkAsync`.

Rules:

- macro chunks are always persisted after first generation
- local chunks are only persisted when dirty/mutated
- a clean local chunk is deleted or omitted so it can be regenerated deterministically

This satisfies the required macro permanence + local elision boundary.

## Migration behavior

Unsupported schema versions are not silently upgraded in v1.

If a stored payload’s schema version does not match the current implementation:

- `FileChunkStore` throws `UnsupportedSchemaVersionException`
- the error message explicitly says migration is required
- the load operation fails fast rather than risking incorrect deserialization

## Recommended migration process for future versions

When schema `2+` arrives, the recommended flow is:

1. detect the old schema from the header
2. deserialize using the old DTO/reader
3. transform to the new in-memory shape
4. reserialize with the new schema version
5. update the JSON sidecar metadata

A dedicated migration CLI is intentionally deferred to a later iteration and called out in `IMPLEMENTATION_REPORT.md`.
