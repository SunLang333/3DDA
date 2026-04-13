using WorldGen.Core;
using WorldGen.Core.Domain;
using WorldGen.Core.Generation.Local;

namespace WorldGen.Tests.Generation;

public sealed class LocalRealizerTests
{
    [Fact]
    public async Task ApplyVegetation_GeneratesTreesAndSupplements_ForForest()
    {
        var profile = WorldBootstrap.CreateDefaultProfile(42UL);
        var dimensions = profile.Dimensions;
        
        var macroChunk = new MacroChunk
        {
            Key = new MacroChunkKey(0, 0),
            Width = dimensions.MacroWidth,
            Height = dimensions.MacroHeight,
            ZBounds = new ZBounds(-1, 1),
            Parcels = []
        };

        // Create a forest parcel
        var forestParcel = new SemanticParcel
        {
            Landcover = LandcoverType.Forest,
            ElevationMode = ElevationMode.Surface,
            Layers = [new SemanticLayer { Kind = SemanticLayerKind.Vegetation }]
        };

        var localX = 5;
        var localY = 5;
        var key = new LocalChunkKey(macroChunk.Key, localX, localY, 0);

        macroChunk.SetParcel(localX, localY, 0, forestParcel);

        var realizer = new LocalRealizer();
        var chunk = await realizer.RealizeAsync(key, macroChunk, profile);

        Assert.NotEmpty(chunk.Props.Where(p => p.Type == LocalPropType.Tree));
        Assert.NotEmpty(chunk.Items.Where(i => i.Type == ItemKind.Supplement));
        // Verify tree count is significantly increased (should be between 15 and 35 based on our implementation)
        var treeCount = chunk.Props.Count(p => p.Type == LocalPropType.Tree);
        Assert.True(treeCount >= 15, $"Expected at least 15 trees, got {treeCount}");
    }

    [Fact]
    public async Task ApplyParcel_GeneratesWeaponsAndSupplements_ForTownCenter()
    {
        var profile = WorldBootstrap.CreateDefaultProfile(42UL);
        var dimensions = profile.Dimensions;
        
        var macroChunk = new MacroChunk
        {
            Key = new MacroChunkKey(0, 0),
            Width = dimensions.MacroWidth,
            Height = dimensions.MacroHeight,
            ZBounds = new ZBounds(-1, 1),
            Parcels = []
        };

        var townCenterParcel = new SemanticParcel
        {
            Landcover = LandcoverType.Settlement,
            Parcel = ParcelType.TownCenter,
            ElevationMode = ElevationMode.Surface,
            Layers = [new SemanticLayer { Kind = SemanticLayerKind.Parcel }]
        };

        var localX = 5;
        var localY = 5;
        var key = new LocalChunkKey(macroChunk.Key, localX, localY, 0);

        macroChunk.SetParcel(localX, localY, 0, townCenterParcel);

        var realizer = new LocalRealizer();
        // Since random placement is involved, we might need multiple iterations to guarantee weapon/supplement spawning or we mock seed. 
        // But Realizer uses PhaseSeedDeriver which is deterministic based on seed + coordinates.
        // We will just test that it's possible or check a specific seed.
        // For town center, weapons have 25% chance, supplements 50% chance. We will check multiple coordinates to ensure they spawn.
        
        bool foundWeapon = false;
        bool foundSupplement = false;
        
        for (int i = 0; i < 16; i++)
        {
            var testKey = new LocalChunkKey(macroChunk.Key, i, i, 0);
            macroChunk.SetParcel(i, i, 0, townCenterParcel);
            var chunk = await realizer.RealizeAsync(testKey, macroChunk, profile);
            if (chunk.Items.Any(item => item.Type == ItemKind.Weapon)) foundWeapon = true;
            if (chunk.Items.Any(item => item.Type == ItemKind.Supplement)) foundSupplement = true;
        }

        Assert.True(foundWeapon, "Should have generated at least one weapon across 20 iterations.");
        Assert.True(foundSupplement, "Should have generated at least one supplement across 20 iterations.");
    }
}
