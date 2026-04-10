using System.Buffers.Binary;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using MessagePack;
using WorldGen.Core.Abstractions;
using WorldGen.Core.Diagnostics;
using WorldGen.Core.Domain;
using WorldGen.Core.Randomization;

namespace WorldGen.Persistence;

public enum StoredChunkKind : byte
{
    WorldProfile = 1,
    MacroChunk = 2,
    LocalChunk = 3
}

public enum ChunkCompressionMode : byte
{
    None = 0,
    Lz4 = 1
}

public sealed class FileChunkStoreOptions
{
    public required string RootPath { get; init; }
    public int SchemaVersion { get; init; } = 2;
    public ChunkCompressionMode CompressionMode { get; init; } = ChunkCompressionMode.Lz4;
}

public sealed class UnsupportedSchemaVersionException : Exception
{
    public UnsupportedSchemaVersionException(int actualVersion, int expectedVersion)
        : base($"Chunk schema version {actualVersion} is not supported by this build. Expected {expectedVersion}. Run the documented migration flow before loading this save.")
    {
        ActualVersion = actualVersion;
        ExpectedVersion = expectedVersion;
    }

    public int ActualVersion { get; }

    public int ExpectedVersion { get; }
}

public sealed class ChunkMetadataSidecar
{
    public int SchemaVersion { get; init; }
    public required string ChunkKind { get; init; }
    public required string Key { get; init; }
    public required string Compression { get; init; }
    public required string ContentHash { get; init; }
    public required string CanonicalHash { get; init; }
    public bool IsMutated { get; init; }
    public bool IsUniform { get; init; }
    public DateTimeOffset PersistedAtUtc { get; init; }
    public required Dictionary<string, string> Summary { get; init; }
}

public sealed class FileChunkStore : IChunkStore
{
    private static readonly byte[] Magic = Encoding.ASCII.GetBytes("WGEN");
    private readonly FileChunkStoreOptions _options;
    private readonly JsonSerializerOptions _jsonOptions;
    private readonly MessagePackSerializerOptions _messagePackOptions;

    public FileChunkStore(string rootPath)
        : this(new FileChunkStoreOptions { RootPath = rootPath })
    {
    }

    public FileChunkStore(FileChunkStoreOptions options)
    {
        _options = options ?? throw new ArgumentNullException(nameof(options));
        Directory.CreateDirectory(_options.RootPath);
        Directory.CreateDirectory(Path.Combine(_options.RootPath, "world"));
        Directory.CreateDirectory(Path.Combine(_options.RootPath, "macro"));
        Directory.CreateDirectory(Path.Combine(_options.RootPath, "local"));
        _jsonOptions = new JsonSerializerOptions
        {
            WriteIndented = true,
            Converters = { new JsonStringEnumConverter() }
        };
        _messagePackOptions = _options.CompressionMode == ChunkCompressionMode.Lz4
            ? MessagePackSerializerOptions.Standard.WithCompression(MessagePackCompression.Lz4BlockArray)
            : MessagePackSerializerOptions.Standard;
    }

    public ValueTask SaveWorldProfileAsync(WorldProfile profile, CancellationToken ct = default)
    {
        var summary = WorldSummaryFactory.CreateWorldProfileSummary(profile);
        var canonicalHash = PhaseSeedDeriver.HashText($"{summary.WorldId:N}:{summary.WorldSeed}:{summary.GenerationVersion}:{summary.ActiveRegionId}:{summary.ContentHash}");
        var sidecar = new ChunkMetadataSidecar
        {
            SchemaVersion = _options.SchemaVersion,
            ChunkKind = StoredChunkKind.WorldProfile.ToString(),
            Key = summary.WorldId.ToString("N"),
            Compression = _options.CompressionMode.ToString(),
            ContentHash = summary.ContentHash,
            CanonicalHash = canonicalHash.ToString("X16"),
            IsMutated = false,
            IsUniform = false,
            PersistedAtUtc = DateTimeOffset.UtcNow,
            Summary = new Dictionary<string, string>
            {
                ["WorldId"] = summary.WorldId.ToString("N"),
                ["WorldSeed"] = summary.WorldSeed.ToString(),
                ["GenerationVersion"] = summary.GenerationVersion.ToString(),
                ["ActiveRegionId"] = summary.ActiveRegionId,
                ["Dimensions"] = $"macro={summary.Dimensions.MacroWidth}x{summary.Dimensions.MacroHeight};local={summary.Dimensions.LocalWidth}x{summary.Dimensions.LocalHeight}x{summary.Dimensions.LocalDepth};z={summary.ZBounds.Min}..{summary.ZBounds.Max}"
            }
        };

        return WritePayloadAsync(StoredChunkKind.WorldProfile, GetWorldProfileBasePath(), profile, sidecar, ct);
    }

