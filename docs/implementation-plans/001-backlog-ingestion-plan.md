# Backlog Media Ingestion - Implementation Plan

**Feature:** Backlog Media Ingestion (Photos + Videos)

**User Story:** [001-backlog-ingestion.md](../user-stories/001-backlog-ingestion.md)

**Related ADR:** [ADR-0001: Metadata and Catalog Architecture](../adr/0001-metadata-catalog-architecture.md)

**Approach:** Vertical Slicing with Outside-In TDD

---

## Implementation Strategy

This implementation follows **vertical slicing** - each slice delivers end-to-end user value and can be tested/released independently.

Given the expanded scope (video support, temporal batching, file naming, metadata merging, interactive prompts), this is a substantial feature. We'll break it into **6 vertical slices** that build incrementally.

*When developing this implementation plan, also consider the following documentation, and note any updates to documentation required by the user story implementation:*
- [Main README](../../README.md)
- [CLAUDE](../../CLAUDE.md)
- [Current State Analysis](../current-state.md)
- [Key Insights](../key-insights.md)

---

## Vertical Slices

### Slice 1: Walking Skeleton - Media Discovery and Simple Copy

**Status:** ✅ Completed (2025-11-04)

**User Value:** Can copy photos AND videos from SD card to archive with basic deduplication. Supports mixed media from day one.

**Acceptance Criteria:**
- [x] CLI command `folio ingest --source <src> --dest <dest>` works
- [x] Scans source directory recursively for media files (JPEG, MOV, MP4)
- [x] Detects media type (photo vs video) by extension
- [x] Computes BLAKE3 hash for each file
- [x] Scans archive recursively to build hash index
- [x] Compares hashes to identify duplicates
- [x] Copies new files to archive root (simple: no folder structure yet)
- [x] Reports: "Found X media files (Y photos, Z videos)" and "Copied N files" / "Skipped N duplicate files"
- [x] Exit code 0 on success, 1 on errors
- [ ] Progress bar shows copy progress (deferred - not needed for small batches)

**Core Library Implementation (`folio-core`):**
- [x] ~~Create `src/hash.rs` module~~ - Integrated into `src/media.rs`
  - [x] `pub fn hash_file(path: &Path) -> Result<Blake3Hash>` - uses `blake3::Hash` directly
- [x] Create `src/media.rs` module
  - [x] Define `MediaType` enum: `Photo(PhotoFormat)`, `Video(VideoFormat)`
  - [x] Define `PhotoFormat` enum: `Jpeg` (Cr2, Nef deferred)
  - [x] Define `VideoFormat` enum: `Mov`, `Mp4` (Mts deferred)
  - [x] Define `MediaItem` struct with `path`, `hash`, `size`, `media_type` fields
  - [x] `pub fn detect_media_type(path: &Path) -> Option<MediaType>` - by extension (case-insensitive)
  - [x] `pub fn scan_directory(path: &Path) -> Result<Vec<MediaItem>>` - find media files recursively
  - [x] ~~`pub fn build_hash_index()`~~ - Implemented inline in CLI (not needed as library function yet)
- [x] ~~Create `src/error.rs`~~ - Using `anyhow::Result` for MVP simplicity
- [x] Export public API in `src/lib.rs`

**Core Library Unit Tests:**
- [x] `hash_file()` - compute hash of sample JPEG fixture
- [x] `hash_file()` - same file twice produces same hash
- [x] ~~`hash_file()` - different files produce different hashes~~ - Implicit in other tests
- [x] ~~`hash_file()` - error when file not found~~ - Covered by anyhow error handling
- [x] `detect_media_type()` - recognizes .jpg, .JPG, .jpeg, .MOV, .mp4 (case-insensitive)
- [x] `detect_media_type()` - returns None for .txt, .pdf
- [x] `scan_directory()` - finds mixed JPEG and MOV files in fixtures
- [x] `scan_directory()` - skips non-media files (test.txt)
- [x] ~~`scan_directory()` - recursive subdirectories~~ - Covered by fixtures test
- [x] ~~`scan_directory()` - handles empty directory~~ - Not needed for MVP
- [x] ~~`build_hash_index()` - creates correct hash → path mapping~~ - Not implemented as library function

