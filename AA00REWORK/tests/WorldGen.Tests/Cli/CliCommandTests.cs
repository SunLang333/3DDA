using WorldGen.Cli;
using WorldGen.Tests.TestSupport;

namespace WorldGen.Tests.Cli;

public sealed class CliCommandTests
{
    [Fact]
    public async Task Generate_world_and_validate_determinism_commands_succeed()
    {
        using var directory = new TemporaryDirectory();
        var handler = new CommandHandlers();
        using var writer = new StringWriter();

        var generateExitCode = await handler.GenerateWorldAsync(123456UL, directory.Path, radius: 1, writer, CancellationToken.None);
        var validateExitCode = await handler.ValidateDeterminismAsync(directory.Path, startX: -1, startY: -1, width: 2, height: 2, writer, CancellationToken.None);

        Assert.Equal(0, generateExitCode);
        Assert.Equal(0, validateExitCode);
        Assert.True(File.Exists(Path.Combine(directory.Path, "world-summary.json")));
        Assert.True(File.Exists(Path.Combine(directory.Path, "macro-0_0-summary.json")));
        Assert.True(File.Exists(Path.Combine(directory.Path, "local-0_0_0-summary.json")));
        Assert.Contains("Determinism validation passed.", writer.ToString(), StringComparison.Ordinal);
    }
}
