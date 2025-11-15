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

    // Act: Run CLI command with --batch-name to avoid interactive prompts
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("folio"));
    cmd.arg("ingest")
        .arg("--source")
        .arg(source.path())
        .arg("--dest")
        .arg(archive.path())
        .arg("--batch-name")
        .arg("test-batch")
        .assert()
        .success()
        .stdout(predicate::str::contains("Found 2 media files"))
        .stdout(predicate::str::contains("temporal batching disabled"));

    // Assert: Verify files were copied to archive in date-based folders
    // sample-with-exif.jpg has EXIF timestamp 2024:11:04 14:02:15
    assert!(archive
        .path()
        .join("2024/11/04/20241104-140215-test-batch.jpg")
        .exists());
    // minimal.mov will use modified date - just verify it exists in archive somewhere
    let mut found_video = false;
    for entry in walkdir::WalkDir::new(archive.path())
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry
            .file_name()
            .to_string_lossy()
            .contains("test-batch.mov")
        {
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

    // Act: Use --batch-name to avoid interactive prompts
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("folio"));
    cmd.arg("ingest")
        .arg("--source")
        .arg(source.path())
        .arg("--dest")
        .arg(archive.path())
        .arg("--batch-name")
        .arg("test-batch")
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
        if entry
            .file_name()
            .to_string_lossy()
            .contains("test-batch.jpg")
        {
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

    // Act: Run ingest with --batch-name
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("folio"));
    cmd.arg("ingest")
        .arg("--source")
        .arg(source.path())
        .arg("--dest")
        .arg(archive.path())
        .arg("--batch-name")
        .arg("test-batch")
        .assert()
        .success();

    // Assert: File organized into YYYY/MM/DD/ folder structure
    // EXIF timestamp is 2024:11:04 14:02:15
    let expected_path = archive
        .path()
        .join("2024/11/04/20241104-140215-test-batch.jpg");
    assert!(
        expected_path.exists(),
        "Expected file at {:?}",
        expected_path
    );
}

#[test]
fn test_ingest_with_batch_name() {
    // Arrange: Create test environment with two photos from different times
    let source = assert_fs::TempDir::new().unwrap();
    let archive = assert_fs::TempDir::new().unwrap();

    let fixtures_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-data/fixtures");

    // Copy two photos with different EXIF timestamps
    // sample-with-exif.jpg: 2024:11:04 14:02:15
    // sample-different-time.jpg: 2024:11:04 18:15:30
    fs::copy(
        fixtures_dir.join("sample-with-exif.jpg"),
        source.path().join("photo1.jpg"),
    )
    .unwrap();

    fs::copy(
        fixtures_dir.join("sample-different-time.jpg"),
        source.path().join("photo2.jpg"),
    )
    .unwrap();

    // Act: Run ingest with --batch-name flag (single name for all files)
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("folio"));
    cmd.arg("ingest")
        .arg("--source")
        .arg(source.path())
        .arg("--dest")
        .arg(archive.path())
        .arg("--batch-name")
        .arg("test-event")
        .assert()
        .success();

    // Assert: Files renamed with YYYYMMDD-HHMMSS-{batch-name}.{ext} format
    // First photo: 2024-11-04 14:02:15 -> 20241104-140215-test-event.jpg
    // Second photo: 2024-11-04 18:15:30 -> 20241104-181530-test-event.jpg
    let expected_path1 = archive
        .path()
        .join("2024/11/04/20241104-140215-test-event.jpg");
    let expected_path2 = archive
        .path()
        .join("2024/11/04/20241104-181530-test-event.jpg");

    assert!(
        expected_path1.exists(),
        "Expected file at {:?}",
        expected_path1
    );
    assert!(
        expected_path2.exists(),
        "Expected file at {:?}",
        expected_path2
    );
}

