# User Story: Backlog Media Ingestion

**Status:** In Progress (Slice 1 of 6 complete as of 2025-11-04)

## Story Description

**As a** family photographer with a 3-4 year backlog

**I want to** quickly ingest photos and video clips from SD cards and MacBook into the archive with automatic deduplication, metadata extraction, standardized file naming, and organized folder structure

**So that** I can clear the backlog of 6,000-8,000 unprocessed photos and videos efficiently while establishing a vendor-neutral foundation

## Detailed Narrative

I have photos and video clips from 2022-2025 sitting on SD cards and my MacBook that need to be organized into the family archive. Currently, the Lightroom workflow is unclear and I'm stuck with a growing backlog of mixed media.

I need a fast, reliable way to:
1. **Scan and group media** - Find photos/videos from source, group by date/time proximity, prompt for batch names
2. **Organize by date hierarchy** - Create `YYYY/MM/DD/` folder structure automatically
3. **Standardize filenames** - Rename to `YYYYMMDD-HHMMSS-{batch-name}.{ext}` format (preserving original extension)
4. **Deduplicate intelligently** - Skip files already in archive, but preserve/merge metadata if XMP sidecars differ
5. **Extract metadata** - Read EXIF from photos/videos, write to XMP sidecars for vendor neutrality
6. **Human-in-the-loop confirmations** - Confirm batch names, confirm copy operation, confirm when safe to reformat SD card
7. **Verify integrity** - Hash verification to ensure safe copy before confirming SD card can be reformatted

This is the **highest priority** - clearing the backlog while building the vendor-neutral foundation. Once photos and video clips are ingested with XMP sidecars, I can view/edit them in Lightroom, Urocissa, or any other XMP-compatible tool.

**Success scenario:**
- Insert SD card from D800 DSLR (e.g., 500 photos and 10 video clips from Thanksgiving 2024)
- Run: `folio ingest --source /Volumes/SD_CARD/DCIM --archive /archive`
- Tool scans source, finds 510 media files (photos + videos)
- Groups files by temporal proximity:
  - **Batch 1:** 2024-11-04 14:00-16:30 (245 files)
  - **Batch 2:** 2024-11-04 18:00-21:00 (265 files)
- **Prompt:** "Batch 1 (245 files, 2024-11-04 14:00-16:30) - Enter name: " → User enters "thanksgiving-arrival"
- **Prompt:** "Batch 2 (265 files, 2024-11-04 18:00-21:00) - Enter name: " → User enters "thanksgiving-dinner"
- Tool organizes into:
  - `/archive/2024/11/04/20241104-140215-thanksgiving-arrival.jpg`
  - `/archive/2024/11/04/20241104-180534-thanksgiving-dinner.jpg`
  - etc.
- Checks for duplicates: finds 15 files already exist, but 3 have different XMP metadata
- **Prompt:** "Found 15 duplicates. 3 have newer metadata. Merge? [y/N]" → User enters "y"
- Merges metadata for 3 files (preserves existing files, updates XMP sidecars)
- **Prompt:** "Ready to copy 495 new files (11 GB). Continue? [y/N]" → User enters "y"
- Copies files with progress bar (~2 minutes)
- Extracts EXIF metadata and writes XMP sidecars
- Verifies all file hashes match
- **Output:** "✓ All files copied and verified. SD card can be safely reformatted."
- Reports: "Ingested 495 media files (485 photos, 10 videos), 15 duplicates (3 metadata merged), 0 errors"
- Can immediately view in Lightroom or Urocissa

## Example Usage

