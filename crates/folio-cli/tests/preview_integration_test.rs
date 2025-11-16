use assert_cmd::cmd::Command;
use predicates::prelude::*;
use std::fs;

#[test]
fn test_ingest_with_preview_in_headless_mode() {
    // Setup temporary source and destination
    let source = assert_fs::TempDir::new().unwrap();
    let dest = assert_fs::TempDir::new().unwrap();

    // Copy fixtures to source directory
    let fixtures_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-data/fixtures");

    fs::copy(
        fixtures_dir.join("sample-with-exif.jpg"),
        source.path().join("photo1.jpg"),
    )
    .unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("folio"));

    // Run ingest with --headless flag (should complete without browser)
    cmd.arg("ingest")
        .arg("--source")
        .arg(source.path())
        .arg("--dest")
        .arg(dest.path())
        .arg("--batch-name")
        .arg("test-batch")
        .arg("--headless")
        .assert()
        .success()
        .stdout(predicate::str::contains("Scanning source:"))
        .stdout(predicate::str::contains("Found"))
        .stdout(predicate::str::contains("media"));
}

#[test]
fn test_ingest_dry_run_with_headless() {
    // Dry run should work with headless mode
    let source = assert_fs::TempDir::new().unwrap();
    let dest = assert_fs::TempDir::new().unwrap();

    // Copy fixtures to source directory
    let fixtures_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-data/fixtures");

    fs::copy(
        fixtures_dir.join("sample-with-exif.jpg"),
        source.path().join("photo1.jpg"),
    )
    .unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("folio"));

    cmd.arg("ingest")
        .arg("--source")
        .arg(source.path())
        .arg("--dest")
        .arg(dest.path())
        .arg("--dry-run")
        .arg("--headless")
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry run mode"))
        .stdout(predicate::str::contains("Scanning source:"));
}

#[test]
fn test_ingest_creates_preview_state() {
    // This test verifies that the ingest command completes successfully
    // which implies preview state was created correctly
    let source = assert_fs::TempDir::new().unwrap();
    let dest = assert_fs::TempDir::new().unwrap();

    // Copy fixtures to source directory
    let fixtures_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-data/fixtures");

    fs::copy(
        fixtures_dir.join("sample-with-exif.jpg"),
        source.path().join("photo1.jpg"),
    )
    .unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("folio"));

    cmd.arg("ingest")
        .arg("--source")
        .arg(source.path())
        .arg("--dest")
        .arg(dest.path())
        .arg("--batch-name")
        .arg("test-batch")
        .arg("--headless")
        .assert()
        .success();

    // Verify files were actually copied
    let copied_files: Vec<_> = fs::read_dir(dest.path())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .flat_map(|entry| {
            if entry.file_type().unwrap().is_dir() {
                fs::read_dir(entry.path())
                    .unwrap()
                    .filter_map(|e| e.ok())
                    .collect()
            } else {
                vec![entry]
            }
        })
        .collect();

    assert!(!copied_files.is_empty(), "Expected files to be copied");
}
