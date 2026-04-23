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
        var resolvedOutputPath = ResolvePath(outputPath, mustExist: false);
        Directory.CreateDirectory(resolvedOutputPath);
        var profile = WorldBootstrap.CreateDefaultProfile(seed);
        await using var manager = CreateManager(resolvedOutputPath, profile);
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
        var localChunk = await manager.GetOrCreateLocalChunkAsync(LocalChunkKey.FromWorldCoordinates(0, 0, 0, profile.Dimensions), ct).ConfigureAwait(false);
        var localSummary = WorldSummaryFactory.CreateLocalSummary(localChunk);
        await File.WriteAllTextAsync(Path.Combine(resolvedOutputPath, "world-summary.json"), JsonSerializer.Serialize(WorldSummaryFactory.CreateWorldProfileSummary(profile), _jsonOptions), ct).ConfigureAwait(false);
        await File.WriteAllTextAsync(Path.Combine(resolvedOutputPath, "macro-0_0-summary.json"), JsonSerializer.Serialize(macroSummary, _jsonOptions), ct).ConfigureAwait(false);
        await File.WriteAllTextAsync(Path.Combine(resolvedOutputPath, "local-0_0_0-summary.json"), JsonSerializer.Serialize(localSummary, _jsonOptions), ct).ConfigureAwait(false);
        await output.WriteLineAsync($"Wrote summaries to {resolvedOutputPath}").ConfigureAwait(false);
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
                var resolvedExportPath = ResolvePath(exportPath, mustExist: false);
                Directory.CreateDirectory(Path.GetDirectoryName(resolvedExportPath)!);
                await File.WriteAllTextAsync(resolvedExportPath, payload, ct).ConfigureAwait(false);
            }

            await output.WriteLineAsync(payload).ConfigureAwait(false);
            return 0;
        }
    }

    public async Task<int> DumpLocalChunkAsync(string worldPath, int x, int y, int z, bool full, string? exportPath, TextWriter output, CancellationToken ct)
    {
        var loaded = await LoadManagerAsync(worldPath, ct).ConfigureAwait(false);
        await using var manager = loaded.Manager;
        {
            var localKey = LocalChunkKey.FromWorldCoordinates(x, y, z, loaded.Profile.Dimensions);
            var chunk = await manager.GetOrCreateLocalChunkAsync(localKey, ct).ConfigureAwait(false);
            var summary = WorldSummaryFactory.CreateLocalSummary(chunk);
            var payload = full
                ? JsonSerializer.Serialize(chunk, _jsonOptions)
                : JsonSerializer.Serialize(summary, _jsonOptions);
            if (!string.IsNullOrWhiteSpace(exportPath))
            {
                var resolvedExportPath = ResolvePath(exportPath, mustExist: false);
                Directory.CreateDirectory(Path.GetDirectoryName(resolvedExportPath)!);
                await File.WriteAllTextAsync(resolvedExportPath, payload, ct).ConfigureAwait(false);
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

    public async Task<int> CountItemsAsync(string worldPath, bool surfaceOnly, TextWriter output, CancellationToken ct)
    {
        var resolvedWorldPath = ResolvePath(worldPath, mustExist: true);
        var loaded = await LoadManagerAsync(resolvedWorldPath, ct).ConfigureAwait(false);
        var profile = loaded.Profile;
        await using var manager = loaded.Manager;

        var macroDir = Path.Combine(resolvedWorldPath, "macro");
        if (!Directory.Exists(macroDir))
        {
            await output.WriteLineAsync("No macro directory found.").ConfigureAwait(false);
            return 1;
        }

        var macroFiles = Directory.GetFiles(macroDir, "*.bin");
        if (macroFiles.Length == 0)
        {
            await output.WriteLineAsync("No macro chunks found.").ConfigureAwait(false);
            return 1;
        }

        var dimensions = profile.Dimensions;
        var minZ = surfaceOnly ? 0 : dimensions.MinZ;
        var maxZ = surfaceOnly ? 0 : dimensions.MaxZ;

        var itemEntries = new Dictionary<ItemKind, long>();
        var itemQuantities = new Dictionary<ItemKind, long>();
        long totalEntries = 0;
        long totalQuantity = 0;

        foreach (var file in macroFiles)
        {
            var name = Path.GetFileNameWithoutExtension(file);
            var parts = name.Split('_', StringSplitOptions.RemoveEmptyEntries);
            if (parts.Length != 2 || !int.TryParse(parts[0], out var mx) || !int.TryParse(parts[1], out var my))
            {
                continue;
            }

            var macroKey = new WorldGen.Core.Domain.MacroChunkKey(mx, my);

            for (var localX = 0; localX < dimensions.MacroWidth; localX++)
            {
                for (var localY = 0; localY < dimensions.MacroHeight; localY++)
                {
                    for (var z = minZ; z <= maxZ; z++)
                    {
                        ct.ThrowIfCancellationRequested();
                        var localKey = new WorldGen.Core.Domain.LocalChunkKey(macroKey, localX, localY, z);
                        var chunk = await manager.GetOrCreateLocalChunkAsync(localKey, ct).ConfigureAwait(false);
                        foreach (var item in chunk.Items)
                        {
                            itemEntries.TryGetValue(item.Type, out var entries);
                            itemEntries[item.Type] = entries + 1;
                            itemQuantities.TryGetValue(item.Type, out var qty);
                            itemQuantities[item.Type] = qty + item.Quantity;
                            totalEntries++;
                            totalQuantity += item.Quantity;
                        }
                    }
                }
            }
        }

        // Output concise summary
        await output.WriteLineAsync($"Total placed item entries: {totalEntries}").ConfigureAwait(false);
        await output.WriteLineAsync($"Total item quantity: {totalQuantity}").ConfigureAwait(false);
        foreach (var kv in itemEntries.OrderBy(k => k.Key.ToString()))
        {
            var kind = kv.Key;
            var entries = kv.Value;
            var qty = itemQuantities.TryGetValue(kind, out var q) ? q : 0;
            await output.WriteLineAsync($"  {kind}: entries={entries}, quantity={qty}").ConfigureAwait(false);
        }

        return 0;
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
        var resolvedWorldPath = ResolvePath(worldPath, mustExist: true);
        var store = new FileChunkStore(resolvedWorldPath);
        var manager = new ChunkManager(store, new MacroGenerationPipeline(), new LocalRealizer(), new ChunkManagerOptions());
        var profile = await store.LoadWorldProfileAsync(ct).ConfigureAwait(false)
            ?? throw new InvalidOperationException(BuildMissingWorldMessage(resolvedWorldPath));
        manager.RegisterWorldProfile(profile);
        return (profile, manager);
    }

    private static string ResolvePath(string path, bool mustExist)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(path);
        if (Path.IsPathRooted(path))
        {
            return Path.GetFullPath(path);
        }

        var relativeSegments = path.Split([Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar], StringSplitOptions.RemoveEmptyEntries);
        var anchoredWorkspaceCandidate = FindAncestorAnchoredCandidate(relativeSegments, path);
        if (!mustExist && !string.IsNullOrWhiteSpace(anchoredWorkspaceCandidate))
        {
            return anchoredWorkspaceCandidate;
        }

        var candidates = new List<string>();
        string? bestCandidate = null;
        var bestPrefixDepth = -1;
        for (var current = new DirectoryInfo(Directory.GetCurrentDirectory()); current is not null; current = current.Parent)
        {
            var candidate = Path.GetFullPath(path, current.FullName);
            candidates.Add(candidate);

            if (!mustExist)
            {
                var prefixDepth = CountExistingPrefixSegments(current.FullName, relativeSegments);
                if (prefixDepth > bestPrefixDepth)
                {
                    bestPrefixDepth = prefixDepth;
                    bestCandidate = candidate;
                }
            }
        }

        if (mustExist && !string.IsNullOrWhiteSpace(anchoredWorkspaceCandidate) && (Directory.Exists(anchoredWorkspaceCandidate) || File.Exists(anchoredWorkspaceCandidate)))
        {
            return anchoredWorkspaceCandidate;
        }

        foreach (var candidate in candidates.Distinct(StringComparer.OrdinalIgnoreCase))
        {
            if (mustExist && (Directory.Exists(candidate) || File.Exists(candidate)))
            {
                return candidate;
            }

            if (!mustExist)
            {
                var parentDirectory = Path.GetDirectoryName(candidate);
                if (!string.IsNullOrWhiteSpace(parentDirectory) && Directory.Exists(parentDirectory))
                {
                    return candidate;
                }
            }
        }

        if (!mustExist && !string.IsNullOrWhiteSpace(bestCandidate))
        {
            return bestCandidate;
        }

        return candidates.Count > 0
            ? candidates[0]
            : Path.GetFullPath(path);
    }

    private static string BuildMissingWorldMessage(string resolvedWorldPath)
    {
        var availableWorlds = FindNearbyWorlds(resolvedWorldPath);
        return availableWorlds.Count == 0
            ? $"No world profile was found under {resolvedWorldPath}. Run generate-world first."
            : $"No world profile was found under {resolvedWorldPath}. Run generate-world first. Nearby world folders: {string.Join(", ", availableWorlds)}";
    }

    private static IReadOnlyList<string> FindNearbyWorlds(string resolvedWorldPath)
    {
        var result = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        var worldParent = Directory.GetParent(resolvedWorldPath);
        var parentsToScan = new[]
        {
            worldParent?.FullName,
            worldParent?.Parent?.FullName
        };

        foreach (var scanRoot in parentsToScan.Where(scanRoot => !string.IsNullOrWhiteSpace(scanRoot) && Directory.Exists(scanRoot)))
        {
            foreach (var candidate in Directory.GetDirectories(scanRoot!))
            {
                if (File.Exists(Path.Combine(candidate, "world", "world-profile.bin")))
                {
                    result.Add(candidate);
                }
            }
        }

        return result.OrderBy(path => path, StringComparer.OrdinalIgnoreCase).Take(5).ToArray();
    }

    private static int CountExistingPrefixSegments(string baseDirectory, IReadOnlyList<string> relativeSegments)
    {
        var prefixDepth = 0;
        var current = baseDirectory;
        foreach (var segment in relativeSegments)
        {
            current = Path.Combine(current, segment);
            if (!Directory.Exists(current) && !File.Exists(current))
            {
                break;
            }

            prefixDepth++;
        }

        return prefixDepth;
    }

    private static string? FindAncestorAnchoredCandidate(IReadOnlyList<string> relativeSegments, string originalPath)
    {
        if (relativeSegments.Count == 0)
        {
            return null;
        }

        var firstSegment = relativeSegments[0];
        for (var current = new DirectoryInfo(Directory.GetCurrentDirectory()); current is not null; current = current.Parent)
        {
            if (!string.Equals(current.Name, firstSegment, StringComparison.OrdinalIgnoreCase) || current.Parent is null)
            {
                continue;
            }

            return Path.GetFullPath(originalPath, current.Parent.FullName);
        }

        return null;
    }
}
