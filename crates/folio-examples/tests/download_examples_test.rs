//! Integration tests for download-examples binary

use assert_cmd::cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;

// NOTE: Actual download tests are skipped because GitHub releases don't exist yet.
// These will be enabled once the folio-example-data repo is set up with releases.

/// Write a minimal workspace `Cargo.toml` into the temp dir.
///
/// The binary resolves `example-data/` (and the release version) relative to
/// the workspace root, walking up from the current directory, so each test
/// sandbox must look like a workspace.
fn write_stub_workspace(temp: &assert_fs::TempDir) {
    temp.child("Cargo.toml")
        .write_str("[workspace]\n\n[workspace.package]\nversion = \"0.1.0\"\n")
        .unwrap();
}

#[test]
fn test_download_examples_help() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("download-examples"));

    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Download pre-generated example data",
        ))
        .stdout(predicate::str::contains("--force"));
}

#[test]
fn test_download_examples_skips_if_already_exists() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_stub_workspace(&temp);

    // Create mock example-data directory
    temp.child("example-data").create_dir_all().unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("download-examples"));
    cmd.current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("already exists"))
        .stdout(predicate::str::contains("Use --force"));
}

#[test]
#[ignore] // Requires actual GitHub release to exist
fn test_download_examples_creates_directory_structure() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_stub_workspace(&temp);

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("download-examples"));

    // Run download-examples with custom target directory
    cmd.current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Downloading example data"))
        .stdout(predicate::str::contains("version"));

    // Verify directory structure exists
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
}

#[test]
#[ignore] // Requires actual GitHub release to exist
fn test_download_examples_validates_structure() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_stub_workspace(&temp);

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("download-examples"));

    cmd.current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Validating structure"));
}

#[test]
#[ignore] // Requires actual GitHub release to exist
fn test_download_examples_shows_version() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_stub_workspace(&temp);

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("download-examples"));

    // The stub workspace pins the version this test observes
    cmd.current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("0.1.0"));
}

#[test]
#[ignore] // Requires actual GitHub release to exist
fn test_download_examples_force_redownload() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_stub_workspace(&temp);

    // First download
    let mut cmd1 = Command::new(assert_cmd::cargo::cargo_bin!("download-examples"));
    cmd1.current_dir(temp.path()).assert().success();

    // Force re-download
    let mut cmd2 = Command::new(assert_cmd::cargo::cargo_bin!("download-examples"));
    cmd2.current_dir(temp.path())
        .arg("--force")
        .assert()
        .success()
        .stdout(predicate::str::contains("Downloading example data"));
}
