# Test Data Strategy for Folio

## Overview

This document defines how we handle test data for the Folio project, especially media files (photos and videos), to support Test-Driven Development while protecting family privacy.

---

## Test Data Categories

### 1. **Unit Test Fixtures** (Committed to Git)
Small, synthetic test files used for unit tests.

**Location:** `test-data/fixtures/`

**Contents:**
- Minimal valid JPEG files (synthetic, no real photos)
- Minimal valid video files (MOV, MP4)
- Files with specific EXIF data for testing
- Corrupted files for error handling tests
- Files without metadata for fallback tests

**Size Limit:** <100KB per file, <5MB total

**How to Generate:**
```bash
# Create minimal valid JPEG (1x1 pixel)
convert -size 1x1 xc:white test-data/fixtures/minimal.jpg

# Add EXIF data with exiftool
exiftool -DateTimeOriginal="2024:11:04 14:02:15" \
         -Make="Nikon" \
         -Model="D800" \
         test-data/fixtures/sample-with-exif.jpg

# Create minimal MOV (1 frame, 1 second)
ffmpeg -f lavfi -i color=c=blue:s=320x240:d=1 \
       -f lavfi -i sine=frequency=1000:duration=1 \
       test-data/fixtures/minimal.mov
```

**Git Status:** ✅ **Committed** (safe, synthetic, small)

---

### 2. **Integration Test Data** (Generated on Demand)
Larger, realistic test files generated during test runs.

**Location:** `test-data/generated/` (gitignored)

**Contents:**
- Larger JPEG files (1-5 MB) for performance testing
- Multiple video files for batch testing
- Generated with realistic EXIF timestamps
- Generated during `cargo test` setup

**How to Generate:**
```rust
// In test setup
use image::{RgbImage, ImageBuffer};

fn generate_test_jpeg(width: u32, height: u32, path: &Path) -> Result<()> {
    let img: RgbImage = ImageBuffer::new(width, height);
    img.save(path)?;
    // Add EXIF with exiftool or kamadak-exif
    Ok(())
}
```

**Git Status:** ❌ **Gitignored** (generated, potentially large)

---

### 3. **Manual Test Data** (Never Committed)
Real or realistic media files for manual testing and development.

**Location:** `test-data/manual/` (gitignored, strict)

**Contents:**
- Downloaded CC0/public domain photos (replace real family photos)
- Sample D800 JPEG files from public sources
- Sample video clips for development testing
- Real SD card dumps (anonymized)