### CLI Usage
```bash
# Ingest from SD card to archive with interactive prompts
folio ingest --source /Volumes/SD_CARD/DCIM --archive /archive

# Expected output
Scanning source: /Volumes/SD_CARD/DCIM
Found 510 media files (500 photos, 10 videos) - 11.2 GB total

Grouping by temporal proximity...
Found 2 temporal batches:

Batch 1: 2024-11-04 14:00:15 to 16:29:42 (245 files, 5.1 GB)
  Sample files: DSC_0001.JPG, DSC_0002.JPG ... DSC_0245.JPG
Enter batch name: thanksgiving-arrival

Batch 2: 2024-11-04 18:05:34 to 21:18:09 (265 files, 6.1 GB)
  Sample files: DSC_0246.JPG, DSC_0247.JPG ... DSC_0510.MOV
Enter batch name: thanksgiving-dinner

Checking for duplicates in /archive...
Found 15 files already exist (by hash)
  - 12 files: identical (will skip)
  - 3 files: XMP metadata differs (updated on source)

Merge newer metadata for 3 files? [y/N]: y

Summary:
  New files to copy: 495 (485 photos, 10 videos)
  Duplicates to skip: 12
  Metadata to merge: 3
  Destination structure: /archive/YYYY/MM/DD/YYYYMMDD-HHMMSS-{batch}.{ext}

Ready to copy 495 files (10.8 GB)? [y/N]: y

Copying files...
[========================================] 495/495 (100%) ETA: 0s

Extracting metadata...
[========================================] 495/495 (100%) ETA: 0s

Writing XMP sidecars...
[========================================] 498/498 (100%) ETA: 0s
  (495 new + 3 metadata merges)

Verifying file integrity...
[========================================] 495/495 (100%) ETA: 0s

✓ Success! All files copied and verified.

  Summary:
    Ingested: 495 media files (485 photos, 10 videos)
    Duplicates skipped: 12
    Metadata merged: 3
    Time: 1m 52s
    Destination: /archive/2024/11/04/

  ✓ SD card can be safely reformatted.

# Dry run (preview without copying or prompting)
folio ingest --source /Volumes/SD_CARD/DCIM --archive /archive --dry-run

# Expected output (dry run)
[DRY RUN] Would group into 2 temporal batches
[DRY RUN] Would copy 495 files (10.8 GB)
[DRY RUN] Would skip 12 duplicates
[DRY RUN] Would merge metadata for 3 files
[DRY RUN] Destination: /archive/2024/11/04/
```

### Library Usage
```rust
use folio_ingest::{IngestConfig, Ingester, BatchNamingStrategy};
use camino::Utf8Path;

fn example() -> Result<()> {
    let config = IngestConfig {
        source: Utf8Path::new("/Volumes/SD_CARD/DCIM"),
        archive_root: Utf8Path::new("/archive"),
        dry_run: false,
        verify_hashes: true,
        batch_naming: BatchNamingStrategy::Interactive, // Prompt user for batch names
        metadata_merge: true, // Merge metadata on duplicates
    };

    let ingester = Ingester::new(config)?;
    let result = ingester.run()?;

    println!("Ingested: {} media files ({} photos, {} videos)",
        result.ingested_count, result.photo_count, result.video_count);
    println!("Duplicates skipped: {}", result.duplicate_count);
    println!("Metadata merged: {}", result.metadata_merged_count);
    println!("Errors: {}", result.error_count);

    if result.all_verified {
        println!("✓ SD card can be safely reformatted");
    }

    Ok(())
}
```

## Acceptance Criteria

### Primary Flow: Media Discovery and Grouping
- [x] **Slice 1:** Scan source directory recursively for media files (JPEG, MOV, MP4)
- [x] **Slice 1:** Detect both photos (JPEG) and video clips (MOV, MP4) - CR2/NEF/MTS deferred
- [ ] **Slice 2:** Extract capture timestamp from EXIF (photos) or file metadata (videos)
- [ ] **Slice 3:** Group files into temporal batches based on time proximity (e.g., 2+ hour gap = new batch)
- [ ] **Slice 3:** Display batch information: date range, file count, total size
- [ ] **Slice 3:** Prompt user for batch name for each temporal group (interactive mode)
- [ ] **Slice 3:** Support `--batch-name` flag to apply single name to all files (non-interactive mode)

