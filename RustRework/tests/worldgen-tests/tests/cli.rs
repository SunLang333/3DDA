use std::env;

mod rustrework_worldgen {
    pub use worldgen_cli::*;
}

use rustrework_worldgen::CommandHandlers;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn generate_world_and_validate_determinism_commands_succeed() {
    let directory = TempDir::new().unwrap();
    let handler = CommandHandlers::default();
    let mut writer = Vec::new();

    let generate_exit_code = handler
        .generate_world(
            123_456,
            directory.path().display().to_string(),
            1,
            &mut writer,
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    let validate_exit_code = handler
        .validate_determinism(
            directory.path().display().to_string(),
            -1,
            -1,
            2,
            2,
            &mut writer,
            &CancellationToken::new(),
        )
        .await
        .unwrap();

    assert_eq!(generate_exit_code, 0);
    assert_eq!(validate_exit_code, 0);
    assert!(directory.path().join("world-summary.json").exists());
    assert!(directory.path().join("macro-0_0-summary.json").exists());
    assert!(directory.path().join("local-0_0_0-summary.json").exists());
    let output = String::from_utf8(writer).unwrap();
    assert!(output.contains("Determinism validation passed."));
}

#[tokio::test]
async fn repo_relative_world_paths_resolve_from_nested_cli_directory() {
    let directory = TempDir::new().unwrap();
    let workspace_root = directory.path().join("workspace");
    let nested_cli_directory = workspace_root.join("AA00REWORK").join("src").join("WorldGen.Cli");
    std::fs::create_dir_all(nested_cli_directory.join("AA00REWORK")).unwrap();

    let previous_current_directory = env::current_dir().unwrap();
    env::set_current_dir(&nested_cli_directory).unwrap();
    async {
        let handler = CommandHandlers::default();
        let mut writer = Vec::new();
        let relative_world_path = std::path::Path::new("AA00REWORK")
            .join("out")
            .join("example_world_single");
        let expected_world_path = workspace_root.join(&relative_world_path);

        let generate_exit_code = handler
            .generate_world(
                654_321,
                relative_world_path.display().to_string(),
                1,
                &mut writer,
                &CancellationToken::new(),
            )
            .await
            .unwrap();
        let validate_exit_code = handler
            .validate_determinism(
                relative_world_path.display().to_string(),
                -1,
                -1,
                2,
                2,
                &mut writer,
                &CancellationToken::new(),
            )
            .await
            .unwrap();

        assert_eq!(generate_exit_code, 0);
        assert_eq!(validate_exit_code, 0);
        assert!(expected_world_path.join("world").join("world-profile.bin").exists());
    }
    .await;
    env::set_current_dir(previous_current_directory).unwrap();
}

#[tokio::test]
async fn count_items_command_succeeds_on_generated_world() {
    let directory = TempDir::new().unwrap();
    let handler = CommandHandlers::default();
    let mut writer = Vec::new();

    handler
        .generate_world(
            42,
            directory.path().display().to_string(),
            1,
            &mut writer,
            &CancellationToken::new(),
        )
        .await
        .unwrap();

    let exit_code = handler
        .count_items(
            directory.path().display().to_string(),
            true,
            &mut writer,
            &CancellationToken::new(),
        )
        .await
        .unwrap();

    assert_eq!(exit_code, 0);
    let output = String::from_utf8(writer).unwrap();
    assert!(output.contains("Total placed item entries:"));
    assert!(output.contains("Total item quantity:"));
}