**How to Obtain:**
- **Option 1:** Download public domain photos from:
  - Unsplash (https://unsplash.com/license)
  - Pexels (https://www.pexels.com/license/)
  - Wikimedia Commons (CC0)
- **Option 2:** Export non-identifiable photos from your archive
  - Landscapes, objects, no people
  - Strip GPS data with exiftool
- **Option 3:** Use `exiftool` to add D800 EXIF to public photos

**Git Status:** ❌ **NEVER COMMITTED** (potentially identifiable, large)

**Safety Rule:** Assume any file in `test-data/manual/` could be accidentally committed. Never put real family photos here.

---

### 4. **Real Backlog Data** (Never in Git, Never in Project Directory)
Actual family photos/videos from SD cards for final validation.

**Location:** Outside project directory entirely (e.g., `/tmp/folio-validation/`)

**Usage:** Final validation only, never during development

**Git Status:** ❌ **NEVER IN PROJECT** (family privacy critical)

---

## .gitignore Configuration

Already configured in `.gitignore`:

```gitignore
# Test data - NEVER commit real family media
test-data/personal/
test-data/samples/
test-data/manual/
test-data/generated/
*.personal.*
*.family.*

# Only test-data/fixtures/ is safe to commit
```

**Additional safety:**
```bash
# Add pre-commit hook to block accidental commits
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash
if git diff --cached --name-only | grep -E "test-data/(manual|personal|samples|generated)"; then
  echo "ERROR: Attempting to commit test data from restricted directories!"
  exit 1
fi
EOF
chmod +x .git/hooks/pre-commit
```

---

## Test-Driven Development Workflow

### Outside-In TDD Process

**For each slice, follow this workflow:**

#### 1. **Write Integration Test First** (Outside-In)

Start with the user-facing behavior (CLI integration test):

```rust
// crates/folio-cli/tests/ingest_test.rs
use assert_cmd::Command;
use assert_fs::prelude::*;

#[test]
fn test_ingest_mixed_media() {
    // Arrange: Create test environment
    let source = assert_fs::TempDir::new().unwrap();
    let archive = assert_fs::TempDir::new().unwrap();

    // Copy fixtures to source directory
    source.child("photo1.jpg").write_binary(
        include_bytes!("../../test-data/fixtures/sample-with-exif.jpg")
    ).unwrap();
    source.child("video1.mov").write_binary(
        include_bytes!("../../test-data/fixtures/minimal.mov")
    ).unwrap();

    // Act: Run CLI command
    let mut cmd = Command::cargo_bin("folio").unwrap();
    cmd.arg("ingest")
        .arg("--source").arg(source.path())
        .arg("--archive").arg(archive.path());

    // Assert: Verify behavior
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("Ingested 2 media files"));

    // Assert: Verify files copied
    assert!(archive.child("photo1.jpg").exists());
    assert!(archive.child("video1.mov").exists());
}
```

**Run it:** `cargo test test_ingest_mixed_media`

**Expected:** ❌ **FAILS** - CLI command not implemented yet

---

#### 2. **Implement Minimal CLI Plumbing**

Just enough to make the test compile and run:

```rust
// crates/folio-cli/src/main.rs
match cli.command {
    Commands::Ingest { source, archive } => {
        println!("Ingesting from {} to {}", source, archive);
        // TODO: Call folio-core library
        Ok(())
    }
}
```

**Run it:** `cargo test test_ingest_mixed_media`

**Expected:** ❌ **FAILS** - Output doesn't match, files not copied

---

#### 3. **Write Library Unit Tests** (Inside-Out)

Now drop down to the library level and write unit tests for core functions:

```rust
// crates/folio-core/src/media.rs
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_detect_media_type_jpeg() {
        let path = PathBuf::from("test.jpg");
        let media_type = detect_media_type(&path);

        assert_eq!(
            media_type,
            Some(MediaType::Photo(PhotoFormat::Jpeg))
        );
    }

    #[test]
    fn test_detect_media_type_mov() {
        let path = PathBuf::from("test.MOV");
        let media_type = detect_media_type(&path);

        assert_eq!(
            media_type,
            Some(MediaType::Video(VideoFormat::Mov))
        );
    }

    #[test]
    fn test_scan_directory() {
        let test_dir = PathBuf::from("../../test-data/fixtures");
        let items = scan_directory(&test_dir).unwrap();

        // Should find sample-with-exif.jpg and minimal.mov
        assert!(items.len() >= 2);
        assert!(items.iter().any(|i| i.media_type.is_photo()));
        assert!(items.iter().any(|i| i.media_type.is_video()));
    }
}
```

**Run it:** `cargo test -p folio-core`

**Expected:** ❌ **FAILS** - Functions not implemented

---

#### 4. **Implement Core Library Functions**

Implement just enough to make unit tests pass:

```rust
// crates/folio-core/src/media.rs
pub fn detect_media_type(path: &Path) -> Option<MediaType> {
    match path.extension()?.to_str()? {
        "jpg" | "JPG" | "jpeg" | "JPEG" => {
            Some(MediaType::Photo(PhotoFormat::Jpeg))
        }
        "mov" | "MOV" => {
            Some(MediaType::Video(VideoFormat::Mov))
        }
        "mp4" | "MP4" => {
            Some(MediaType::Video(VideoFormat::Mp4))
        }
        _ => None,
    }
}

pub fn scan_directory(path: &Path) -> Result<Vec<MediaItem>> {
    let mut items = Vec::new();

    for entry in walkdir::WalkDir::new(path) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        if let Some(media_type) = detect_media_type(path) {
            items.push(MediaItem {
                path: path.to_path_buf(),
                media_type,
                // ... other fields
            });
        }
    }

    Ok(items)
}
```

**Run it:** `cargo test -p folio-core`

**Expected:** ✅ **PASSES** - Core functions work

---

#### 5. **Wire Up CLI to Library**

Connect the CLI to the library functions:

```rust
// crates/folio-cli/src/main.rs
use folio_core::{scan_directory, MediaType};

match cli.command {
    Commands::Ingest { source, archive } => {
        let source_items = scan_directory(&source)?;
        let archive_items = scan_directory(&archive)?;

        // TODO: Deduplication, copying

        let photo_count = source_items.iter()
            .filter(|i| matches!(i.media_type, MediaType::Photo(_)))
            .count();
        let video_count = source_items.iter()
            .filter(|i| matches!(i.media_type, MediaType::Video(_)))
            .count();

        println!("Ingested {} media files ({} photos, {} videos)",
            source_items.len(), photo_count, video_count);

        Ok(())
    }
}
```

**Run it:** `cargo test test_ingest_mixed_media`

**Expected:** ✅ **PASSES** - Integration test passes!

---

#### 6. **Refactor** (Green → Better Green)

Now that tests are green, refactor for clarity:

```rust
// Extract to separate module
mod ingest;

// Better error handling
use anyhow::Context;

// Better structure
struct IngestOperation {
    source: PathBuf,
    archive: PathBuf,
}

impl IngestOperation {
    fn run(&self) -> Result<IngestResult> {
        // ...
    }
}
```

**Run tests frequently:** `cargo test --workspace`

**Expected:** ✅ **STILL PASSES** - Refactoring didn't break anything

---

## Test Data Management Commands

### Setup Test Fixtures (One-Time)

```bash
# Run this once to set up minimal test fixtures
./scripts/setup-test-fixtures.sh
```

Create `scripts/setup-test-fixtures.sh`:
```bash
#!/bin/bash
set -e

FIXTURES_DIR="test-data/fixtures"
mkdir -p "$FIXTURES_DIR"

echo "Creating minimal test fixtures..."

# Minimal 1x1 JPEG (white pixel)
convert -size 1x1 xc:white "$FIXTURES_DIR/minimal.jpg"

# JPEG with EXIF data
convert -size 100x100 xc:blue "$FIXTURES_DIR/sample-with-exif.jpg"
exiftool -overwrite_original \
    -DateTimeOriginal="2024:11:04 14:02:15" \
    -Make="Nikon" \
    -Model="D800" \
    -GPSLatitude=40.7128 \
    -GPSLongitude=-74.0060 \
    "$FIXTURES_DIR/sample-with-exif.jpg"

# JPEG without EXIF
convert -size 50x50 xc:red "$FIXTURES_DIR/no-exif.jpg"

# Minimal 1-second video
ffmpeg -f lavfi -i color=c=blue:s=320x240:d=1 \
       -f lavfi -i sine=frequency=1000:duration=1 \
       -y "$FIXTURES_DIR/minimal.mov"

# Another video with different timestamp
ffmpeg -f lavfi -i color=c=green:s=320x240:d=1 \
       -f lavfi -i sine=frequency=1000:duration=1 \
       -y "$FIXTURES_DIR/minimal2.mov"

# Corrupted JPEG (truncated)
head -c 100 "$FIXTURES_DIR/minimal.jpg" > "$FIXTURES_DIR/corrupted.jpg"

echo "✓ Test fixtures created in $FIXTURES_DIR"
ls -lh "$FIXTURES_DIR"
```

### Download Manual Test Data

Create `scripts/download-test-media.sh`:
```bash
#!/bin/bash
set -e

MANUAL_DIR="test-data/manual"
mkdir -p "$MANUAL_DIR"

echo "Downloading public domain test media..."

# Download CC0 photos from Unsplash
curl -L "https://unsplash.com/photos/random/download?w=1920" \
    -o "$MANUAL_DIR/landscape-1.jpg"

curl -L "https://unsplash.com/photos/random/download?w=1920" \
    -o "$MANUAL_DIR/landscape-2.jpg"

# Add realistic D800 EXIF
exiftool -overwrite_original \
    -Make="Nikon" \
    -Model="D800" \
    -DateTimeOriginal="2024:11:04 14:30:00" \
    "$MANUAL_DIR/landscape-1.jpg"

exiftool -overwrite_original \
    -Make="Nikon" \
    -Model="D800" \
    -DateTimeOriginal="2024:11:04 18:45:00" \
    "$MANUAL_DIR/landscape-2.jpg"

echo "✓ Manual test data downloaded to $MANUAL_DIR"
echo "⚠️  This directory is gitignored - safe for development"
```

---

## Property-Based Testing with Generated Media

For complex scenarios, use `proptest` to generate test cases:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_filename_generation_never_panics(
        year in 2000u32..=2030u32,
        month in 1u32..=12u32,
        day in 1u32..=28u32,  // Safe for all months
        hour in 0u32..=23u32,
        minute in 0u32..=59u32,
        second in 0u32..=59u32,
        batch_name in "[a-z0-9-]{3,20}",  // Valid batch names
        sequence in 0u32..1000u32,
    ) {
        let timestamp = // ... construct DateTime
        let filename = generate_filename(&item, &batch_name, sequence);

        // Should never panic
        assert!(filename.len() > 0);
        // Should match expected format
        assert!(filename.contains(&batch_name));
    }
}
```

---

## Continuous Integration Test Data

In CI (GitHub Actions), test fixtures are committed, but manual data isn't available.

```yaml
# .github/workflows/test.yml
name: Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      # Test fixtures are already in git
      # No need to download manual test data

      - name: Run tests
        run: cargo test --workspace

      # Tests should pass with only fixtures
```

**Key Point:** All critical tests must work with committed fixtures only.

---

## Manual Testing Workflow

For manual testing during development:

```bash
# 1. Download test media (one-time)
./scripts/download-test-media.sh

# 2. Run folio with manual test data
cargo run --bin folio -- ingest \
    --source test-data/manual \
    --archive /tmp/folio-test-archive

# 3. Verify results
ls -R /tmp/folio-test-archive
cat /tmp/folio-test-archive/2024/11/04/*.xmp

# 4. Test with Lightroom (manual)
# Import /tmp/folio-test-archive into Lightroom
# Verify XMP sidecars are readable
```

---

## Summary

### ✅ Safe to Commit:
- `test-data/fixtures/` - Small synthetic files (<5MB total)
- Unit test code with `include_bytes!()` references

### ❌ Never Commit:
- `test-data/manual/` - Downloaded or exported test media
- `test-data/generated/` - Generated during tests
- `test-data/personal/` - Real family media (never use!)
- Any file >1MB (generally)

### 🔄 TDD Workflow:
1. Write **integration test** (CLI, fails ❌)
2. Write **unit tests** (library, fail ❌)
3. Implement **library functions** (unit tests pass ✅)
4. Wire up **CLI to library** (integration test passes ✅)
5. **Refactor** (tests still pass ✅)
6. **Repeat** for next feature

### 📊 Test Data Sources:
- **Unit tests:** Committed fixtures (`test-data/fixtures/`)
- **Integration tests:** Generated or fixtures (`assert_fs::TempDir`)
- **Manual testing:** Downloaded CC0 media (`test-data/manual/`)
- **Final validation:** Real backlog (outside project, `/tmp/`)

This strategy keeps your git repo clean, protects family privacy, and enables comprehensive TDD!
