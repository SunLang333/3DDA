using WorldGen.Core.Diagnostics;
using WorldGen.Core.Domain;

namespace WorldGen.Adapter.Godot;

/// <summary>
/// Minimal adapter-facing projection that demonstrates how a LocalChunk can be translated
/// into engine-side scene data without introducing any Godot dependency into the core.
/// </summary>
public sealed class GodotChunkSceneBlueprint
{
    public required string SceneName { get; init; }
    public required string ChunkId { get; init; }
    public required IReadOnlyList<string> MeshLayers { get; init; }
    public required IReadOnlyList<string> PropNodes { get; init; }
    public required string SummaryHash { get; init; }

    public static GodotChunkSceneBlueprint FromLocalChunk(LocalChunk chunk)
        => new()
        {
            SceneName = $"Chunk_{chunk.Key.X}_{chunk.Key.Y}_{chunk.Key.Z}",
            ChunkId = chunk.Key.ToString(),
            MeshLayers =
            [
                $"terrain:{chunk.Width}x{chunk.Height}",
                $"materials:{string.Join(',', chunk.Cells.Select(cell => cell.Material).Distinct().OrderBy(material => material))}"
            ],
            PropNodes = chunk.Props
                .OrderBy(prop => prop.Z)
                .ThenBy(prop => prop.Y)
                .ThenBy(prop => prop.X)
                .Select(prop => $"{prop.Type}@{prop.X},{prop.Y},{prop.Z}")
                .ToArray(),
            SummaryHash = CanonicalWorldSerializer.ToTextHash(chunk.Provenance.CanonicalHash)
        };
}
