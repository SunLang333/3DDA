using WorldGen.Core;
using WorldGen.Core.Diagnostics;
using WorldGen.Core.Domain;
using WorldGen.Core.Generation.Macro;
using WorldGen.Persistence;
using WorldGen.Tests.TestSupport;

namespace WorldGen.Tests.Persistence;

public sealed class ChunkPersistenceTests
{
    [Fact]
    public async Task Macro_chunk_persistence_roundtrips_canonical_state()
    {
        using var directory = new TemporaryDirectory();
        var store = new FileChunkStore(directory.Path);
        var profile = WorldBootstrap.CreateDefaultProfile(8675309UL);
        await store.SaveWorldProfileAsync(profile, CancellationToken.None);

        var generator = new MacroGenerationPipeline();
        var chunk = await generator.GenerateAsync(new MacroChunkKey(1, 1), profile, CancellationToken.None);
        await store.SaveMacroChunkAsync(chunk, CancellationToken.None);
        var loaded = await store.LoadMacroChunkAsync(new MacroChunkKey(1, 1), CancellationToken.None);

        Assert.NotNull(loaded);
        Assert.Equal(CanonicalWorldSerializer.SerializeMacroChunk(chunk), CanonicalWorldSerializer.SerializeMacroChunk(loaded!));
    }

    [Fact]
    public async Task Local_chunk_mutation_roundtrips_without_modifying_macro_state()
    {
        using var directory = new TemporaryDirectory();
        var store = new FileChunkStore(directory.Path);
        var profile = WorldBootstrap.CreateDefaultProfile(20260409UL);
        var manager = new WorldGen.Core.Services.ChunkManager(store, new MacroGenerationPipeline(), new WorldGen.Core.Generation.Local.LocalRealizer());
        manager.RegisterWorldProfile(profile);
        await manager.PersistWorldProfileAsync(CancellationToken.None);

        var localKey = LocalChunkKey.FromWorldCoordinates(0, 0, 0, profile.Dimensions);
        var macroKey = localKey.GetOwningMacroChunkKey();
        var macroChunk = await manager.GetOrCreateMacroChunkAsync(macroKey, CancellationToken.None);
        var beforeMacroHash = macroChunk.Provenance.CanonicalHash;
        var localChunk = await manager.GetOrCreateLocalChunkAsync(localKey, CancellationToken.None);
        localChunk.AddItem(new PlacedItem { Type = ItemKind.Fuel, Quantity = 7, X = 2, Y = 2, Z = 1 });
        await manager.PersistLocalChunkAsync(localChunk, CancellationToken.None);
        await manager.DisposeAsync();

        var reloadStore = new FileChunkStore(directory.Path);
        var reloadManager = new WorldGen.Core.Services.ChunkManager(reloadStore, new MacroGenerationPipeline(), new WorldGen.Core.Generation.Local.LocalRealizer());
        reloadManager.RegisterWorldProfile(profile);
        var reloadedLocal = await reloadManager.GetOrCreateLocalChunkAsync(localKey, CancellationToken.None);
        var reloadedMacro = await reloadManager.GetOrCreateMacroChunkAsync(macroKey, CancellationToken.None);

        Assert.Contains(reloadedLocal.Items, item => item.Type == ItemKind.Fuel && item.Quantity == 7);
        Assert.False(reloadedLocal.IsDirty);
        Assert.True(reloadedLocal.Provenance.IsMutated);
        Assert.Equal(beforeMacroHash, reloadedMacro.Provenance.CanonicalHash);
        await reloadManager.DisposeAsync();
    }
}
