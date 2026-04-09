using WorldGen.Core.Domain;
using WorldGen.Core.Randomization;

namespace WorldGen.Tests.Determinism;

public sealed class SeedDerivationTests
{
    [Fact]
    public void Phase_seed_derivation_is_stable_and_phase_partitioned()
    {
        var macroKey = new MacroChunkKey(12, -8);
        var localKey = new LocalChunkKey(7, -3, 1);

        var macroHydrologyA = PhaseSeedDeriver.Hash64(123456789UL, macroKey, "macro:hydrology");
        var macroHydrologyB = PhaseSeedDeriver.Hash64(123456789UL, macroKey, "macro:hydrology");
        var macroSettlement = PhaseSeedDeriver.Hash64(123456789UL, macroKey, "macro:settlement");
        var localRealizationA = PhaseSeedDeriver.Hash64(123456789UL, localKey, "local:realization");
        var localRealizationB = PhaseSeedDeriver.Hash64(123456789UL, localKey, "local:realization");

        Assert.Equal(macroHydrologyA, macroHydrologyB);
        Assert.Equal(localRealizationA, localRealizationB);
        Assert.NotEqual(macroHydrologyA, macroSettlement);
        Assert.NotEqual(macroHydrologyA, localRealizationA);
        Assert.NotEqual(0UL, macroHydrologyA);
        Assert.NotEqual(0UL, localRealizationA);
    }
}
