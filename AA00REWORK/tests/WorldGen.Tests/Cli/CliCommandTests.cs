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

    [Fact]
    public async Task Repo_relative_world_paths_resolve_from_nested_cli_directory()
    {
        using var directory = new TemporaryDirectory();
        var workspaceRoot = Path.Combine(directory.Path, "workspace");
        var nestedCliDirectory = Path.Combine(workspaceRoot, "AA00REWORK", "src", "WorldGen.Cli");
        Directory.CreateDirectory(nestedCliDirectory);
        Directory.CreateDirectory(Path.Combine(nestedCliDirectory, "AA00REWORK"));

        var previousCurrentDirectory = Directory.GetCurrentDirectory();
        Directory.SetCurrentDirectory(nestedCliDirectory);
        try
        {
            var handler = new CommandHandlers();
            using var writer = new StringWriter();
            var relativeWorldPath = Path.Combine("AA00REWORK", "out", "example_world_single");
            var expectedWorldPath = Path.Combine(workspaceRoot, relativeWorldPath);

            var generateExitCode = await handler.GenerateWorldAsync(654321UL, relativeWorldPath, radius: 1, writer, CancellationToken.None);
            var validateExitCode = await handler.ValidateDeterminismAsync(relativeWorldPath, startX: -1, startY: -1, width: 2, height: 2, writer, CancellationToken.None);

            Assert.Equal(0, generateExitCode);
            Assert.Equal(0, validateExitCode);
            Assert.True(File.Exists(Path.Combine(expectedWorldPath, "world", "world-profile.bin")));
        }
        finally
        {
            Directory.SetCurrentDirectory(previousCurrentDirectory);
        }
    }
}