#[test]
fn test_ingest_detects_temporal_batches_with_dry_run() {
    // Arrange: Create test environment with photos from two temporal batches
    let source = assert_fs::TempDir::new().unwrap();
    let archive = assert_fs::TempDir::new().unwrap();

    let fixtures_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-data/fixtures");

    // Copy photos with timestamps 4+ hours apart (> 2-hour default gap threshold)
    // Batch 1: sample-with-exif.jpg at 2024:11:04 14:02:15
    // Batch 2: sample-different-time.jpg at 2024:11:04 18:15:30 (4h 13m later)
    fs::copy(
        fixtures_dir.join("sample-with-exif.jpg"),
        source.path().join("photo1.jpg"),
    )
    .unwrap();

    fs::copy(
        fixtures_dir.join("sample-different-time.jpg"),
        source.path().join("photo2.jpg"),
    )
    .unwrap();

    // Act: Run ingest WITHOUT --batch-name (should detect batches)
    // Use --dry-run because interactive prompts not implemented yet
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("folio"));
    cmd.arg("ingest")
        .arg("--source")
        .arg(source.path())
        .arg("--dest")
        .arg(archive.path())
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Found 2 media files"))
        .stdout(predicate::str::contains("Detected 2 temporal batches"));

    // Verify: Temporal batching detected (would prompt for names in non-dry-run mode)
}

#[test]
fn test_ingest_batch_name_disables_temporal_batching() {
    // Arrange: Same setup as above - photos 4+ hours apart
    let source = assert_fs::TempDir::new().unwrap();
    let archive = assert_fs::TempDir::new().unwrap();

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

    fs::copy(
        fixtures_dir.join("sample-different-time.jpg"),
        source.path().join("photo2.jpg"),
    )
    .unwrap();

    // Act: Run ingest WITH --batch-name (should disable temporal batching)
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("folio"));
    cmd.arg("ingest")
        .arg("--source")
        .arg(source.path())
        .arg("--dest")
        .arg(archive.path())
        .arg("--batch-name")
        .arg("test-batch")
        .assert()
        .success()
        .stdout(predicate::str::contains("Found 2 media files"))
        .stdout(predicate::str::contains(
            "Using single batch name for all files (temporal batching disabled)",
        ))
        .stdout(predicate::str::contains("Detected 2 temporal batches").not());

    // Verify: Both files get same batch name, temporal batching was skipped
    let expected_path1 = archive
        .path()
        .join("2024/11/04/20241104-140215-test-batch.jpg");
    let expected_path2 = archive
        .path()
        .join("2024/11/04/20241104-181530-test-batch.jpg");

    assert!(
        expected_path1.exists(),
        "Expected file at {:?}",
        expected_path1
    );
    assert!(
        expected_path2.exists(),
        "Expected file at {:?}",
        expected_path2
    );
}

#[test]
fn test_ingest_custom_gap_threshold() {
    // Arrange: Photos 4+ hours apart
    let source = assert_fs::TempDir::new().unwrap();
    let archive = assert_fs::TempDir::new().unwrap();

    let fixtures_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-data/fixtures");

    // Copy photos with timestamps 4+ hours apart
    // Batch 1: sample-with-exif.jpg at 2024:11:04 14:02:15
    // Batch 2: sample-different-time.jpg at 2024:11:04 18:15:30 (4h 13m later)
    fs::copy(
        fixtures_dir.join("sample-with-exif.jpg"),
        source.path().join("photo1.jpg"),
    )
    .unwrap();

    fs::copy(
        fixtures_dir.join("sample-different-time.jpg"),
        source.path().join("photo2.jpg"),
    )
    .unwrap();

    // Act: Use 5.0 hour gap threshold (should be 1 batch since files are 4h 13m apart)
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("folio"));
    cmd.arg("ingest")
        .arg("--source")
        .arg(source.path())
        .arg("--dest")
        .arg(archive.path())
        .arg("--gap-threshold")
        .arg("5.0")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Found 2 media files"))
        .stdout(predicate::str::contains("Detected 1 temporal batch"))
        .stdout(predicate::str::contains("gap threshold: 5.0 hours"));

    // Verify: Custom gap threshold was used, files grouped into single batch
}