**CLI Implementation (`folio-cli`):**
- [x] Update `src/main.rs` with `Commands::Ingest` structure:
  - [x] `--source, -s <PATH>` (required)
  - [x] `--dest, -d <PATH>` (required) - using "dest" instead of "archive" for clarity
  - [x] `--dry-run` flag (optional)
- [x] Implement ingest handler:
  - [x] Scan source directory
  - [x] Scan destination directory (build hash index inline)
  - [x] Identify duplicates by hash
  - [x] Copy new files (no progress bar yet - deferred)
  - [x] Count photos vs videos separately
  - [x] Format summary output
- [x] Handle errors with helpful messages using anyhow Context

**CLI Integration Tests:**
- [x] Test ingest with mixed media (2 files: 1 photo, 1 video) - `test_ingest_mixed_media`
- [x] Test filtering of non-media files - `test_ingest_filters_non_media`
- [x] Test dry-run mode - `test_ingest_dry_run`
- [x] Assert files copied correctly
- [x] Assert exit code 0
- [x] Assert output contains "Found N media files"
- [x] Run ingest again → duplicates detected and skipped
- [x] Updated to use `cargo_bin!` macro (avoiding deprecated `Command::cargo_bin`)

**Documentation:**
- [x] Add rustdoc comments to core functions in `media.rs`
- [ ] Update README.md with basic usage example (deferred to later slice)
- [x] CLI `--help` text works (auto-generated by clap)

**Notes:**
- No folder structure or file naming yet - files go to dest root
- No metadata extraction yet - just file copying with dedup
- This slice lets user start clearing backlog immediately with mixed media
- Keep it simple: linear scanning, single-threaded
- **Actual implementation:** Simpler than planned - no separate hash module, used `anyhow` for errors

---

### Slice 2: Timestamp Extraction and YYYY/MM/DD Folder Organization

**Status:** Not Started

**User Value:** Files are automatically organized into date-based folders. Archive structure is clean and navigable.

**Acceptance Criteria:**
- [ ] Extracts capture timestamp from photos (EXIF DateTimeOriginal)
- [ ] Extracts capture timestamp from videos (file creation date or metadata)
- [ ] Falls back to file modified date if no capture timestamp available
- [ ] Creates `YYYY/MM/DD/` folder hierarchy in archive
- [ ] Copies files into date-based folders
- [ ] Reports files without valid timestamps (warnings)

**Core Library Implementation (`folio-core`):**
- [ ] Create `src/timestamp.rs` module
  - [ ] `pub fn get_capture_timestamp(path: &Path, media_type: &MediaType) -> Result<Option<DateTime<Utc>>>`
  - [ ] For photos: use `kamadak-exif` to read EXIF DateTimeOriginal
  - [ ] For videos: use file metadata (creation date) as fallback
  - [ ] Return None if no timestamp found (will use modified date)
  - [ ] `pub fn get_file_modified_date(path: &Path) -> Result<DateTime<Utc>>` - fallback
  - [ ] `pub fn generate_folder_path(timestamp: DateTime<Utc>) -> PathBuf` - create `YYYY/MM/DD`
- [ ] Update `MediaItem` struct:
  - [ ] Add `timestamp: Option<DateTime<Utc>>` field
  - [ ] Add `folder_path: PathBuf` field (computed from timestamp)
- [ ] Update `scan_directory()` to extract timestamps

**Core Library Unit Tests:**
- [ ] `get_capture_timestamp()` - extract from D800 JPEG with EXIF
- [ ] `get_capture_timestamp()` - handle JPEG without EXIF (returns None)
- [ ] `get_capture_timestamp()` - extract from MOV file
- [ ] `get_capture_timestamp()` - error handling for corrupted files
- [ ] `get_file_modified_date()` - fallback works correctly
- [ ] `generate_folder_path()` - 2024-11-04 → `2024/11/04`
- [ ] `generate_folder_path()` - 2022-01-15 → `2022/01/15`

**CLI Implementation (`folio-cli`):**
- [ ] Update ingest handler:
  - [ ] Extract timestamps during scan
  - [ ] Use modified date as fallback
  - [ ] Create date-based folder structure in archive
  - [ ] Copy files to appropriate folders
  - [ ] Warn about files without timestamps

