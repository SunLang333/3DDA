using WorldGen.Core;
using WorldGen.Core.Domain;
using WorldGen.Core.Generation.Macro;

namespace WorldGen.Tests.Generation;

public sealed class VerticalExpansionTests
{
    [Fact]
    public async Task Vertical_derivation_extends_surface_triggers_across_full_cdda_z_bounds()
    {
        var profile = WorldBootstrap.CreateDefaultProfile(20260409UL);
        var builder = new MacroChunkBuilder(new MacroChunkKey(0, 0), profile);

        var bridge = builder.GetOrCreateParcel(1, 1, 0);
        bridge.Landcover = LandcoverType.Water;
        bridge.Infrastructure = InfrastructureType.Bridge;
        bridge.Parcel = ParcelType.BridgeSpan;
        bridge.Flags |= CellFlags.Watery | CellFlags.HasRoad;

        var manhole = builder.GetOrCreateParcel(2, 2, 0);
        manhole.Infrastructure = InfrastructureType.Road;
        manhole.Parcel = ParcelType.Manhole;
        manhole.Flags |= CellFlags.HasRoad | CellFlags.Traversable | CellFlags.UndergroundConnection;

        var house = builder.GetOrCreateParcel(3, 3, 0);
        house.Landcover = LandcoverType.Settlement;
        house.Parcel = ParcelType.ResidentialBlock;
        house.Flags |= CellFlags.Settlement | CellFlags.Traversable;
        house.SettlementId = 1;

        var tower = builder.GetOrCreateParcel(4, 4, 0);
        tower.Landcover = LandcoverType.Settlement;
        tower.Parcel = ParcelType.SkyscraperBase;
        tower.Flags |= CellFlags.Settlement | CellFlags.Traversable;
        tower.SettlementId = 1;
        builder.Urbanity[builder.GetMetricIndex(4, 4)] = 1.0;

        await new VerticalDerivationStage().ExecuteAsync(builder, CancellationToken.None);
        var chunk = builder.Build();

        Assert.Equal(ZBounds.Cdda.Min, chunk.MinZ);
        Assert.Equal(ZBounds.Cdda.Max, chunk.MaxZ);
        Assert.Equal(ParcelType.BridgeRoof, chunk.ResolveParcel(1, 1, 1).Parcel);
        Assert.Equal(ParcelType.BridgeSupport, chunk.ResolveParcel(1, 1, -1).Parcel);
        Assert.Equal(ParcelType.SewerTunnel, chunk.ResolveParcel(2, 2, -1).Parcel);
        Assert.Equal(ParcelType.Basement, chunk.ResolveParcel(3, 3, -1).Parcel);
        Assert.Equal(ParcelType.SkyscraperFloor, chunk.ResolveParcel(4, 4, 10).Parcel);
        Assert.Equal(ParcelType.SubwayStation, chunk.ResolveParcel(4, 4, -2).Parcel);
        Assert.Contains(chunk.FeatureSummary.Features, feature => feature.Kind == FeatureKind.Sewer);
        Assert.Contains(chunk.FeatureSummary.Features, feature => feature.Kind == FeatureKind.Subway);
        Assert.InRange(chunk.StoredParcelCount, 6, profile.Dimensions.MacroWidth * profile.Dimensions.MacroHeight * profile.ZBounds.LayerCount);
    }
}