#[test]
fn test_ingest_validates_batch_name() {
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

    // Act: Try to use invalid batch name with spaces
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("folio"));
    cmd.arg("ingest")
        .arg("--source")
        .arg(source.path())
        .arg("--dest")
        .arg(archive.path())
        .arg("--batch-name")
        .arg("invalid batch name") // spaces not allowed
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid batch name"));

    // Act: Try to use invalid batch name with special characters
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("folio"));
    cmd.arg("ingest")
        .arg("--source")
        .arg(source.path())
        .arg("--dest")
        .arg(archive.path())
        .arg("--batch-name")
        .arg("vacation!")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid batch name"));

    // Act: Try to use empty batch name
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("folio"));
    cmd.arg("ingest")
        .arg("--source")
        .arg(source.path())
        .arg("--dest")
        .arg(archive.path())
        .arg("--batch-name")
        .arg("")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid batch name"));
}

#[test]
fn test_ingest_interactive_mode_with_valid_input() {
    // Arrange: Two photos from different temporal batches
    let source = assert_fs::TempDir::new().unwrap();
    let archive = assert_fs::TempDir::new().unwrap();

    let fixtures_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test-data/fixtures");

    // Copy photos with timestamps 4+ hours apart (> 2-hour default gap threshold)
    // Batch 1: sample-with-exif.jpg at 2024:11:04 14:02:15
    // Batch 2: sample-different-time.jpg at 2024:11:04 18:15:30 (4h 13m later)
    fs::copy(
        fixtures_dir.join("sample-with-exif.jpg"),
        source.path().join("photo1.jpg"),
    )
    .unwrap();

    fs::copy(
        fixtures_dir.join("sample-different-time.jpg"),
        source.path().join("photo2.jpg"),
    )
    .unwrap();

    // Act: Run ingest in interactive mode with mocked stdin
    // Provide batch names via stdin: "morning\nafternoon\n"
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("folio"));
    cmd.arg("ingest")
        .arg("--source")
        .arg(source.path())
        .arg("--dest")
        .arg(archive.path())
        .write_stdin("morning\nafternoon\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Detected 2 temporal batches"))
        .stdout(predicate::str::contains("Batch 1 of 2"))
        .stdout(predicate::str::contains("Batch 2 of 2"))
        .stdout(predicate::str::contains("Copied 2 files"));

    // Assert: Files renamed with batch-specific names
    let expected_path1 = archive
        .path()
        .join("2024/11/04/20241104-140215-morning.jpg");
    let expected_path2 = archive
        .path()
        .join("2024/11/04/20241104-181530-afternoon.jpg");

    assert!(
        expected_path1.exists(),
        "Expected file at {:?}",
        expected_path1
    );
    assert!(
        expected_path2.exists(),
        "Expected file at {:?}",
        expected_path2
    );
}

#[test]
fn test_ingest_interactive_mode_with_invalid_then_valid_input() {
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

    // Act: Provide invalid input first (with space), then valid input
    // stdin: "invalid name\nvalid-name\n"
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("folio"));
    cmd.arg("ingest")
        .arg("--source")
        .arg(source.path())
        .arg("--dest")
        .arg(archive.path())
        .write_stdin("invalid name\nvalid-name\n")
        .assert()
        .success()
        .stderr(predicate::str::contains("Invalid batch name"))
        .stdout(predicate::str::contains("Batch 1 of 1"))
        .stdout(predicate::str::contains("Copied 1 file"));

    // Assert: File renamed with valid batch name
    let mut found_file = false;
    for entry in WalkDir::new(archive.path())
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry
            .file_name()
            .to_string_lossy()
            .contains("valid-name.jpg")
        {
            found_file = true;
            break;
        }
    }
    assert!(found_file, "File with valid-name should exist in archive");
}
