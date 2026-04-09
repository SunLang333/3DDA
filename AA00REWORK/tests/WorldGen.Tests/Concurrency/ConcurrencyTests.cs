using WorldGen.Core;
using WorldGen.Core.Domain;
using WorldGen.Core.Generation.Local;
using WorldGen.Core.Generation.Macro;
using WorldGen.Core.Services;
using WorldGen.Persistence;
using WorldGen.Tests.TestSupport;

namespace WorldGen.Tests.Concurrency;

public sealed class ConcurrencyTests
{
    [Fact]
    public async Task Parallel_macro_generation_completes_without_deadlock_and_is_consistent()
    {
        using var directory = new TemporaryDirectory();
        var profile = WorldBootstrap.CreateDefaultProfile(777UL);
        await using var manager = new ChunkManager(new FileChunkStore(directory.Path), new MacroGenerationPipeline(), new LocalRealizer());
        manager.RegisterWorldProfile(profile);
        await manager.PersistWorldProfileAsync(CancellationToken.None);
        using var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(20));

        var uniqueTasks =
            from y in Enumerable.Range(-2, 5)
            from x in Enumerable.Range(-2, 5)
            select manager.GetOrCreateMacroChunkAsync(new MacroChunkKey(x, y), timeout.Token).AsTask();

        var repeatedTasks = Enumerable.Range(0, 10)
            .Select(_ => manager.GetOrCreateMacroChunkAsync(new MacroChunkKey(3, 3), timeout.Token).AsTask())
            .ToArray();

        var uniqueResults = await Task.WhenAll(uniqueTasks);
        var repeatedResults = await Task.WhenAll(repeatedTasks);

        Assert.Equal(25, uniqueResults.Length);
        Assert.All(uniqueResults, chunk => Assert.NotEqual(0UL, chunk.Provenance.CanonicalHash));
        Assert.All(repeatedResults, chunk => Assert.Equal(repeatedResults[0].Provenance.CanonicalHash, chunk.Provenance.CanonicalHash));
    }
}