**CLI Integration Tests:**
- [ ] Ingest files with EXIF timestamps → assert correct folder structure
- [ ] Ingest files without timestamps → assert uses modified date
- [ ] Verify `archive/2024/11/04/` created and populated
- [ ] Mix of dates → multiple folders created

**Documentation:**
- [ ] Document folder structure convention
- [ ] Document timestamp extraction logic (EXIF → creation → modified)
- [ ] Update README with example folder structure

**Notes:**
- No file renaming yet - original filenames preserved
- Focus on correct date extraction and folder organization
- Timestamp extraction is critical for next slice (temporal batching)

---

### Slice 3: Temporal Batching and Interactive File Naming

**Status:** Not Started

**User Value:** Files from the same event are grouped together and named consistently. User provides meaningful batch names interactively.

**Acceptance Criteria:**
- [ ] Groups media files by temporal proximity (default: 2+ hour gap = new batch)
- [ ] Displays batch information: date range, file count, total size, sample files
- [ ] Prompts user for batch name for each temporal group
- [ ] Validates batch name (alphanumeric + hyphens/underscores only)
- [ ] Renames files to `YYYYMMDD-HHMMSS-{batch-name}.{ext}` format
- [ ] Handles filename collisions (appends `-001`, `-002`, etc.)
- [ ] Supports `--batch-name <name>` flag to skip prompts (use single name for all)
- [ ] Supports `--gap-threshold <hours>` to adjust grouping sensitivity

**Core Library Implementation (`folio-core`):**
- [ ] Create `src/batching.rs` module
  - [ ] Define `TemporalBatch` struct with `start_time`, `end_time`, `items: Vec<MediaItem>`, `batch_name: Option<String>`
  - [ ] `pub fn group_by_temporal_proximity(items: &[MediaItem], gap_threshold: Duration) -> Vec<TemporalBatch>`
    - Sort items by timestamp
    - Group consecutive items with gap < threshold
  - [ ] `pub fn generate_filename(item: &MediaItem, batch_name: &str, sequence: u32) -> String`
    - Format: `YYYYMMDD-HHMMSS-{batch-name}.{ext}`
    - Example: `20241104-140215-thanksgiving-arrival.jpg`
  - [ ] `pub fn validate_batch_name(name: &str) -> Result<()>` - check alphanumeric + `-_`
- [ ] Update `MediaItem` with `generated_filename: Option<String>` field

**Core Library Unit Tests:**
- [ ] `group_by_temporal_proximity()` - 2 batches with 2-hour gap
- [ ] `group_by_temporal_proximity()` - single batch (all within threshold)
- [ ] `group_by_temporal_proximity()` - 3 batches with varying gaps
- [ ] `group_by_temporal_proximity()` - empty input
- [ ] `generate_filename()` - correct format with batch name
- [ ] `generate_filename()` - sequence number handling
- [ ] `generate_filename()` - preserves extension case (.JPG vs .jpg)
- [ ] `validate_batch_name()` - accepts valid names
- [ ] `validate_batch_name()` - rejects spaces, special chars

**CLI Implementation (`folio-cli`):**
- [ ] Add `--batch-name <NAME>` flag (optional)
- [ ] Add `--gap-threshold <HOURS>` flag (default: 2.0)
- [ ] Implement interactive batch naming:
  - [ ] Display batch summary (date range, count, samples)
  - [ ] Prompt: "Enter batch name: "
  - [ ] Read user input
  - [ ] Validate input, re-prompt if invalid
  - [ ] Store batch name
- [ ] Generate filenames for all items in batch
- [ ] Handle filename collisions (detect, append sequence)
- [ ] Copy files with new names

**CLI Integration Tests:**
- [ ] Test with mocked stdin input (simulate user entering batch names)
- [ ] Test `--batch-name` flag (no prompts)
- [ ] Test temporal grouping with known timestamps
- [ ] Test filename generation and collision handling
- [ ] Verify final filenames match expected format

**Documentation:**
- [ ] Document batch naming interactive workflow
- [ ] Document filename format specification
- [ ] Document `--batch-name` and `--gap-threshold` flags
- [ ] Add examples with different batch scenarios