    public async ValueTask<WorldProfile?> LoadWorldProfileAsync(CancellationToken ct = default)
    {
        var binaryPath = GetBinaryPath(GetWorldProfileBasePath());
        if (!File.Exists(binaryPath))
        {
            return null;
        }

        return await ReadPayloadAsync<WorldProfile>(StoredChunkKind.WorldProfile, GetWorldProfileBasePath(), ct).ConfigureAwait(false);
    }

    public ValueTask SaveMacroChunkAsync(MacroChunk chunk, CancellationToken ct = default)
    {
        var summary = WorldSummaryFactory.CreateMacroSummary(chunk);
        var sidecar = new ChunkMetadataSidecar
        {
            SchemaVersion = _options.SchemaVersion,
            ChunkKind = StoredChunkKind.MacroChunk.ToString(),
            Key = chunk.Key.ToString(),
            Compression = _options.CompressionMode.ToString(),
            ContentHash = chunk.Provenance.ContentHash,
            CanonicalHash = summary.CanonicalHash.ToString("X16"),
            IsMutated = false,
            IsUniform = false,
            PersistedAtUtc = DateTimeOffset.UtcNow,
            Summary = new Dictionary<string, string>
            {
                ["RegionId"] = summary.RegionId,
                ["SurfaceHistogram"] = string.Join(", ", summary.SurfaceLandcoverHistogram.OrderBy(entry => entry.Key).Select(entry => $"{entry.Key}:{entry.Value}")),
                ["SettlementCount"] = summary.SettlementCount.ToString(),
                ["SpecialCount"] = summary.SpecialCount.ToString(),
                ["RoadConnections"] = $"N:{summary.Connectivity.NorthRoadConnections} S:{summary.Connectivity.SouthRoadConnections} E:{summary.Connectivity.EastRoadConnections} W:{summary.Connectivity.WestRoadConnections}",
                ["WaterConnections"] = $"N:{summary.Connectivity.NorthWaterConnections} S:{summary.Connectivity.SouthWaterConnections} E:{summary.Connectivity.EastWaterConnections} W:{summary.Connectivity.WestWaterConnections}"
            }
        };

        return WritePayloadAsync(StoredChunkKind.MacroChunk, GetMacroBasePath(chunk.Key), chunk, sidecar, ct);
    }

    public async ValueTask<MacroChunk?> LoadMacroChunkAsync(MacroChunkKey key, CancellationToken ct = default)
    {
        var binaryPath = GetBinaryPath(GetMacroBasePath(key));
        if (!File.Exists(binaryPath))
        {
            return null;
        }

        return await ReadPayloadAsync<MacroChunk>(StoredChunkKind.MacroChunk, GetMacroBasePath(key), ct).ConfigureAwait(false);
    }

    public ValueTask SaveLocalChunkAsync(LocalChunk chunk, CancellationToken ct = default)
    {
        var summary = WorldSummaryFactory.CreateLocalSummary(chunk);
        var sidecar = new ChunkMetadataSidecar
        {
            SchemaVersion = _options.SchemaVersion,
            ChunkKind = StoredChunkKind.LocalChunk.ToString(),
            Key = chunk.Key.ToString(),
            Compression = _options.CompressionMode.ToString(),
            ContentHash = chunk.Provenance.ContentHash,
            CanonicalHash = summary.CanonicalHash.ToString("X16"),
            IsMutated = chunk.Provenance.IsMutated,
            IsUniform = chunk.IsUniform,
            PersistedAtUtc = DateTimeOffset.UtcNow,
            Summary = new Dictionary<string, string>
            {
                ["MacroChunk"] = chunk.Key.Macro.ToString(),
                ["LocalCell"] = $"{chunk.Key.X},{chunk.Key.Y},{chunk.Key.Z}",
                ["Materials"] = string.Join(", ", summary.MaterialHistogram.OrderBy(entry => entry.Key).Select(entry => $"{entry.Key}:{entry.Value}")),
                ["Props"] = summary.PropCount.ToString(),
                ["Items"] = summary.ItemCount.ToString(),
                ["Fields"] = summary.FieldCount.ToString(),
                ["MutationCount"] = summary.MutationCount.ToString(),
                ["SourceMacroCanonicalHash"] = summary.SourceMacroCanonicalHash.ToString("X16")
            }
        };

        return WritePayloadAsync(StoredChunkKind.LocalChunk, GetLocalBasePath(chunk.Key), chunk, sidecar, ct);
    }

