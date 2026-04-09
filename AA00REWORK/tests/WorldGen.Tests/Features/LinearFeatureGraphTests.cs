using WorldGen.Core.Domain;
using WorldGen.Core.Features;

namespace WorldGen.Tests.Features;

public sealed class LinearFeatureGraphTests
{
    [Fact]
    public void Rasterize_segment_creates_expected_diagonal_points()
    {
        var graph = LinearFeatureGraph.CreateLine(
            FeatureKind.River,
            new WorldCellCoordinate(0, 0, 0),
            new WorldCellCoordinate(3, 3, 0));

        var points = graph.Rasterize();

        Assert.Equal(4, points.Count);
        Assert.Contains(new WorldCellCoordinate(0, 0, 0), points);
        Assert.Contains(new WorldCellCoordinate(1, 1, 0), points);
        Assert.Contains(new WorldCellCoordinate(2, 2, 0), points);
        Assert.Contains(new WorldCellCoordinate(3, 3, 0), points);
    }
}
