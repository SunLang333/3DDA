using WorldGen.Cli;

var handler = new CommandHandlers();
var command = args.Length > 0 ? args[0] : string.Empty;
var options = ParseOptions(args.Skip(1).ToArray());
using var cts = new CancellationTokenSource();
Console.CancelKeyPress += (_, eventArgs) =>
{
	eventArgs.Cancel = true;
	cts.Cancel();
};

try
{
	var exitCode = command switch
	{
		"generate-world" => await handler.GenerateWorldAsync(
			seed: ParseUInt64(options, "seed"),
			outputPath: ParseRequired(options, "out"),
			radius: ParseInt32(options, "radius", defaultValue: 1),
			output: Console.Out,
			ct: cts.Token),
		"dump-macro-chunk" => await handler.DumpMacroChunkAsync(
			worldPath: ParseRequired(options, "world"),
			x: ParseInt32(options, "x"),
			y: ParseInt32(options, "y"),
			exportPath: ParseOptional(options, "export"),
			output: Console.Out,
			ct: cts.Token),
		"dump-local-chunk" => await handler.DumpLocalChunkAsync(
				worldPath: ParseRequired(options, "world"),
				x: ParseInt32(options, "x"),
				y: ParseInt32(options, "y"),
				z: ParseInt32(options, "z"),
				full: options.ContainsKey("full"),
				exportPath: ParseOptional(options, "export"),
				output: Console.Out,
				ct: cts.Token),
		"validate-determinism" => await handler.ValidateDeterminismAsync(
			worldPath: ParseRequired(options, "world"),
			startX: ParseInt32(options, "start-x", 0),
			startY: ParseInt32(options, "start-y", 0),
			width: ParseInt32(options, "width", 2),
			height: ParseInt32(options, "height", 2),
			output: Console.Out,
			ct: cts.Token),
		"count-items" => await handler.CountItemsAsync(
				worldPath: ParseRequired(options, "world"),
				surfaceOnly: options.ContainsKey("surface-only"),
				output: Console.Out,
				ct: cts.Token),
		_ => ShowUsage(Console.Error)
	};

	return exitCode;
}
catch (OperationCanceledException)
{
	await Console.Error.WriteLineAsync("Operation canceled.");
	return 2;
}
catch (Exception ex)
{
	await Console.Error.WriteLineAsync(ex.Message);
	return 1;
}

static Dictionary<string, string> ParseOptions(string[] args)
{
	var options = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
	for (var index = 0; index < args.Length; index++)
	{
		var token = args[index];
		if (!token.StartsWith("--", StringComparison.Ordinal))
		{
			continue;
		}

		var key = token[2..];
		var value = index + 1 < args.Length && !args[index + 1].StartsWith("--", StringComparison.Ordinal)
			? args[++index]
			: "true";
		options[key] = value;
	}

	return options;
}

static string ParseRequired(IReadOnlyDictionary<string, string> options, string name)
	=> options.TryGetValue(name, out var value)
		? value
		: throw new InvalidOperationException($"Missing required option --{name}.");

static string? ParseOptional(IReadOnlyDictionary<string, string> options, string name)
	=> options.TryGetValue(name, out var value) ? value : null;

static int ParseInt32(IReadOnlyDictionary<string, string> options, string name, int? defaultValue = null)
{
	if (!options.TryGetValue(name, out var value))
	{
		return defaultValue ?? throw new InvalidOperationException($"Missing required option --{name}.");
	}

	return int.TryParse(value, out var parsed)
		? parsed
		: throw new InvalidOperationException($"Option --{name} must be a valid integer.");
}

static ulong ParseUInt64(IReadOnlyDictionary<string, string> options, string name)
{
	var value = ParseRequired(options, name);
	return ulong.TryParse(value, out var parsed)
		? parsed
		: throw new InvalidOperationException($"Option --{name} must be a valid unsigned integer.");
}

static int ShowUsage(TextWriter output)
{
	output.WriteLine("Usage:");
	output.WriteLine("  generate-world --seed <ulong> --out <path> [--radius <int>]");
	output.WriteLine("  dump-macro-chunk --world <path> --x <int> --y <int> [--export <path>]");
	output.WriteLine("  dump-local-chunk --world <path> --x <int> --y <int> --z <int> [--export <path>]");
	output.WriteLine("  validate-determinism --world <path> [--start-x <int>] [--start-y <int>] [--width <int>] [--height <int>]");
	return 1;
}
