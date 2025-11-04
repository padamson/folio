use assert_cmd::cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::fs;

#[test]
fn test_ingest_mixed_media() {
    // Arrange: Create test environment
    let source = assert_fs::TempDir::new().unwrap();
    let archive = assert_fs::TempDir::new().unwrap();

    // Copy fixtures to source directory
    let fixtures_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-data/fixtures");

    // Copy a photo and a video from fixtures
    fs::copy(
        fixtures_dir.join("sample-with-exif.jpg"),
        source.path().join("photo1.jpg"),
    )
    .unwrap();

    fs::copy(
        fixtures_dir.join("minimal.mov"),
        source.path().join("video1.mov"),
    )
    .unwrap();

    // Act: Run CLI command
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("folio"));
    cmd.arg("ingest")
        .arg("--source")
        .arg(source.path())
        .arg("--dest")
        .arg(archive.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Found 2 media files"));

    // Assert: Verify files were copied to archive
    assert!(archive.child("photo1.jpg").exists());
    assert!(archive.child("video1.mov").exists());
}

#[test]
fn test_ingest_filters_non_media() {
    // Arrange
    let source = assert_fs::TempDir::new().unwrap();
    let archive = assert_fs::TempDir::new().unwrap();

    let fixtures_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-data/fixtures");

    // Copy one media file and one text file
    fs::copy(
        fixtures_dir.join("minimal.jpg"),
        source.path().join("photo.jpg"),
    )
    .unwrap();

    fs::copy(
        fixtures_dir.join("test.txt"),
        source.path().join("readme.txt"),
    )
    .unwrap();

    // Act
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("folio"));
    cmd.arg("ingest")
        .arg("--source")
        .arg(source.path())
        .arg("--dest")
        .arg(archive.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Found 1 media file"));

    // Assert: Only photo copied, not text file
    assert!(archive.child("photo.jpg").exists());
    assert!(!archive.child("readme.txt").exists());
}

#[test]
fn test_ingest_dry_run() {
    // Arrange
    let source = assert_fs::TempDir::new().unwrap();
    let archive = assert_fs::TempDir::new().unwrap();

    let fixtures_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-data/fixtures");

    fs::copy(
        fixtures_dir.join("minimal.jpg"),
        source.path().join("photo.jpg"),
    )
    .unwrap();

    // Act: Run with --dry-run flag
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("folio"));
    cmd.arg("ingest")
        .arg("--source")
        .arg(source.path())
        .arg("--dest")
        .arg(archive.path())
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry run"))
        .stdout(predicate::str::contains("Found 1 media file"));

    // Assert: Files NOT actually copied in dry run
    assert!(!archive.child("photo.jpg").exists());
}