**Notes:**
- Interactive prompts are critical - test carefully
- Filename collision handling must be robust
- Consider adding `--yes/-y` flag later to auto-confirm (not in this slice)

---

### Slice 4: Metadata Extraction and XMP Sidecar Generation

**Status:** Not Started

**User Value:** Metadata is preserved in vendor-neutral XMP format. Can view/edit ingested media in Lightroom, digiKam, Urocissa.

**Acceptance Criteria:**
- [ ] Extracts EXIF metadata from photos
- [ ] Extracts available metadata from videos (creation date, camera if present)
- [ ] Writes XMP sidecar file for each photo with EXIF data
- [ ] Writes XMP sidecar file for each video with available metadata
- [ ] Handles files without metadata gracefully (creates minimal XMP)
- [ ] XMP is valid XML
- [ ] XMP sidecars are readable by Lightroom (manual test)
- [ ] XMP sidecars are readable by digiKam (manual test)

**Core Library Implementation (`folio-core`):**
- [ ] Create `src/metadata.rs` module
  - [ ] Define `ExifData` struct with common fields:
    - `date_time_original: Option<DateTime<Utc>>`
    - `camera_make: Option<String>`
    - `camera_model: Option<String>`
    - `lens_model: Option<String>`
    - `aperture: Option<f64>`
    - `shutter_speed: Option<String>`
    - `iso: Option<u32>`
    - `gps_latitude: Option<f64>`
    - `gps_longitude: Option<f64>`
  - [ ] `pub fn extract_metadata(path: &Path, media_type: &MediaType) -> Result<ExifData>`
    - For photos: use `kamadak-exif`
    - For videos: basic metadata only (date, camera if available)
- [ ] Create `src/xmp.rs` module
  - [ ] Define `XmpMetadata` struct
  - [ ] `pub fn create_xmp_from_metadata(metadata: &ExifData) -> XmpMetadata`
  - [ ] `pub fn write_xmp_sidecar(media_path: &Path, xmp: &XmpMetadata) -> Result<()>`
  - [ ] `pub fn read_xmp_sidecar(media_path: &Path) -> Result<Option<XmpMetadata>>`
  - [ ] Use `quick-xml` for XML generation/parsing
  - [ ] XMP format (minimal for MVP):
    ```xml
    <?xml version="1.0" encoding="UTF-8"?>
    <x:xmpmeta xmlns:x="adobe:ns:meta/">
     <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
      <rdf:Description xmlns:exif="http://ns.adobe.com/exif/1.0/"
                       xmlns:dc="http://purl.org/dc/elements/1.1/">
       <exif:DateTimeOriginal>2024-11-04T14:02:15</exif:DateTimeOriginal>
       <exif:Make>Nikon</exif:Make>
       <exif:Model>D800</exif:Model>
       ...
      </rdf:Description>
     </rdf:RDF>
    </x:xmpmeta>
    ```

**Core Library Unit Tests:**
- [ ] `extract_metadata()` - from D800 JPEG with EXIF
- [ ] `extract_metadata()` - from JPEG without EXIF (returns empty struct)
- [ ] `extract_metadata()` - from MOV file
- [ ] `create_xmp_from_metadata()` - converts metadata to XMP structure
- [ ] `write_xmp_sidecar()` - creates valid XML file
- [ ] `read_xmp_sidecar()` - parses XMP we created
- [ ] Round-trip test: metadata → XMP → parse → verify fields preserved
- [ ] XMP validation: parse with `quick-xml` to verify well-formed

**CLI Implementation (`folio-cli`):**
- [ ] After copying file, extract metadata
- [ ] Generate XMP sidecar
- [ ] Write XMP file next to media file
- [ ] Show progress: "Writing XMP sidecars..."
- [ ] Warn if metadata extraction fails (but continue)

**CLI Integration Tests:**
- [ ] Ingest photos with EXIF → assert `.xmp` files created
- [ ] Ingest videos → assert `.xmp` files created
- [ ] Parse XMP files → verify valid XML
- [ ] Verify XMP contains expected metadata fields

**Documentation:**
- [ ] Document XMP sidecar format
- [ ] Add example XMP file to docs
- [ ] Update README: emphasize vendor neutrality
- [ ] Document manual testing procedure (Lightroom/digiKam import)

