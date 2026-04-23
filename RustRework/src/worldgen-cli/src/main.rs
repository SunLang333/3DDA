use std::collections::BTreeMap;
use std::io::{self, Write};

use ctrlc::set_handler;
use tokio_util::sync::CancellationToken;
use worldgen_cli::CommandHandlers;
use worldgen_core::{Result, WorldGenError};

#[tokio::main]
async fn main() {
    let exit_code = match run().await {
        Ok(code) => code,
        Err(WorldGenError::Cancelled) => {
            let _ = writeln!(io::stderr(), "Operation canceled.");
            2
        }
        Err(error) => {
            let _ = writeln!(io::stderr(), "{error}");
            1
        }
    };

    std::process::exit(exit_code);
}

async fn run() -> Result<i32> {
    let handler = CommandHandlers::default();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = args.first().cloned().unwrap_or_default();
    let options = parse_options(&args[1..]);

    let token = CancellationToken::new();
    let token_for_signal = token.clone();
    set_handler(move || token_for_signal.cancel())
        .map_err(|error| WorldGenError::state(format!("failed to install Ctrl+C handler: {error}")))?;

    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();

    match command.as_str() {
        "generate-world" => handler
            .generate_world(
                parse_u64(&options, "seed")?,
                parse_required(&options, "out")?,
                parse_i32_with_default(&options, "radius", 1)?,
                &mut stdout,
                &token,
            )
            .await,
        "dump-macro-chunk" => handler
            .dump_macro_chunk(
                parse_required(&options, "world")?,
                parse_i32(&options, "x")?,
                parse_i32(&options, "y")?,
                parse_optional(&options, "export"),
                &mut stdout,
                &token,
            )
            .await,
        "dump-local-chunk" => handler
            .dump_local_chunk(
                parse_required(&options, "world")?,
                parse_i32(&options, "x")?,
                parse_i32(&options, "y")?,
                parse_i32(&options, "z")?,
                options.contains_key("full"),
                parse_optional(&options, "export"),
                &mut stdout,
                &token,
            )
            .await,
        "validate-determinism" => handler
            .validate_determinism(
                parse_required(&options, "world")?,
                parse_i32_with_default(&options, "start-x", 0)?,
                parse_i32_with_default(&options, "start-y", 0)?,
                parse_i32_with_default(&options, "width", 2)?,
                parse_i32_with_default(&options, "height", 2)?,
                &mut stdout,
                &token,
            )
            .await,
        "count-items" => handler
            .count_items(
                parse_required(&options, "world")?,
                options.contains_key("surface-only"),
                &mut stdout,
                &token,
            )
            .await,
        _ => show_usage(&mut stderr),
    }
}

fn parse_options(args: &[String]) -> BTreeMap<String, String> {
    let mut options = BTreeMap::new();
    let mut index = 0usize;
    while index < args.len() {
        let token = &args[index];
        if !token.starts_with("--") {
            index += 1;
            continue;
        }

        let key = token.trim_start_matches("--").to_string();
        let value = if index + 1 < args.len() && !args[index + 1].starts_with("--") {
            index += 1;
            args[index].clone()
        } else {
            "true".to_string()
        };
        options.insert(key, value);
        index += 1;
    }

    options
}

fn parse_required(options: &BTreeMap<String, String>, name: &str) -> Result<String> {
    options
        .get(name)
        .cloned()
        .ok_or_else(|| WorldGenError::missing_option(name))
}

fn parse_optional(options: &BTreeMap<String, String>, name: &str) -> Option<String> {
    options.get(name).cloned()
}

fn parse_i32(options: &BTreeMap<String, String>, name: &str) -> Result<i32> {
    let value = parse_required(options, name)?;
    value.parse::<i32>().map_err(|_| {
        WorldGenError::invalid_option(format!("Option --{name} must be a valid integer."))
    })
}

fn parse_i32_with_default(options: &BTreeMap<String, String>, name: &str, default: i32) -> Result<i32> {
    match options.get(name) {
        Some(value) => value.parse::<i32>().map_err(|_| {
            WorldGenError::invalid_option(format!("Option --{name} must be a valid integer."))
        }),
        None => Ok(default),
    }
}

fn parse_u64(options: &BTreeMap<String, String>, name: &str) -> Result<u64> {
    let value = parse_required(options, name)?;
    value.parse::<u64>().map_err(|_| {
        WorldGenError::invalid_option(format!("Option --{name} must be a valid unsigned integer."))
    })
}

fn show_usage(output: &mut impl Write) -> Result<i32> {
    writeln!(output, "Usage:")?;
    writeln!(output, "  generate-world --seed <ulong> --out <path> [--radius <int>]")?;
    writeln!(output, "  dump-macro-chunk --world <path> --x <int> --y <int> [--export <path>]")?;
    writeln!(output, "  dump-local-chunk --world <path> --x <int> --y <int> --z <int> [--export <path>]")?;
    writeln!(output, "  validate-determinism --world <path> [--start-x <int>] [--start-y <int>] [--width <int>] [--height <int>]")?;
    Ok(1)
}
