# Example Data Strategy for Folio

## Overview

This document defines the strategy for generating, version-controlling, and distributing realistic example media files that demonstrate complete user story workflows. These examples enable contributors and users to test Folio's ingestion and organization capabilities with realistic data without requiring personal photos or videos.

## Vision and Goals

### Problem Statement

Testing family media software requires realistic data:
- **Privacy concerns**: Can't share real family photos in public repositories
- **Size constraints**: Real media files are too large for git version control
- **Reproducibility**: Contributors need identical test data for consistent testing
- **Realism**: Synthetic fixtures are insufficient for demonstrating real workflows

### Solution

**AI-generated example media + version-controlled distribution:**
1. **Generate** realistic photos and videos using AI services (Stable Diffusion, ffmpeg)
2. **Version control** generated files in a separate GitHub repository ([folio-example-data](https://github.com/padamson/folio-example-data))
3. **Release** versioned archives matching Folio releases (e.g., v0.1.3)
4. **Download** pre-generated data via CLI tool (no API keys required for end users)
5. **Regenerate** data when workflows change using `generate-examples` binary

### Key Benefits

- **Realistic**: AI-generated photos look like real family photos (people, landscapes, events)
- **Safe to share**: No privacy concerns, freely distributable
- **Version controlled**: Tied to specific Folio versions and user stories
- **Cost-effective**: ~$0.30 per dataset using optimized AI generation
- **Reproducible**: Same data for all contributors
- **No API keys needed**: End users download pre-generated files

## Architecture

### Crate Structure

**New crate:** `crates/folio-examples/`

```
crates/folio-examples/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Shared types and utilities
│   ├── generator/          # AI generation logic
│   │   ├── mod.rs
│   │   ├── photo.rs        # Stable Diffusion integration
│   │   ├── video.rs        # ffmpeg video stitching
│   │   └── metadata.rs     # EXIF/XMP generation
│   ├── downloader/         # GitHub release fetching
│   │   ├── mod.rs
│   │   └── github.rs       # GitHub API integration
│   ├── bin/
│   │   ├── generate-examples.rs   # Binary: AI generation
│   │   └── download-examples.rs   # Binary: Fetch from GitHub
│   └── workflows/          # User story workflow definitions
│       ├── mod.rs
│       └── story_001.rs    # Thanksgiving SD card scenario
├── tests/
│   └── integration.rs
└── README.md
```

### Two Binary Workflow

#### 1. `generate-examples` (For Maintainers)

**Purpose:** Generate AI photos and videos, package into versioned archives

**Requirements:**
- Replicate API key (for Stable Diffusion)
- ffmpeg installed locally
- Network access

**Usage:**
```bash
# Set API key
export REPLICATE_API_TOKEN="your-token"

# Generate example data for User Story 001
cargo run --bin generate-examples -- \
    --story 001 \
    --output example-data/001-thanksgiving \
    --release v0.1.3

# This creates:
# - 100 AI-generated photos (800x600, realistic D800 style)
# - 2 videos (stitched from 30 AI frames each, 5 seconds @ 6fps)
# - DCIM/100NIKON folder structure
# - Realistic EXIF metadata with temporal batching
# - Total cost: ~$0.30 via Replicate API
```

**Output:** Generates archive ready for GitHub release:
```
example-data/
└── 001-thanksgiving/
    ├── folio-example-001-v0.1.3.tar.gz   # Compressed archive (~150 MB)
    ├── checksums.txt                      # SHA256 checksums
    └── DCIM/                              # Uncompressed source
        └── 100NIKON/
            ├── DSC_0001.JPG
            ├── ...
            └── DSC_0101.MOV
```

#### 2. `download-examples` (For Contributors)

**Purpose:** Download pre-generated example data from GitHub releases

**Requirements:**
- Network access only (no API keys!)

**Usage:**
```bash
# Download example data for User Story 001
cargo run --bin download-examples -- \
    --story 001 \
    --version v0.1.3 \
    --output test-data/examples/

# This downloads from:
# https://github.com/padamson/folio-example-data/releases/download/v0.1.3/folio-example-001-v0.1.3.tar.gz

# Verifies checksums and extracts to:
# test-data/examples/001-thanksgiving/
```

## Version Control Strategy

### Separate Repository

**Repository:** `https://github.com/padamson/folio-example-data`

**Why separate?**
- Keep main Folio repo lightweight (no large binary files)
- Enable git LFS for efficient large file storage
- Independent release cycle for example data
- Clear separation of code vs. data

**Structure:**
```
folio-example-data/
├── README.md                   # Usage instructions
├── .gitattributes              # Git LFS configuration
├── story-001/                  # User Story 001 examples
│   ├── DCIM/
│   │   └── 100NIKON/
│   │       ├── DSC_0001.JPG
│   │       ├── ...
│   │       └── DSC_0101.MOV
│   └── README.md               # Story-specific documentation
├── story-002/                  # Future: User Story 002 examples
└── checksums/
    ├── v0.1.3.txt
    └── v0.2.0.txt
```

### Versioning Scheme

**Format:** `v0.x.y`
- `0` = Pre-release (before v1.0.0)
- `x` = User story number
- `y` = Slice number

**Examples:**
- `v0.1.3` = User Story 001, Slice 3 (initial example data)
- `v0.1.4` = User Story 001, Slice 4 (updated example data)
- `v0.2.1` = User Story 002, Slice 1

**Compatibility:**
- Example data version matches Folio version
- `folio v0.1.3` expects `folio-example-data v0.1.3`
- Breaking changes to workflows increment user story number (x)

### Release Process

**When to release new example data:**
1. New user story implemented (e.g., Story 002 complete)
2. Workflow changes affecting example outputs (e.g., new metadata fields)
3. Bug fixes requiring updated example data

**Steps:**
1. **Generate** new example data:
   ```bash
   cargo run --bin generate-examples -- \
       --story 001 \
       --output example-data/001-thanksgiving \
       --release v0.1.3
   ```

2. **Commit** to `folio-example-data` repository:
   ```bash
   cd ../folio-example-data
   git add story-001/
   git commit -m "Add User Story 001 example data (v0.1.3)"
   git tag v0.1.3
   git push origin v0.1.3
   ```

3. **Create GitHub Release**:
   - Upload `folio-example-001-v0.1.3.tar.gz` as release asset
   - Upload `checksums.txt` as release asset
   - Release notes describe the workflow scenario

4. **Update Folio documentation**:
   - Link to example data release in user story documentation
   - Update CHANGELOG.md to reference available example data

## AI Generation Implementation

### Photo Generation (Stable Diffusion)

**Service:** Replicate API ([replicate.com](https://replicate.com))

**Model:** `stability-ai/sdxl:latest` (Stable Diffusion XL)

**Cost:** ~$0.002 per 800x600 image = **$0.20 for 100 photos**

**Prompt Strategy:**

Generate realistic family event photos with variety:

```rust
// Example prompts for Thanksgiving scenario
let prompts = vec![
    // Arrival batch (14:00-16:30)
    "family arriving at house, car in driveway, warm afternoon light, photorealistic",
    "family members greeting at front door, hugs, candid moment",
    "kids playing in backyard, autumn leaves, golden hour lighting",
    "landscape photo of house exterior, fall foliage, DSLR quality",
    // ... 47 more varied prompts

    // Dinner batch (18:00-21:00)
    "thanksgiving dinner table, food, warm indoor lighting, photorealistic",
    "family seated around dinner table, laughing, candid",
    "close-up of turkey and side dishes, food photography",
    "portrait of family member smiling, indoor flash, shallow depth of field",
    // ... 47 more varied prompts
];

for (i, prompt) in prompts.iter().enumerate() {
    let image = replicate::generate_image(
        model: "stability-ai/sdxl:latest",
        prompt: prompt,
        width: 800,
        height: 600,
        seed: 42 + i,  // Reproducible results
    ).await?;

    // Save as DSC_{0001 + i}.JPG
    image.save(format!("DSC_{:04}.JPG", 1 + i))?;
}
```

**Resolution:** 800x600 pixels
- Realistic for D800 downsampled files (~1-2 MB per JPEG)
- Small enough for fast downloads (~150 MB total compressed)
- Large enough for realistic workflow testing

**EXIF Metadata:**

After generation, add realistic D800 EXIF using `kamadak-exif` crate:

```rust
use exif::{In, Tag};

fn add_d800_metadata(path: &Path, timestamp: DateTime, gps: Option<(f64, f64)>) -> Result<()> {
    let file = File::open(path)?;
    let mut bufreader = BufReader::new(&file);
    let exifreader = exif::Reader::new();
    let mut exif = exifreader.read_from_container(&mut bufreader)?;

    // Camera info
    exif.set_field(Tag::Make, "Nikon");
    exif.set_field(Tag::Model, "D800");
    exif.set_field(Tag::LensModel, "AF-S NIKKOR 24-70mm f/2.8G ED");

    // Timestamp
    exif.set_field(Tag::DateTimeOriginal, timestamp.format("%Y:%m:%d %H:%M:%S").to_string());

    // Camera settings (realistic for family event)
    exif.set_field(Tag::FNumber, "2.8");
    exif.set_field(Tag::ExposureTime, "1/125");
    exif.set_field(Tag::ISOSpeedRatings, "400");
    exif.set_field(Tag::FocalLength, "50mm");

    // GPS (if provided)
    if let Some((lat, lon)) = gps {
        exif.set_field(Tag::GPSLatitude, lat);
        exif.set_field(Tag::GPSLongitude, lon);
    }

    // Write back to file
    exif.write_to_file(path)?;
    Ok(())
}
```

### Video Generation (ffmpeg Frame Stitching)

**Innovation:** Instead of expensive AI video generation, stitch AI photo frames into videos

**Cost:** 30 frames per video × $0.002 = **$0.06 per video** (vs. $0.50+ for AI video)

**Process:**

1. **Generate 30 AI frames** for each video:
   ```rust
   // Generate 30 frames for 5-second video @ 6fps
   for frame_num in 0..30 {
       let prompt = format!(
           "video frame {}/30: family member walking in living room, motion blur, photorealistic",
           frame_num + 1
       );

       let image = replicate::generate_image(
           model: "stability-ai/sdxl:latest",
           prompt: &prompt,
           width: 1920,
           height: 1080,
           seed: 1000 + frame_num,
       ).await?;

       image.save(format!("frame_{:04}.jpg", frame_num))?;
   }
   ```

2. **Stitch frames with ffmpeg**:
   ```rust
   use std::process::Command;

   fn stitch_video_frames(frames_dir: &Path, output: &Path) -> Result<()> {
       Command::new("ffmpeg")
           .args(&[
               "-framerate", "6",                    // 6 fps (30 frames = 5 seconds)
               "-pattern_type", "glob",
               "-i", "frame_*.jpg",
               "-c:v", "libx264",                    // H.264 codec
               "-pix_fmt", "yuv420p",                // Compatible pixel format
               "-movflags", "+faststart",            // Optimize for streaming
               "-an",                                 // No audio
               output.to_str().unwrap(),
           ])
           .current_dir(frames_dir)
           .output()?;

       Ok(())
   }
   ```

3. **Add QuickTime metadata**:
   ```rust
   // Use exiftool to add MOV metadata
   Command::new("exiftool")
       .args(&[
           "-Make=Nikon",
           "-Model=D800",
           "-CreateDate=2024:11:04 15:23:45",
           "-overwrite_original",
           output.to_str().unwrap(),
       ])
       .output()?;
   ```

**Output:** MOV files (~10-20 MB each) with realistic motion and metadata

**Total Cost:**
- 100 photos: $0.20
- 2 videos (60 frames): $0.12
- **Total: ~$0.32 per dataset**

## User Story 001: Thanksgiving SD Card Scenario

### Workflow Description

**Scenario:** Simulates a family returning from Thanksgiving 2024 with a D800 SD card containing photos and videos from two distinct events.

**Temporal Batches:**
1. **Batch 1: Thanksgiving Arrival** (51 files)
   - Time range: 2024-11-04 14:00:15 to 16:29:42
   - Content: Family arriving, outdoor photos, kids playing, landscape shots
   - 50 photos + 1 video (5 seconds)

2. **Batch 2: Thanksgiving Dinner** (51 files)
   - Time range: 2024-11-04 18:05:34 to 21:18:09
   - Content: Indoor dinner, food photos, candid family moments, portraits
   - 50 photos + 1 video (5 seconds)

**Gap:** 1 hour 35 minutes between batches (exceeds default 2-hour threshold for temporal grouping)

### Example Data Structure

```
example-data/001-thanksgiving/
└── DCIM/
    └── 100NIKON/
        ├── DSC_0001.JPG       # 2024-11-04 14:00:15 (arrival)
        ├── DSC_0002.JPG       # 2024-11-04 14:03:28
        ├── DSC_0003.JPG       # 2024-11-04 14:07:52
        ├── ...
        ├── DSC_0050.JPG       # 2024-11-04 16:29:42
        ├── DSC_0051.MOV       # 2024-11-04 15:23:45 (arrival video)
        ├── DSC_0052.JPG       # 2024-11-04 18:05:34 (dinner)
        ├── DSC_0053.JPG       # 2024-11-04 18:12:19
        ├── ...
        ├── DSC_0100.JPG       # 2024-11-04 21:18:09
        └── DSC_0101.MOV       # 2024-11-04 19:47:23 (dinner video)
```

### Demonstrated Workflows

This example data enables testing all User Story 001 acceptance criteria:

#### 1. Fresh Ingestion (Interactive)
```bash
cargo run --bin folio -- ingest \
    --source test-data/examples/001-thanksgiving/DCIM \
    --archive /tmp/folio-archive

# Expected:
# - Scans 102 files
# - Groups into 2 batches
# - Prompts for batch names
# - Copies all files
# - Generates XMP sidecars
```

#### 2. Deduplication
```bash
# Run twice to verify idempotency
cargo run --bin folio -- ingest \
    --source test-data/examples/001-thanksgiving/DCIM \
    --archive /tmp/folio-archive \
    --batch-name thanksgiving

# Second run should skip all 102 files (duplicates)
```

#### 3. Non-Interactive Batch Naming
```bash
cargo run --bin folio -- ingest \
    --source test-data/examples/001-thanksgiving/DCIM \
    --archive /tmp/folio-archive \
    --batch-name thanksgiving-2024
```

#### 4. Dry Run Preview
```bash
cargo run --bin folio -- ingest \
    --source test-data/examples/001-thanksgiving/DCIM \
    --archive /tmp/folio-archive \
    --dry-run
```

#### 5. Custom Temporal Batching
```bash
# 4-hour threshold → 1 batch
cargo run --bin folio -- ingest \
    --source test-data/examples/001-thanksgiving/DCIM \
    --archive /tmp/folio-archive \
    --gap-threshold 4 \
    --batch-name thanksgiving

# Expected: 1 batch (102 files)
```

#### 6. Browser Preview
```bash
cargo run --bin folio -- ingest \
    --source test-data/examples/001-thanksgiving/DCIM \
    --archive /tmp/folio-archive \
    --preview

# Opens browser to http://127.0.0.1:RANDOM_PORT
# Shows batch groupings in real-time
```

### Verification

After downloading example data, verify characteristics:

```bash
# Count files
find test-data/examples/001-thanksgiving -type f | wc -l
# Expected: 102

# Check EXIF timestamps
exiftool -DateTimeOriginal test-data/examples/001-thanksgiving/DCIM/100NIKON/DSC_0001.JPG
# Expected: 2024:11:04 14:00:15

# Verify camera model
exiftool -Make -Model test-data/examples/001-thanksgiving/DCIM/100NIKON/DSC_0001.JPG
# Expected: Nikon D800

# Check file sizes
du -sh test-data/examples/001-thanksgiving/
# Expected: ~150 MB uncompressed
```

## Usage for Contributors

### Quick Start

**Download example data** (no API key required):

```bash
# From folio project root
cargo run --bin download-examples -- \
    --story 001 \
    --version v0.1.3

# This creates: test-data/examples/001-thanksgiving/
```

**Run workflows:**

```bash
# Fresh ingestion
cargo run --bin folio -- ingest \
    --source test-data/examples/001-thanksgiving/DCIM \
    --archive /tmp/folio-test

# Verify results
ls -R /tmp/folio-test
```

### Regeneration (Maintainers Only)

If workflows change and example data needs updating:

```bash
# Set API key
export REPLICATE_API_TOKEN="r8_..."

# Regenerate
cargo run --bin generate-examples -- \
    --story 001 \
    --output example-data/001-thanksgiving \
    --release v0.1.4

# Commit to folio-example-data repo
cd ../folio-example-data
git add story-001/
git commit -m "Update User Story 001 examples (v0.1.4)"
git tag v0.1.4
git push origin v0.1.4

# Create GitHub release with archive
gh release create v0.1.4 \
    example-data/folio-example-001-v0.1.4.tar.gz \
    example-data/checksums.txt
```

## Integration with Testing

### Integration Tests

```rust
// crates/folio-cli/tests/thanksgiving_test.rs
use assert_cmd::Command;
use assert_fs::prelude::*;

#[test]
fn test_thanksgiving_workflow() {
    // Download example data first (or skip test if not available)
    let example_path = PathBuf::from("test-data/examples/001-thanksgiving");
    if !example_path.exists() {
        eprintln!("Skipping: Example data not downloaded");
        eprintln!("Run: cargo run --bin download-examples -- --story 001");
        return;
    }

    // Setup
    let archive = assert_fs::TempDir::new().unwrap();

    // Act
    let mut cmd = Command::cargo_bin("folio").unwrap();
    cmd.arg("ingest")
        .arg("--source").arg(example_path.join("DCIM"))
        .arg("--archive").arg(archive.path())
        .arg("--batch-name").arg("thanksgiving");

    // Assert
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("Ingested 102 files"));

    // Verify structure
    assert!(archive.child("2024/11/04").exists());

    // Verify XMP sidecars
    let xmp_count = walkdir::WalkDir::new(archive.path())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension() == Some("xmp".as_ref()))
        .count();
    assert_eq!(xmp_count, 102);
}
```

### CI/CD Integration

```yaml
# .github/workflows/test.yml
name: Integration Tests with Example Data

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      # Download example data
      - name: Download example data
        run: |
          cargo run --bin download-examples -- \
            --story 001 \
            --version v0.1.3 \
            --output test-data/examples/

      # Run integration tests
      - name: Run tests
        run: cargo nextest run --workspace
```

## Future User Stories

### Story 002: Ongoing Family Events

**Scenario:** Monthly family gatherings with incremental ingestion

**Example data:**
- Multiple SD card dumps (monthly)
- Partial duplicates (some photos already archived)
- Demonstrates incremental workflow

### Story 003: Multi-Device Ingestion

**Scenario:** Photos from multiple devices (DSLR, iPhone, Android)

**Example data:**
- Mixed formats (JPEG, HEIC, MP4, MOV)
- Different EXIF formats
- Demonstrates multi-device support

## Cost Analysis

### Per Dataset Costs

**User Story 001 (Thanksgiving):**
- 100 photos @ $0.002 each = $0.20
- 60 video frames (2 videos) @ $0.002 each = $0.12
- **Total: $0.32 per generation**

**Amortized Costs:**
- Generate once, distribute infinitely via GitHub releases
- Contributors download pre-generated data (no cost)
- Regenerate only when workflows change (~2-3 times per story)

### Comparison to Alternatives

| Approach | Cost | Quality | Shareable |
|----------|------|---------|-----------|
| **AI Generation + Download** (our approach) | $0.32 once | High | ✅ Yes |
| Manual curation (CC0 photos) | Free | Medium | ✅ Yes |
| Real family photos (anonymized) | Free | High | ❌ Privacy risk |
| AI video generation | $1.50+ | High | ✅ Yes |
| Synthetic test data | Free | Low | ✅ Yes |

**Winner:** AI generation with frame stitching provides best quality/cost/shareability balance.

## Technical Dependencies

### Rust Crates

```toml
[dependencies]
# HTTP client for Replicate API
reqwest = { version = "0.11", features = ["json", "blocking"] }
tokio = { version = "1", features = ["full"] }

# Image processing
image = "0.25"
kamadak-exif = "0.5"

# Archive creation
tar = "0.4"
flat2 = "1.0"

# GitHub API integration
octocrab = "0.38"

# CLI
clap = { version = "4.5", features = ["derive"] }
```

### External Tools

- **ffmpeg**: Required for video frame stitching
- **exiftool**: Optional (can use kamadak-exif instead)

### API Keys

- **Replicate API**: Required for `generate-examples` binary
  - Get key: https://replicate.com/account/api-tokens
  - Pricing: Pay-as-you-go (~$0.002 per image)

## Summary

### Key Innovations

1. **AI-generated realism** without privacy concerns
2. **Frame-stitching video** dramatically reduces cost ($0.06 vs $0.50+ per video)
3. **Version-controlled distribution** via separate GitHub repo
4. **Dual-binary approach** separates generation (maintainers) from download (contributors)
5. **Semantic versioning** ties example data to user stories and slices

### Workflow

```
Maintainer:                       Contributor:
┌─────────────────┐              ┌─────────────────┐
│ generate-examples│              │download-examples│
│  (Replicate API)│              │  (GitHub API)   │
└────────┬────────┘              └────────┬────────┘
         │                                 │
         ▼                                 ▼
   ┌─────────────┐               ┌─────────────────┐
   │folio-example│──release──────▶│test-data/       │
   │   -data     │     v0.1.3     │  examples/      │
   │  (GitHub)   │                │  001-thanksgiving│
   └─────────────┘                └─────────────────┘
```

### Benefits

- **Privacy-safe**: No real family photos
- **Cost-effective**: ~$0.30 per dataset
- **Reproducible**: Same data for all contributors
- **Realistic**: AI-generated photos/videos look authentic
- **Shareable**: Freely distributable via GitHub
- **Versioned**: Tied to Folio releases