**Notes:**
- **Critical slice** - achieves vendor-neutral architecture goal
- Must manually test with Lightroom/digiKam/Urocissa after implementation
- Start with minimal XMP (just EXIF fields), expand later if needed
- Research available Rust XMP libraries first, fall back to custom XML if needed

---

### Slice 5: Intelligent Deduplication with Metadata Merging

**Status:** Not Started

**User Value:** Duplicates with updated metadata don't lose edits. Can re-run ingestion safely without losing XMP changes.

**Acceptance Criteria:**
- [ ] Detects exact duplicates (matching hash)
- [ ] Detects duplicates with different XMP metadata
- [ ] Displays duplicate analysis: "12 exact, 3 with metadata differences"
- [ ] Prompts: "Merge newer metadata for 3 files? [y/N]"
- [ ] If yes: updates existing XMP sidecars (doesn't re-copy media files)
- [ ] If no: skips files entirely
- [ ] Reports: "X duplicates skipped, Y metadata merged"

**Core Library Implementation (`folio-core`):**
- [ ] Extend `src/xmp.rs` module:
  - [ ] `pub fn merge_xmp_metadata(existing: &XmpMetadata, new: &XmpMetadata) -> XmpMetadata`
    - Simple strategy: newer file wins for all fields
    - Can make more sophisticated later (merge tags, preserve user edits)
  - [ ] `pub fn has_metadata_differences(xmp1: &XmpMetadata, xmp2: &XmpMetadata) -> bool`
- [ ] Update deduplication logic:
  - [ ] Build hash index with XMP info
  - [ ] Categorize duplicates: exact vs metadata-diff
  - [ ] Return `DeduplicationResult` struct with both categories

**Core Library Unit Tests:**
- [ ] `merge_xmp_metadata()` - merges two XMP structures
- [ ] `merge_xmp_metadata()` - newer timestamp wins
- [ ] `has_metadata_differences()` - detects differences
- [ ] `has_metadata_differences()` - identical XMP returns false
- [ ] Dedup logic categorizes duplicates correctly

**CLI Implementation (`folio-cli`):**
- [ ] Analyze duplicates during scan phase
- [ ] Display duplicate summary (exact vs metadata-diff)
- [ ] Interactive prompt for metadata merge decision
- [ ] If user confirms: update XMP sidecars for affected files
- [ ] Report merge count in final summary

**CLI Integration Tests:**
- [ ] Ingest files, modify XMP, re-ingest → detect metadata differences
- [ ] Test merge confirmation (yes/no)
- [ ] Verify XMP files updated when merge confirmed
- [ ] Verify media files not re-copied

**Documentation:**
- [ ] Document metadata merge behavior
- [ ] Explain merge strategy (newer wins)
- [ ] Document use case: re-running ingestion safely

**Notes:**
- Start with simple merge strategy (newer wins all fields)
- More sophisticated merging (preserve user tags, etc.) can come later
- This enables safe re-runs of ingestion workflow

---

### Slice 6: Human-in-the-Loop Confirmations and Safety Features

**Status:** Not Started

**User Value:** User has full control and visibility before operations. Safety confirmation before SD card can be reformatted.

**Acceptance Criteria:**
- [ ] Displays comprehensive summary before copying:
  - Files to copy (count, size, breakdown by photo/video)
  - Duplicates to skip
  - Metadata to merge
  - Destination structure example
- [ ] Prompt: "Ready to copy N files (X GB)? [y/N]"
- [ ] If no: abort without changes, exit code 0
- [ ] If yes: proceed with copy
- [ ] After copy, verify file integrity (compare hashes)
- [ ] Display: "✓ All files copied and verified. SD card can be safely reformatted."
- [ ] Only show "safe to reformat" if 100% verified
- [ ] Supports `--dry-run` flag (no prompts, no copying, just preview)
- [ ] Supports `--yes/-y` flag to auto-confirm (dangerous, for automation)
- [ ] Handle Ctrl-C gracefully (report progress, exit code 130)

**Core Library Implementation (`folio-core`):**
- [ ] Add verification functions:
  - [ ] `pub fn verify_file_integrity(source_hash: &Blake3Hash, dest_path: &Path) -> Result<bool>`
  - [ ] Re-hash destination file, compare with source

**Core Library Unit Tests:**
- [ ] `verify_file_integrity()` - succeeds when hashes match
- [ ] `verify_file_integrity()` - fails when hashes differ

**CLI Implementation (`folio-cli`):**
- [ ] Add `--dry-run` flag
  - [ ] If set: perform all analysis, skip copying and prompts
  - [ ] Print "[DRY RUN]" summary
- [ ] Add `--yes/-y` flag
  - [ ] If set: auto-answer "yes" to all prompts
  - [ ] Dangerous - document clearly
- [ ] Implement pre-copy summary:
  - [ ] Display comprehensive operation summary
  - [ ] Show destination structure example
  - [ ] Prompt for confirmation
- [ ] Implement post-copy verification:
  - [ ] Progress bar: "Verifying file integrity..."
  - [ ] Verify each copied file
  - [ ] Fail loudly if any hash mismatch
- [ ] Implement graceful Ctrl-C handling:
  - [ ] Catch SIGINT
  - [ ] Report progress so far
  - [ ] Exit with code 130
- [ ] Final message logic:
  - [ ] Only show "safe to reformat" if `all_verified == true`

**CLI Integration Tests:**
- [ ] Test `--dry-run` (no files copied, no prompts)
- [ ] Test confirmation prompt (simulate "n" → abort)
- [ ] Test confirmation prompt (simulate "y" → proceed)
- [ ] Test `--yes` flag (no prompts, auto-proceed)
- [ ] Test verification success (all hashes match)
- [ ] Test verification failure (simulate corruption)
- [ ] Test Ctrl-C handling (if possible with `assert_cmd`)

**Documentation:**
- [ ] Document interactive workflow with screenshots/examples
- [ ] Document `--dry-run` and `--yes` flags
- [ ] Document safety features (verification, confirmations)
- [ ] Add "Safe to Reformat SD Card" section to README

**Notes:**
- User safety is paramount - multiple confirmation points
- Clear, actionable prompts with safe defaults
- Verification is non-negotiable - never say "safe to reformat" without it

---

## Slice Priority and Dependencies

| Slice | Priority | Depends On |  Status | Actual Time | Notes |
|-------|----------|------------|---------|-------------|-------|
| Slice 1: Media Discovery & Copy | Must Have | None | ✅ Completed | <1 day | TDD workflow made this faster than estimated |
| Slice 2: Timestamp & Folders | Must Have | Slice 1 | Not Started | Est: 2-3 days | |
| Slice 3: Temporal Batching & Naming | Must Have | Slice 2 | Not Started | Est: 4-5 days | |
| Slice 4: Metadata & XMP | Must Have | Slice 1 | Not Started | Est: 4-5 days | |
| Slice 5: Metadata Merging | Should Have | Slice 4 | Not Started | Est: 2-3 days | |
| Slice 6: Confirmations & Safety | Must Have | Slice 1-4 | Not Started | Est: 2-3 days | |

**Recommended order:** 1 → 2 → 3 → 4 → 6 (then 5 if time)

**MVP:** Slices 1-4 + 6 deliver core backlog clearance value

**Total estimated time:** 16-22 days (3-4 weeks of focused development)

---

## Definition of Done

The feature is complete when ALL of the following are true:

### Code Quality
- [ ] All tests passing: `cargo test --workspace`
- [ ] Code formatted: `cargo fmt --check`
- [ ] No clippy warnings: `cargo clippy -- -D warnings`
- [ ] Library documentation complete with examples: `cargo doc --open`
- [ ] CLI help text accurate and helpful: `folio ingest --help`
- [ ] No `unsafe` code (or justified and documented)

### Functionality
- [ ] All acceptance criteria from user story are met
- [ ] All vertical slices 1-4 + 6 marked as "Completed" (slice 5 optional for MVP)
- [ ] Supports photos (JPEG) and videos (MOV, MP4)
- [ ] Creates `YYYY/MM/DD/` folder structure
- [ ] Renames files to `YYYYMMDD-HHMMSS-{batch}.{ext}` format
- [ ] Interactive batch naming works correctly
- [ ] XMP sidecars generated for all media
- [ ] Deduplication with metadata merge works
- [ ] All interactive prompts function correctly
- [ ] Verification confirms safe SD card reformatting

### Performance
- [ ] Performance target met: 2,000 media files in <5 minutes (real timing test)
- [ ] Memory usage <1GB for large batches (test with 2,000+ files)

### Compatibility Testing
- [ ] XMP compatibility validated: Manual import test with Lightroom
- [ ] XMP compatibility validated: Manual import test with digiKam or Urocissa
- [ ] Round-trip test: Rust creates XMP → Lightroom edits → Rust reads changes

### Documentation
- [ ] README.md updated with complete ingestion workflow
- [ ] Examples added showing interactive workflow
- [ ] All CLI flags documented
- [ ] User story acceptance criteria checked off
- [ ] Implementation plan marked as completed

### Real-world Validation
- [ ] Real backlog test: Process 100-500 actual backlog photos/videos successfully
- [ ] Verify folder structure is correct
- [ ] Verify filenames are correct
- [ ] Verify XMP sidecars readable by Lightroom
- [ ] Verify can safely reformat SD card after ingestion

---

## Learnings & Adjustments

### What's Working Well

- **Outside-in TDD approach** - Writing integration tests first (CLI level) then implementing library functions worked perfectly. Tests drove the design naturally.
- **Test fixtures strategy** - The `test-data/fixtures/` approach with small synthetic media files (<100KB) makes tests fast and safe to commit to git.
- **Using `assert_cmd` and `assert_fs`** - These crates make CLI testing straightforward. The `cargo_bin!` macro pattern is cleaner than deprecated `Command::cargo_bin()`.
- **BLAKE3 for hashing** - Extremely fast, simple API. Perfect for deduplication.
- **`walkdir` for directory traversal** - Reliable, handles edge cases well.
- **Case-insensitive extension matching** - Using `.to_lowercase()` on extensions handles mixed-case files (`.JPG`, `.jpg`, `.MOV`) elegantly.

### Challenges Encountered

- **Test fixture generation requires external tools** - The `setup-test-fixtures.sh` script depends on ImageMagick (`convert`), `exiftool`, and `ffmpeg`. These need to be installed before running the setup script. This is acceptable for developer setup but worth documenting.
- **`Command::cargo_bin()` deprecated** - The `assert_cmd` example patterns use deprecated API. Updated to use `cargo_bin!` macro instead. This was caught by compiler warnings.
- **Video fixture files are larger than expected** - Even 1-second video files are ~12KB each (vs ~600 bytes for minimal JPEG). Total fixtures still well under 5MB limit, but worth noting for future fixture planning.

### Adjustments Made to Plan

- **Simplified error handling** - Used `anyhow::Result` instead of custom `FolioError` enum. For MVP, the flexibility and ergonomics of `anyhow` outweigh benefits of typed errors. Can add typed errors later if needed.
- **No separate hash module** - Integrated hashing directly into `media.rs`. The `blake3::Hash` type is sufficient; no need for custom wrapper type at this stage.
- **Deferred progress bars** - Simple file copying is fast enough without progress indication for test fixtures. Will add `indicatif` progress bars in later slice when processing larger batches.
- **Changed CLI flag name** - Used `--dest` instead of `--archive` for destination. Shorter and clearer in context of the command.
- **Skipped some planned tests** - Eliminated redundant tests (e.g., "different files produce different hashes" is implicit). Focused on essential test coverage.

### Lessons for Future Features

- **Start simple, iterate** - The initial plan had more complexity than needed. Starting with the simplest implementation that passes tests was faster and easier to understand.
- **Trust the test-driven process** - Writing failing tests first really does clarify requirements and drive clean API design. Don't skip the "red" phase.
- **Keep test fixtures committed** - Having test fixtures in git makes tests reproducible across machines and CI environments. The setup script makes regeneration easy if needed.
- **Integration tests are invaluable** - CLI integration tests caught issues that unit tests wouldn't (like output formatting, exit codes, dry-run logic).
- **Defer optimization** - No progress bars, no parallel copying, no custom error types in Slice 1. These can be added when needed. Focus on working end-to-end functionality first.
- **Document setup requirements** - External tool dependencies (ImageMagick, exiftool, ffmpeg) need clear documentation for new contributors.
