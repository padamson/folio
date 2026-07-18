//! Integration tests for generate-examples binary

use assert_cmd::cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;

/// Write a minimal workspace `Cargo.toml` into the temp dir.
///
/// The binaries resolve `example-data/` relative to the workspace root
/// (walking up from the current directory), so each test sandbox must look
/// like a workspace or the binary errors out before reaching the behavior
/// under test.
fn write_stub_workspace(temp: &assert_fs::TempDir) {
    temp.child("Cargo.toml")
        .write_str("[workspace]\n\n[workspace.package]\nversion = \"0.1.0\"\n")
        .unwrap();
}

#[test]
fn test_generate_examples_help() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("generate-examples"));

    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Generate AI-powered example data"))
        .stdout(predicate::str::contains("--force"))
        .stdout(predicate::str::contains("--count"));
}

#[test]
fn test_generate_examples_skips_if_already_exists() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_stub_workspace(&temp);

    // Create mock example-data directory
    temp.child("example-data").create_dir_all().unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("generate-examples"));
    cmd.current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("already exists"))
        .stdout(predicate::str::contains("Use --force"));
}

#[test]
fn test_generate_examples_requires_api_key() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_stub_workspace(&temp);

    // Ensure the OpenAI backend is selected and no key is present, regardless
    // of what the developer's shell exports
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("generate-examples"));
    cmd.current_dir(temp.path())
        .env_remove("IMAGE_BACKEND")
        .env_remove("OPENAI_API_KEY")
        .assert()
        .failure()
        .stderr(predicate::str::contains("OPENAI_API_KEY"));
}

#[test]
#[ignore] // Expensive: Requires OpenAI API key and generates actual images
fn test_generate_examples_creates_directory_structure() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_stub_workspace(&temp);

    // This test requires a valid OPENAI_API_KEY in environment
    // Run with: cargo test --test generate_examples_test -- --ignored --nocapture

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("generate-examples"));

    cmd.current_dir(temp.path())
        .arg("--count")
        .arg("2") // Generate only 2 photos to save API costs
        .assert()
        .success()
        .stdout(predicate::str::contains("Generating"))
        .stdout(predicate::str::contains(
            "Example data generated successfully",
        ));

    // Verify directory structure
    temp.child("example-data")
        .assert(predicates::path::is_dir());
    temp.child("example-data/sd-card-thanksgiving")
        .assert(predicates::path::is_dir());
    temp.child("example-data/sd-card-thanksgiving/DCIM")
        .assert(predicates::path::is_dir());
    temp.child("example-data/sd-card-thanksgiving/DCIM/100NIKON")
        .assert(predicates::path::is_dir());
    temp.child("example-data/archive")
        .assert(predicates::path::is_dir());
    temp.child("example-data/archive-with-existing")
        .assert(predicates::path::is_dir());

    // Verify photos were created
    temp.child("example-data/sd-card-thanksgiving/DCIM/100NIKON/DSC_0001.JPG")
        .assert(predicates::path::is_file());
    temp.child("example-data/sd-card-thanksgiving/DCIM/100NIKON/DSC_0003.JPG")
        .assert(predicates::path::is_file());
}

#[test]
#[ignore] // Expensive: Requires OpenAI API key
fn test_generate_examples_force_regeneration() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_stub_workspace(&temp);

    // Create mock example-data directory
    temp.child("example-data").create_dir_all().unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("generate-examples"));
    cmd.current_dir(temp.path())
        .arg("--force")
        .arg("--count")
        .arg("2")
        .assert()
        .success()
        .stdout(predicate::str::contains("Generating"));
}

#[test]
#[ignore] // Expensive: Creates videos, requires ffmpeg
fn test_generate_examples_creates_videos() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_stub_workspace(&temp);

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("generate-examples"));

    cmd.current_dir(temp.path())
        .arg("--count")
        .arg("60") // Need at least 60 photos for 2 videos
        .assert()
        .success();

    // Verify videos were created
    temp.child("example-data/sd-card-thanksgiving/DCIM/100NIKON/DSC_0051.MOV")
        .assert(predicates::path::is_file());
    temp.child("example-data/sd-card-thanksgiving/DCIM/100NIKON/DSC_0102.MOV")
        .assert(predicates::path::is_file());
}

#[test]
#[ignore] // Expensive: Requires OpenAI API key
fn test_generate_examples_custom_count() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_stub_workspace(&temp);

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("generate-examples"));

    cmd.current_dir(temp.path())
        .arg("--count")
        .arg("5")
        .assert()
        .success()
        .stdout(predicate::str::contains("Generating 5 AI photos"));
}

// Backend detection tests
//
// Note: backend detection happens only after the example-data existence
// check, so meaningful tests here must NOT pre-create example-data/ — they
// drive the binary far enough to construct the backend config and fail on
// its validation.

#[test]
fn test_backend_detection_invalid() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_stub_workspace(&temp);

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("generate-examples"));
    cmd.current_dir(temp.path())
        .env("IMAGE_BACKEND", "invalid")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid IMAGE_BACKEND"));
}

#[test]
fn test_openai_backend_requires_api_key() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_stub_workspace(&temp);

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("generate-examples"));
    cmd.current_dir(temp.path())
        .env("IMAGE_BACKEND", "openai")
        .env_remove("OPENAI_API_KEY")
        .assert()
        .failure()
        .stderr(predicate::str::contains("OPENAI_API_KEY"));
}

#[test]
#[ignore] // Tries to connect to ComfyUI (expects failure, but still attempts connection)
fn test_local_backend_uses_default_url() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_stub_workspace(&temp);

    // Do NOT create example-data directory - we want to see the config message
    // But we'll fail because ComfyUI won't be running - that's okay
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("generate-examples"));
    cmd.current_dir(temp.path())
        .env("IMAGE_BACKEND", "local")
        .env_remove("LOCAL_SD_URL") // Should default to localhost:8188
        .env_remove("OPENAI_API_KEY") // Make sure OpenAI isn't used
        .assert()
        .failure() // Will fail because ComfyUI not running, but we'll see the message
        .stdout(predicate::str::contains("localhost:8188"));
}

// Local SD integration tests (requires ComfyUI running)

#[test]
#[ignore] // Requires ComfyUI server running at localhost:8188
fn test_generate_with_local_sd() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_stub_workspace(&temp);

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("generate-examples"));

    cmd.current_dir(temp.path())
        .env("IMAGE_BACKEND", "local")
        .env("LOCAL_SD_URL", "http://localhost:8188")
        .arg("--count")
        .arg("2") // Generate only 2 photos
        .assert()
        .success()
        .stdout(predicate::str::contains("Using local Stable Diffusion"))
        .stdout(predicate::str::contains(
            "Example data generated successfully",
        ));

    // Verify photos were created
    temp.child("example-data/sd-card-thanksgiving/DCIM/100NIKON/DSC_0001.JPG")
        .assert(predicates::path::is_file());
    temp.child("example-data/sd-card-thanksgiving/DCIM/100NIKON/DSC_0003.JPG")
        .assert(predicates::path::is_file());
}

#[test]
#[ignore] // Requires ComfyUI server
fn test_local_sd_connection_error() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_stub_workspace(&temp);

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("generate-examples"));

    cmd.current_dir(temp.path())
        .env("IMAGE_BACKEND", "local")
        .env("LOCAL_SD_URL", "http://localhost:9999") // Invalid port
        .arg("--count")
        .arg("2")
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Failed to connect")
                .or(predicate::str::contains("Connection refused")),
        );
}
