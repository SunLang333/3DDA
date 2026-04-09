using System.Text.Json;
using System.Text.Json.Serialization;
using WorldGen.Core;
using WorldGen.Core.Diagnostics;
using WorldGen.Core.Domain;
using WorldGen.Core.Generation.Local;
using WorldGen.Core.Generation.Macro;
using WorldGen.Core.Services;
using WorldGen.Persistence;

namespace WorldGen.Cli;

public sealed class CommandHandlers
{
    private readonly JsonSerializerOptions _jsonOptions = new()
    {
        WriteIndented = true,
        Converters = { new JsonStringEnumConverter() }
    };

    public async Task<int> GenerateWorldAsync(ulong seed, string outputPath, int radius, TextWriter output, CancellationToken ct)
    {
        Directory.CreateDirectory(outputPath);
        var profile = WorldBootstrap.CreateDefaultProfile(seed);
        await using var manager = CreateManager(outputPath, profile);
        await manager.PersistWorldProfileAsync(ct).ConfigureAwait(false);

        for (var y = -radius; y <= radius; y++)
        {
            for (var x = -radius; x <= radius; x++)
            {
                var chunk = await manager.GetOrCreateMacroChunkAsync(new MacroChunkKey(x, y), ct).ConfigureAwait(false);
                await output.WriteLineAsync($"Generated macro chunk {chunk.Key} ({CanonicalWorldSerializer.ToTextHash(chunk.Provenance.CanonicalHash)})").ConfigureAwait(false);
            }
        }

        var macroSummary = WorldSummaryFactory.CreateMacroSummary(await manager.GetOrCreateMacroChunkAsync(new MacroChunkKey(0, 0), ct).ConfigureAwait(false));
        var localChunk = await manager.GetOrCreateLocalChunkAsync(new LocalChunkKey(0, 0, 0), ct).ConfigureAwait(false);
        var localSummary = WorldSummaryFactory.CreateLocalSummary(localChunk);
        await File.WriteAllTextAsync(Path.Combine(outputPath, "world-summary.json"), JsonSerializer.Serialize(WorldSummaryFactory.CreateWorldProfileSummary(profile), _jsonOptions), ct).ConfigureAwait(false);
        await File.WriteAllTextAsync(Path.Combine(outputPath, "macro-0_0-summary.json"), JsonSerializer.Serialize(macroSummary, _jsonOptions), ct).ConfigureAwait(false);
        await File.WriteAllTextAsync(Path.Combine(outputPath, "local-0_0_0-summary.json"), JsonSerializer.Serialize(localSummary, _jsonOptions), ct).ConfigureAwait(false);
        await output.WriteLineAsync($"Wrote summaries to {outputPath}").ConfigureAwait(false);
        return 0;
    }

    public async Task<int> DumpMacroChunkAsync(string worldPath, int x, int y, string? exportPath, TextWriter output, CancellationToken ct)
    {
        var loaded = await LoadManagerAsync(worldPath, ct).ConfigureAwait(false);
        await using var manager = loaded.Manager;
        {
            var chunk = await manager.GetOrCreateMacroChunkAsync(new MacroChunkKey(x, y), ct).ConfigureAwait(false);
            var summary = WorldSummaryFactory.CreateMacroSummary(chunk);
            var payload = JsonSerializer.Serialize(summary, _jsonOptions);
            if (!string.IsNullOrWhiteSpace(exportPath))
            {
                await File.WriteAllTextAsync(exportPath, payload, ct).ConfigureAwait(false);
            }

            await output.WriteLineAsync(payload).ConfigureAwait(false);
            return 0;
        }
    }

    public async Task<int> DumpLocalChunkAsync(string worldPath, int x, int y, int z, string? exportPath, TextWriter output, CancellationToken ct)
    {
        var loaded = await LoadManagerAsync(worldPath, ct).ConfigureAwait(false);
        await using var manager = loaded.Manager;
        {
            var chunk = await manager.GetOrCreateLocalChunkAsync(new LocalChunkKey(x, y, z), ct).ConfigureAwait(false);
            var summary = WorldSummaryFactory.CreateLocalSummary(chunk);
            var payload = JsonSerializer.Serialize(summary, _jsonOptions);
            if (!string.IsNullOrWhiteSpace(exportPath))
            {
                await File.WriteAllTextAsync(exportPath, payload, ct).ConfigureAwait(false);
            }

            await output.WriteLineAsync(payload).ConfigureAwait(false);
            return 0;
        }
    }

    public async Task<int> ValidateDeterminismAsync(string worldPath, int startX, int startY, int width, int height, TextWriter output, CancellationToken ct)
    {
        var loaded = await LoadManagerAsync(worldPath, ct).ConfigureAwait(false);
        var profile = loaded.Profile;
        await using var manager = loaded.Manager;
        {
            var generator = new MacroGenerationPipeline();
            var mismatches = new List<string>();
            for (var y = startY; y < startY + height; y++)
            {
                for (var x = startX; x < startX + width; x++)
                {
                    var key = new MacroChunkKey(x, y);
                    var generatedA = await generator.GenerateAsync(key, profile, ct).ConfigureAwait(false);
                    var generatedB = await generator.GenerateAsync(key, profile, ct).ConfigureAwait(false);
                    var persisted = await manager.GetOrCreateMacroChunkAsync(key, ct).ConfigureAwait(false);
                    var bytesA = CanonicalWorldSerializer.SerializeMacroChunk(generatedA);
                    var bytesB = CanonicalWorldSerializer.SerializeMacroChunk(generatedB);
                    var bytesPersisted = CanonicalWorldSerializer.SerializeMacroChunk(persisted);
                    var equal = bytesA.AsSpan().SequenceEqual(bytesB) && bytesA.AsSpan().SequenceEqual(bytesPersisted);
                    if (!equal)
                    {
                        mismatches.Add(key.ToString());
                    }
                }
            }

            if (mismatches.Count == 0)
            {
                await output.WriteLineAsync("Determinism validation passed.").ConfigureAwait(false);
                return 0;
            }

            await output.WriteLineAsync($"Determinism validation failed for: {string.Join(", ", mismatches)}").ConfigureAwait(false);
            return 1;
        }
    }

    private static ChunkManager CreateManager(string outputPath, WorldProfile profile)
    {
        var manager = new ChunkManager(
            new FileChunkStore(outputPath),
            new MacroGenerationPipeline(),
            new LocalRealizer(),
            new ChunkManagerOptions());
        manager.RegisterWorldProfile(profile);
        return manager;
    }

    private static async Task<(WorldProfile Profile, ChunkManager Manager)> LoadManagerAsync(string worldPath, CancellationToken ct)
    {
        var store = new FileChunkStore(worldPath);
        var manager = new ChunkManager(store, new MacroGenerationPipeline(), new LocalRealizer(), new ChunkManagerOptions());
        var profile = await store.LoadWorldProfileAsync(ct).ConfigureAwait(false)
            ?? throw new InvalidOperationException($"No world profile was found under {worldPath}. Run generate-world first.");
        manager.RegisterWorldProfile(profile);
        return (profile, manager);
    }
}
