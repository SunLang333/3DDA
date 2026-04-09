using WorldGen.Core;
using WorldGen.Core.Diagnostics;
using WorldGen.Core.Domain;
using WorldGen.Core.Generation.Macro;

namespace WorldGen.Tests.Determinism;

public sealed class MacroDeterminismIntegrationTests
{
    [Fact]
    public async Task Macro_generation_is_bitwise_stable_for_the_same_seed_and_key()
    {
        var profile = WorldBootstrap.CreateDefaultProfile(123456UL);
        var generator = new MacroGenerationPipeline();
        var key = new MacroChunkKey(-2, 3);

        var chunkA = await generator.GenerateAsync(key, profile, CancellationToken.None);
        var chunkB = await generator.GenerateAsync(key, profile, CancellationToken.None);

        Assert.Equal(CanonicalWorldSerializer.SerializeMacroChunk(chunkA), CanonicalWorldSerializer.SerializeMacroChunk(chunkB));
    }
}