### Primary Flow: File Organization
- [ ] **Slice 2:** Create `YYYY/MM/DD/` folder hierarchy based on file capture date
- [ ] **Slice 3:** Rename files to `YYYYMMDD-HHMMSS-{batch-name}.{ext}` format
  - YYYYMMDD = capture date
  - HHMMSS = capture time
  - {batch-name} = user-provided name or original filename
  - {ext} = original file extension (preserve case: .jpg, .JPG, .MOV)
- [ ] **Slice 3:** Handle filename collisions (append sequence number if needed: `-001`, `-002`)
- [x] **Slice 1:** Preserve original file extension case (via case-insensitive detection)

### Primary Flow: Deduplication
- [x] **Slice 1:** Compute hash (BLAKE3) for each source file
- [x] **Slice 1:** Scan destination recursively to build hash index of existing files
- [x] **Slice 1:** Compare source hashes against destination hashes
- [x] **Slice 1:** For exact duplicates (matching hash): skip file copy
- [ ] **Slice 5:** For duplicates with different XMP metadata: detect metadata differences
- [ ] **Slice 5:** Prompt user: "Merge newer metadata? [y/N]"
- [ ] **Slice 5:** If yes: update existing XMP sidecar (don't re-copy media file)
- [ ] **Slice 5:** If no: skip file entirely

### Primary Flow: Metadata Extraction
- [ ] **Slice 4:** Extract EXIF metadata from photos (JPEG, CR2, NEF)
- [ ] **Slice 4:** Extract metadata from video files (creation date, camera model if available)
- [ ] **Slice 4:** Write XMP sidecar file for each photo with EXIF data
- [ ] **Slice 4:** Write XMP sidecar file for each video with available metadata
- [ ] **Slice 4:** Handle files without metadata gracefully (create minimal XMP)

### Primary Flow: Copy and Verification
- [ ] **Slice 6:** Display summary before copying:
  - Files to copy, duplicates to skip, metadata to merge
  - Total size, destination structure
- [ ] **Slice 6:** Prompt: "Ready to copy N files (X GB)? [y/N]"
- [ ] **Slice 6:** If no: abort without changes
- [ ] **Slice 6:** If yes: proceed with copy
- [x] **Slice 1:** Copy files (basic implementation, no progress bar yet)
- [ ] **Slice 6:** Add progress bar (current file, percentage, time remaining)
- [ ] **Slice 6:** Verify file integrity after copy (compare source/dest hashes)
- [ ] **Slice 6:** Fail and report if any hash mismatches detected
- [x] **Slice 1:** Generate basic summary (files ingested, photos/videos count, duplicates skipped)
- [ ] **Slice 6:** Enhanced summary with metadata merged, time elapsed
- [ ] **Slice 6:** Display: "✓ SD card can be safely reformatted" (only if all files verified)

### Secondary Flows
- [x] **Slice 1:** Support `--dry-run` flag to preview without copying files
- [ ] **Slice 3:** Support `--batch-name <name>` flag to use single name for all files (skip interactive prompts)
- [ ] **Slice 6:** Support `--yes` or `-y` flag to auto-confirm all prompts (dangerous but useful for automation)
- [x] **Slice 1:** Create destination directories if they don't exist
- [x] **Slice 1:** Handle multiple source subdirectories (walk entire tree recursively)
- [x] **Slice 1:** Preserve original file modified timestamps on copied files (via std::fs::copy)
- [x] **Slice 1:** Support re-running on partially completed ingestion (idempotent - skip already copied files via hash dedup)

### Error Handling
- [ ] Error if source directory doesn't exist or is unreadable
- [ ] Error if archive root doesn't exist or is not writable
- [ ] Warn and skip if individual file cannot be read (continue processing others)
- [ ] Warn if file has no timestamp metadata (use file modified date as fallback)
- [ ] Warn if EXIF/metadata extraction fails (still copy file, create minimal XMP, note in report)
- [ ] Warn if XMP sidecar write fails (file is copied, but metadata might be lost)
- [ ] Error and abort if hash verification fails after copy (data corruption detected)
- [ ] Handle user cancellation (Ctrl-C) gracefully - report progress so far
- [ ] Provide clear error messages with file paths and actionable guidance
- [ ] Exit code 0 on success, 1 on any errors, 130 on user cancellation

### Non-Functional Requirements
- [ ] **Performance:** Ingest 2,000 media files (photos + videos, ~40GB) in <5 minutes
- [ ] **Memory:** Use <1GB RAM even for large batches (stream processing, don't load all in memory)
- [ ] **Safety:** Never lose media data - verify hashes before reporting "safe to reformat"
- [ ] **Progress visibility:** Show real-time progress (current file, batch, %, time remaining)
- [ ] **Vendor neutrality:** XMP sidecars are readable by Lightroom, digiKam, Urocissa
- [ ] **Idempotent:** Can re-run safely - duplicates are always skipped, metadata can be re-merged
- [ ] **Interactive UX:** Prompts are clear, concise, with good defaults ([y/N] = default No)
- [ ] **Batch naming:** Temporal grouping should intelligently detect event boundaries

## Technical Requirements

### Core Library (`folio-core`) Requirements
- **Data structures:**
  - `MediaItem` - represents a photo or video with path, hash, metadata, timestamp, media_type
  - `MediaType` enum - Photo(format), Video(format)
  - `ExifData` - extracted EXIF/metadata fields (date, camera, settings, GPS)
  - `XmpMetadata` - XMP structure with merge capability
  - `TemporalBatch` - group of media items with time range, file list
  - `FileNamingScheme` - rules for generating standardized filenames
- **Public API functions:**
  - `hash_file(path: &Path) -> Result<Blake3Hash>` - compute file hash
  - `extract_metadata(path: &Path, media_type: MediaType) -> Result<ExifData>` - read EXIF/metadata from photo or video
  - `get_capture_timestamp(path: &Path) -> Result<DateTime>` - get capture time from EXIF or file metadata
  - `write_xmp_sidecar(media_path: &Path, metadata: &ExifData) -> Result<()>` - create XMP file
  - `read_xmp_sidecar(media_path: &Path) -> Result<Option<XmpMetadata>>` - parse existing XMP
  - `merge_xmp_metadata(existing: &XmpMetadata, new: &XmpMetadata) -> XmpMetadata` - merge two XMP sidecars
  - `group_by_temporal_proximity(items: &[MediaItem], gap_threshold: Duration) -> Vec<TemporalBatch>` - group files by time
  - `generate_filename(item: &MediaItem, batch_name: &str, sequence: u32) -> String` - create standardized filename
  - `generate_folder_path(timestamp: DateTime) -> PathBuf` - create `YYYY/MM/DD` path
- **Error types:**
  - `FolioError` - top-level error enum
  - Variants: `SourceNotFound`, `ArchiveNotWritable`, `FileReadError`, `MetadataExtractionFailed`, `XmpWriteFailed`, `HashMismatch`, `InvalidTimestamp`
- **Dependencies:**
  - `kamadak-exif` - EXIF reading from photos
  - `ffmpeg` crate or `mp4parse` - video metadata extraction
  - `blake3` - fast hashing
  - `quick-xml` or custom XMP library - XMP generation and parsing
  - `walkdir` - directory traversal
  - `chrono` - date/time handling

### CLI (`folio-cli`) Requirements
- **Command structure:**
  ```
  folio ingest --source <PATH> --archive <PATH> [OPTIONS]
  ```
- **Arguments:**
  - `--source, -s <PATH>` - source directory (required)
  - `--archive, -a <PATH>` - archive root directory (required)
  - `--dry-run` - preview mode without copying or prompting (optional)
  - `--batch-name <NAME>` - use single name for all files, skip interactive prompts (optional)
  - `--yes, -y` - auto-confirm all prompts (optional, dangerous)
  - `--gap-threshold <HOURS>` - hours between batches (default: 2)
- **Interactive prompts:**
  - Batch naming: "Enter batch name: " (for each temporal batch)
  - Metadata merge: "Merge newer metadata for N files? [y/N]: "
  - Copy confirmation: "Ready to copy N files (X GB)? [y/N]: "
  - All prompts have safe defaults (No)
  - Support Ctrl-C cancellation gracefully
- **Output formatting:**
  - Temporal batch summaries (date range, count, size)
  - Duplicate analysis (exact vs metadata-diff)
  - Copy summary before execution
  - Progress bars for: copy, metadata extraction, XMP writing, verification (using `indicatif`)
  - Final summary with timing
  - Clear "safe to reformat" message
- **Exit codes:**
  - 0: Success (all files ingested and verified)
  - 1: Errors occurred (partial success possible)
  - 130: User cancelled (Ctrl-C)

### Performance Requirements
- **Speed target:** 400+ photos/minute (for 20MB JPEGs)
  - Bottleneck likely: disk I/O (copying 40GB)
  - EXIF extraction and hashing should be fast (<100ms per photo)
- **Memory target:** <1GB RAM for 2,000 photo batch
  - Stream processing, don't load all files into memory
  - Process one photo at a time
- **Concurrency:** Single-threaded initially (simplicity), can parallelize later if needed

### Platform Requirements
- **macOS** - primary platform (development MacBook)
- **Filesystem:**
  - Read from SD card (exFAT or FAT32 typically)
  - Write to NAS (SMB/NFS network storage)
- **External dependencies:** None beyond Rust stdlib (prefer pure Rust crates)

## Test Strategy

### Library Unit Tests (`folio-core`)
- **`hash_file()` tests:**
  - Happy path: compute hash of sample JPEG
  - Same file twice produces same hash
  - Different files produce different hashes
  - Error case: file not found, permission denied
- **`extract_exif()` tests:**
  - Extract from D800 JPEG (sample fixture)
  - Extract date, camera model, GPS, settings
  - Handle JPEG without EXIF (empty result, not error)
  - Handle corrupted EXIF data (graceful failure)
- **`write_xmp_sidecar()` tests:**
  - Create XMP file next to photo
  - XMP contains correct metadata fields
  - XMP is valid XML
  - Overwrite existing XMP (or fail - TBD)
- **`read_xmp_sidecar()` tests:**
  - Parse XMP created by our tool
  - Parse XMP created by Lightroom (round-trip test)
  - Handle missing XMP file (None result)
  - Handle malformed XML (error)

### CLI Integration Tests (`folio-cli`)
- **Basic ingestion:**
  - Create test source directory with 10 sample JPEGs
  - Run `folio ingest --source <test-source> --dest <test-dest>`
  - Verify 10 files copied to dest
  - Verify 10 XMP sidecars created
  - Verify exit code 0
- **Deduplication:**
  - Ingest 10 files
  - Run ingest again with same source
  - Verify 0 files copied (all duplicates)
  - Verify message "10 duplicates skipped"
- **Dry run:**
  - Run `folio ingest --source <src> --dest <dest> --dry-run`
  - Verify NO files copied
  - Verify summary shows what WOULD happen
- **Error cases:**
  - Source directory doesn't exist → error message, exit code 1
  - Dest not writable → error message, exit code 1
  - Help output: `folio ingest --help` shows usage
- **Progress output:**
  - Mock slow copy to verify progress bar appears
  - Verify percentage and time estimates

### End-to-End Tests
- **Real workflow simulation:**
  - Copy actual D800 JPEG samples (not family photos!) to test source
  - Run ingestion
  - Open resulting XMP in text editor - verify valid XML
  - Import destination folder into Lightroom - verify metadata readable
  - Test with Urocissa (if available) - verify compatible
- **Performance benchmark:**
  - Use `criterion` to benchmark:
    - Hash computation time (per file)
    - EXIF extraction time (per file)
    - XMP write time (per file)
  - Target: <200ms total per 20MB file (excluding copy time)

### Test Data
- **Sample files needed:**
  - 10-20 D800 JPEG files (20-24MB each)
  - Mix of: with/without GPS, different dates, different camera settings
  - Corrupted JPEG (for error handling tests)
  - Non-JPEG file (for filtering tests)
- **Location:** `test-data/fixtures/d800-samples/`
- **How to generate:**
  - Export sample non-family photos from existing archive
  - Or download CC0 photos and add synthetic EXIF
- **IMPORTANT:** Never commit real family photos (see `.gitignore`)

## Dependencies

**External (Rust crates):**
- `kamadak-exif` v0.5 - mature EXIF library
- `blake3` v1.5 - fast, secure hashing
- `walkdir` v2.5 - recursive directory traversal
- `indicatif` v0.17 - progress bars
- `camino` v1.1 - UTF-8 paths
- `chrono` v0.4 - date/time handling
- Need to research: XMP library (or build with `quick-xml`)

**Internal:**
- None (this is the first feature)

**Related ADRs:**
- [ADR-0001: Metadata and Catalog Architecture](../adr/0001-metadata-catalog-architecture.md)

## Risks & Mitigations

- **Risk:** XMP format is complex, homegrown implementation may be incomplete
  - **Mitigation:** Start with minimal XMP (just EXIF fields), expand later. Test round-trip with Lightroom early.

- **Risk:** Performance insufficient for 40GB batches
  - **Mitigation:** Benchmark early. Optimize hot path (hashing, EXIF). Consider parallelization if needed.

- **Risk:** Deduplication has false positives (marks different photos as duplicates)
  - **Mitigation:** Use BLAKE3 (cryptographically strong). Test with similar photos. Add --force flag to skip dedup if needed.

- **Risk:** Data loss if copy fails partway through
  - **Mitigation:** Atomic operations. Verify hash after copy. Don't delete source until user confirms. Consider --verify flag.

- **Risk:** NAS may be slow for copying
  - **Mitigation:** Benchmark against local disk first. May need progress visibility more than raw speed.

## Notes

### Priority Justification
This is Phase 1, Priority 1 because:
- Directly addresses the 3-4 year backlog (primary goal)
- Establishes vendor-neutral foundation (XMP sidecars)
- Delivers immediate value (can start clearing backlog in 2-3 weeks)
- Validates architecture decision (XMP compatibility testing)

### Deferred to Later
- Mobile device ingestion (Phase 3) - important but backlog clearance from SD cards is higher priority
- Advanced XMP features (face tags, AI labels) - basic EXIF/metadata first
- Catalog/search index - not needed for ingestion workflow
- Automatic batch name suggestion (AI/ML-based event detection) - manual naming sufficient for now
- Parallel file copying - single-threaded sufficient initially

### Open Questions
- **XMP library:** Use existing crate or build custom with quick-xml?
  - Need to research available Rust XMP libraries
  - Must support XMP merging (not just writing)
  - Fallback: minimal XML generation for basic EXIF fields
- **Video metadata extraction:** Which Rust crate?
  - Options: `ffmpeg` (requires system lib), `mp4parse` (pure Rust), `mediainfo` wrapper
  - Need creation date, camera model if available
  - Preference: pure Rust to avoid system dependencies
- **Temporal grouping threshold:** 2 hours default appropriate?
  - Test with real SD card data
  - Should it be adaptive based on total time span?
- **Filename collision handling:** Sequence suffix `-001`, `-002` or timestamp milliseconds?
  - `-001` more human-readable
  - Milliseconds more unique but ugly
- **XMP metadata merge strategy:** Which fields take precedence?
  - Newer timestamp wins? User-edited fields win? Merge tags/keywords?
  - Start simple: newer file wins for all fields
- **Batch naming validation:** Restrict characters? (e.g., no spaces, use hyphens)
  - Good for filesystem compatibility
  - Enforce: `[a-z0-9-_]` only?