    public async ValueTask<LocalChunk?> LoadLocalChunkAsync(LocalChunkKey key, CancellationToken ct = default)
    {
        var binaryPath = GetBinaryPath(GetLocalBasePath(key));
        if (!File.Exists(binaryPath))
        {
            return null;
        }

        return await ReadPayloadAsync<LocalChunk>(StoredChunkKind.LocalChunk, GetLocalBasePath(key), ct).ConfigureAwait(false);
    }

    public ValueTask DeleteLocalChunkAsync(LocalChunkKey key, CancellationToken ct = default)
    {
        var basePath = GetLocalBasePath(key);
        var binaryPath = GetBinaryPath(basePath);
        var jsonPath = GetJsonPath(basePath);
        if (File.Exists(binaryPath))
        {
            File.Delete(binaryPath);
        }

        if (File.Exists(jsonPath))
        {
            File.Delete(jsonPath);
        }

        return ValueTask.CompletedTask;
    }

    private async ValueTask WritePayloadAsync<T>(StoredChunkKind expectedKind, string basePath, T payload, ChunkMetadataSidecar sidecar, CancellationToken ct)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(basePath)!);
        var payloadBytes = MessagePackSerializer.Serialize(payload, _messagePackOptions);
        var header = CreateHeader(expectedKind, payloadBytes.Length);
        var output = new byte[header.Length + payloadBytes.Length];
        Buffer.BlockCopy(header, 0, output, 0, header.Length);
        Buffer.BlockCopy(payloadBytes, 0, output, header.Length, payloadBytes.Length);
        await File.WriteAllBytesAsync(GetBinaryPath(basePath), output, ct).ConfigureAwait(false);
        await File.WriteAllTextAsync(GetJsonPath(basePath), JsonSerializer.Serialize(sidecar, _jsonOptions), ct).ConfigureAwait(false);
    }

    private async ValueTask<T> ReadPayloadAsync<T>(StoredChunkKind expectedKind, string basePath, CancellationToken ct)
    {
        var bytes = await File.ReadAllBytesAsync(GetBinaryPath(basePath), ct).ConfigureAwait(false);
        ValidateHeader(bytes, expectedKind, out var payloadOffset, out var payloadLength);
        var payload = new ReadOnlyMemory<byte>(bytes, payloadOffset, payloadLength);
        return MessagePackSerializer.Deserialize<T>(payload, _messagePackOptions);
    }

    private byte[] CreateHeader(StoredChunkKind kind, int payloadLength)
    {
        var header = new byte[14];
        Buffer.BlockCopy(Magic, 0, header, 0, Magic.Length);
        BinaryPrimitives.WriteInt32LittleEndian(header.AsSpan(4, 4), _options.SchemaVersion);
        header[8] = (byte)kind;
        header[9] = (byte)_options.CompressionMode;
        BinaryPrimitives.WriteInt32LittleEndian(header.AsSpan(10, 4), payloadLength);
        return header;
    }

    private void ValidateHeader(byte[] data, StoredChunkKind expectedKind, out int payloadOffset, out int payloadLength)
    {
        payloadOffset = 14;
        if (data.Length < payloadOffset)
        {
            throw new InvalidDataException("Chunk payload is truncated before the header completed.");
        }

        if (!data.AsSpan(0, 4).SequenceEqual(Magic))
        {
            throw new InvalidDataException("Chunk payload magic header does not match WorldGen format.");
        }

        var schemaVersion = BinaryPrimitives.ReadInt32LittleEndian(data.AsSpan(4, 4));
        if (schemaVersion != _options.SchemaVersion)
        {
            throw new UnsupportedSchemaVersionException(schemaVersion, _options.SchemaVersion);
        }

        var kind = (StoredChunkKind)data[8];
        if (kind != expectedKind)
        {
            throw new InvalidDataException($"Chunk payload kind {kind} does not match expected {expectedKind}.");
        }

        payloadLength = BinaryPrimitives.ReadInt32LittleEndian(data.AsSpan(10, 4));
        if (data.Length != payloadOffset + payloadLength)
        {
            throw new InvalidDataException("Chunk payload length does not match the header.");
        }
    }

    private string GetWorldProfileBasePath() => Path.Combine(_options.RootPath, "world", "world-profile");

    private string GetMacroBasePath(MacroChunkKey key) => Path.Combine(_options.RootPath, "macro", $"{key.X}_{key.Y}");

    private string GetLocalBasePath(LocalChunkKey key) => Path.Combine(_options.RootPath, "local", $"{key.Macro.X}_{key.Macro.Y}", $"{key.X}_{key.Y}_{key.Z}");

    private static string GetBinaryPath(string basePath) => $"{basePath}.bin";

    private static string GetJsonPath(string basePath) => $"{basePath}.json";
}
