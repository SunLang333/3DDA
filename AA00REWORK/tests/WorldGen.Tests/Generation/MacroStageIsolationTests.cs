using WorldGen.Core;
using WorldGen.Core.Domain;
using WorldGen.Core.Generation.Macro;

namespace WorldGen.Tests.Generation;

public sealed class MacroStageIsolationTests
{
    [Fact]
    public async Task Density_stage_populates_metric_arrays_without_creating_features()
    {
        var profile = WorldBootstrap.CreateDefaultProfile(42UL);
        var builder = new MacroChunkBuilder(new(profile.Dimensions.MacroWidth / 2, 0), profile);
        var stage = new CalculateDensityMetricsStage();

        await stage.ExecuteAsync(builder, CancellationToken.None);

        Assert.All(builder.Urbanity, value => Assert.InRange(value, 0d, 1d));
        Assert.All(builder.Forestosity, value => Assert.InRange(value, 0d, 1d));
        Assert.All(builder.Hydrology, value => Assert.InRange(value, 0d, 1d));
        Assert.Empty(builder.Features);
    }

    [Fact]
    public async Task Hydrology_stage_marks_watery_cells_and_emits_feature_segments()
    {
        var profile = WorldBootstrap.CreateDefaultProfile(99UL);
        profile.ActiveRegion.HydrologyBias = 1.0;
        var builder = new MacroChunkBuilder(new MacroChunkKey(0, 0), profile);
        await new InitializeLayersStage().ExecuteAsync(builder, CancellationToken.None);
        await new CalculateDensityMetricsStage().ExecuteAsync(builder, CancellationToken.None);

        await new PlaceHydrologyStage().ExecuteAsync(builder, CancellationToken.None);

        Assert.NotEmpty(builder.Features);
        Assert.Contains(builder.Cells, cell => (cell.Flags & WorldGen.Core.Domain.CellFlags.Watery) != 0);
    }
}
