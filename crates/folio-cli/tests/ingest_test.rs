use assert_cmd::cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::fs;
use walkdir::WalkDir;

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

    // Assert: Verify files were copied to archive in date-based folders
    // sample-with-exif.jpg has EXIF timestamp 2024:11:04 14:02:15
    assert!(archive.path().join("2024/11/04/photo1.jpg").exists());
    // minimal.mov will use modified date - just verify it exists in archive somewhere
    let mut found_video = false;
    for entry in walkdir::WalkDir::new(archive.path())
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry.file_name().to_string_lossy() == "video1.mov" {
            found_video = true;
            break;
        }
    }
    assert!(found_video, "Video file should exist in archive");
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

    // Assert: Only photo copied in date folder, not text file
    // minimal.jpg will use modified date - verify it exists somewhere in archive
    let mut found_photo = false;
    for entry in WalkDir::new(archive.path())
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry.file_name().to_string_lossy() == "photo.jpg" {
            found_photo = true;
        }
        // Text file should never be copied
        assert_ne!(entry.file_name().to_string_lossy(), "readme.txt");
    }
    assert!(found_photo, "Photo should exist in archive");
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

#[test]
fn test_ingest_organizes_by_date() {
    // Arrange: Create test environment
    let source = assert_fs::TempDir::new().unwrap();
    let archive = assert_fs::TempDir::new().unwrap();

    let fixtures_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-data/fixtures");

    // Copy photo with EXIF timestamp: 2024-11-04 14:02:15
    fs::copy(
        fixtures_dir.join("sample-with-exif.jpg"),
        source.path().join("photo1.jpg"),
    )
    .unwrap();

    // Act: Run ingest
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("folio"));
    cmd.arg("ingest")
        .arg("--source")
        .arg(source.path())
        .arg("--dest")
        .arg(archive.path())
        .assert()
        .success();

    // Assert: File organized into YYYY/MM/DD/ folder structure
    // EXIF timestamp is 2024:11:04 14:02:15
    let expected_path = archive.path().join("2024/11/04/photo1.jpg");
    assert!(
        expected_path.exists(),
        "Expected file at {:?}",
        expected_path
    );
}
