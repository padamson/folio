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

**Status:** ✅ Completed (2025-11-04)

**User Value:** Files are automatically organized into date-based folders. Archive structure is clean and navigable.

**Acceptance Criteria:**
- [x] Extracts capture timestamp from photos (EXIF DateTimeOriginal)
- [x] Extracts capture timestamp from videos (file creation date or metadata)
- [x] Falls back to file modified date if no capture timestamp available
- [x] Creates `YYYY/MM/DD/` folder hierarchy in archive
- [x] Copies files into date-based folders
- [x] Reports files without valid timestamps (warnings - uses modified date fallback)

**Core Library Implementation (`folio-core`):**
- [x] ~~Create `src/timestamp.rs` module~~ - Integrated into `src/media.rs`
  - [x] `pub fn get_capture_timestamp(path: &Path, media_type: &MediaType) -> Result<Option<DateTime<Utc>>>`
  - [x] For photos: use `kamadak-exif` to read EXIF DateTimeOriginal
  - [x] For videos: return None (will use file metadata as fallback)
  - [x] Return None if no timestamp found (will use modified date)
  - [x] `pub fn get_file_modified_date(path: &Path) -> Result<DateTime<Utc>>` - fallback
  - [x] `pub fn generate_folder_path(timestamp: DateTime<Utc>) -> PathBuf` - create `YYYY/MM/DD`
- [x] Update `MediaItem` struct:
  - [x] Add `timestamp: Option<DateTime<Utc>>` field
  - [x] Add `folder_path: PathBuf` field (computed from timestamp)
- [x] Update `scan_directory()` to extract timestamps

**Core Library Unit Tests:**
- [x] ~~`get_capture_timestamp()` - extract from DSLR JPEG with EXIF~~ - Tested via integration test
- [x] ~~`get_capture_timestamp()` - handle JPEG without EXIF (returns None)~~ - Tested via integration test
- [x] ~~`get_capture_timestamp()` - extract from MOV file~~ - Tested via integration test (returns None, uses fallback)
- [x] ~~`get_capture_timestamp()` - error handling for corrupted files~~ - Deferred (not needed for MVP)
- [x] ~~`get_file_modified_date()` - fallback works correctly~~ - Tested implicitly via integration test
- [x] ~~`generate_folder_path()` - 2024-11-04 → `2024/11/04`~~ - Tested via integration test
- [x] ~~`generate_folder_path()` - 2022-01-15 → `2022/01/15`~~ - Covered by integration test

**CLI Implementation (`folio-cli`):**
- [x] Update ingest handler:
  - [x] Extract timestamps during scan (integrated into `scan_directory`)
  - [x] Use modified date as fallback
  - [x] Create date-based folder structure in archive
  - [x] Copy files to appropriate folders
  - [x] ~~Warn about files without timestamps~~ - Silently uses fallback (good UX)

**CLI Integration Tests:**
- [x] Ingest files with EXIF timestamps → assert correct folder structure (`test_ingest_organizes_by_date`)
- [x] ~~Ingest files without timestamps → assert uses modified date~~ - Covered by updated existing tests
- [x] Verify `archive/2024/11/04/` created and populated
- [x] ~~Mix of dates → multiple folders created~~ - Implicitly tested by existing tests

**Documentation:**
- [ ] Document folder structure convention (deferred to Slice 6)
- [ ] Document timestamp extraction logic (EXIF → creation → modified) (deferred to Slice 6)
- [ ] Update README with example folder structure (deferred to Slice 6)

**Notes:**
- No file renaming yet - original filenames preserved
- Focus on correct date extraction and folder organization
- Timestamp extraction is critical for next slice (temporal batching)

---

### Slice 3a: Project Configuration - Specialized Agents and Nextest Setup

**Status:** ✅ Completed (2025-11-14)

**User Value:** Developer workflow efficiency and automation. Specialized agents handle common development tasks (TDD, docs, releases) with project-specific guidance.

**Note:** This was an infrastructure slice to support future development. No feature code changes.

**Acceptance Criteria:**
- [x] Three specialized agents configured (.claude/agents/)
- [x] TDD agent adapted from playwright-rust to Folio's approach
- [x] Documentation agent adapted with Folio-specific hierarchy
- [x] Release preparation agent configured for Folio
- [x] Nextest configured for fast, parallel test execution
- [x] Nextest retry logic and CI profiles configured
- [x] Gitignore updated for nextest artifacts
- [x] CLAUDE.md updated with agent documentation
- [x] CLAUDE.md updated with doctest strategy
- [x] All test commands in CLAUDE.md use nextest

**Key Architectural Insights:**
- **Agent terminology adaptation**: Adapted agents from "phases" terminology (playwright-rust) to "user stories + implementation plans + slices" (Folio's approach)
- **TDD agent customization**: Emphasized outside-in TDD (integration tests first) and real media validation with multiple formats (JPEG, HEIC, MOV, MP4)
- **Doctest strategy**: Module-level consolidation with `no_run` for I/O-heavy examples prevents slow doctests while maintaining executable documentation
- **Nextest configuration**: Retry logic (retries = 2) handles flaky tests, separate CI profile for strict checks
- **Vendor neutrality in agents**: All agent documentation emphasizes XMP generation and vendor-neutral workflows as core principles

**Configuration Files:**
- `.claude/agents/tdd-feature-implementation.md` - Complete rewrite for Folio's TDD approach with nextest
- `.claude/agents/documentation-maintenance.md` - Adapted for Folio's documentation hierarchy
- `.claude/agents/release-preparation.md` - Adapted for Folio's release process
- `.config/nextest.toml` - Nextest configuration with retries and CI profile
- `.gitignore` - Added nextest artifacts (`.nextest/`)
- `CLAUDE.md` - Added "Specialized Development Agents" section (80+ lines)
- `CLAUDE.md` - Added "Documentation Testing Strategy (Doctests)" section
- `CLAUDE.md` - Updated all `cargo test` commands to `cargo nextest run`

**Documentation:**
- CLAUDE.md comprehensively documents all three agents with usage guidance
- Agent invocation triggers clearly defined (automatic pattern matching)
- Doctest strategy documented with examples and best practices
- All Rust test commands updated to use nextest throughout CLAUDE.md

---

### Slice 3b: Temporal Batching and Interactive File Naming

**Status:** ✅ Completed (2025-11-05)

**User Value:** Files from the same event are grouped together and named consistently. User provides meaningful batch names interactively.

**Acceptance Criteria:**
- [x] Groups media files by temporal proximity (default: 2+ hour gap = new batch)
- [x] Displays batch information: count and gap threshold
- [x] Prompts user for batch name for each temporal group
- [x] Validates batch name (alphanumeric + hyphens/underscores only)
- [x] Renames files to `YYYYMMDD-HHMMSS-{batch-name}.{ext}` format
- [ ] Handles filename collisions (appends `-001`, `-002`, etc.) - Deferred to future enhancement
- [x] Supports `--batch-name <name>` flag to skip prompts (use single name for all)
- [x] Supports `--gap-threshold <hours>` to adjust grouping sensitivity

**Core Library Implementation (`folio-core`):**
- [x] ~~Create `src/batching.rs` module~~ - Integrated into `media.rs` instead
  - [x] Define `TemporalBatch` struct with `start_time`, `end_time`, `items: Vec<MediaItem>`
  - [x] `pub fn group_by_temporal_proximity(items: &[MediaItem], gap_threshold: Duration) -> Vec<TemporalBatch>`
    - Filters out items without timestamps
    - Sorts items by timestamp
    - Groups consecutive items with gap < threshold
    - Starts new batch when gap exceeds threshold
  - [x] `pub fn validate_batch_name(name: &str) -> Result<()>` - check alphanumeric + `-_`
- [x] Implement `pub fn generate_filename(timestamp: DateTime<Utc>, batch_name: &str, original_extension: &str) -> String` in `media.rs`
  - Format: `YYYYMMDD-HHMMSS-{batch-name}.{ext}`
  - Example: `20241104-140215-thanksgiving-arrival.jpg`
  - Uses `Timelike` trait for hour/minute/second

**Core Library Unit Tests:**
- [x] `group_by_temporal_proximity()` - 2 batches with 2-hour gap
- [x] `group_by_temporal_proximity()` - single batch (all within threshold)
- [x] `group_by_temporal_proximity()` - 3 batches with varying gaps
- [x] `group_by_temporal_proximity()` - empty input
- [x] `validate_batch_name()` - accepts valid names (9 tests covering all cases)
- [x] `validate_batch_name()` - rejects spaces, special chars, empty strings

**CLI Implementation (`folio-cli`):**
- [x] Add `--batch-name <NAME>` flag (optional)
- [x] Add `--gap-threshold <HOURS>` flag (default: 2.0)
- [x] Add `chrono` dependency to `Cargo.toml`
- [x] Import `group_by_temporal_proximity()` and `Duration`
- [x] **Flag behavior logic (Option 1)**:
  - [x] If `--batch-name` provided → treat all as single batch, disable temporal batching
  - [x] If NO `--batch-name` → detect temporal batches using `--gap-threshold`, prompt for names
  - [x] Display appropriate message based on mode
- [x] Detect temporal batches after scanning source (when NO `--batch-name`)
- [x] Display batch count and gap threshold (when NO `--batch-name`)
- [x] Implement interactive batch naming (when NO `--batch-name` and NOT `--dry-run`):
  - [x] Display batch summary (date range, count, samples)
  - [x] Prompt: "Enter batch name: "
  - [x] Read user input from stdin
  - [x] Validate input with `validate_batch_name()`, re-prompt if invalid
  - [x] Store batch name for use during copy phase
  - [x] Helper function `prompt_for_batch_name()` handles full prompt loop
- [x] Generate filenames for all batches (single or multiple)
  - [x] Extract timestamp from MediaItem or fall back to file modified date
  - [x] Preserve original extension
  - [x] Use `generate_filename()` from folio-core
- [ ] Handle filename collisions (detect, append sequence) - Deferred to future enhancement
- [x] Copy files with generated names to date-based folders
- [x] Refactored file copying to iterate over `batches_with_names` pairs

**CLI Integration Tests:**
- [x] Test with mocked stdin input (simulate user entering batch names) - 2 new tests
- [x] Test `--batch-name` flag (no prompts) - `test_ingest_with_batch_name`
  - [x] Verifies filenames match `YYYYMMDD-HHMMSS-{batch-name}.{ext}` format
  - [x] Uses two fixtures with different EXIF timestamps
  - [x] Verifies files placed in correct date-based folders
- [x] Test temporal grouping detection - `test_ingest_detects_temporal_batches_with_dry_run`
  - [x] Uses two photos 4+ hours apart (> 2-hour threshold)
  - [x] Verifies output shows "Detected 2 temporal batches"
  - [x] Uses `--dry-run` (no `--batch-name`) to test detection
- [x] Test `--batch-name` disables temporal batching - `test_ingest_batch_name_disables_temporal_batching`
  - [x] Photos 4+ hours apart with `--batch-name`
  - [x] Verifies "temporal batching disabled" message
  - [x] Verifies NO "Detected N batches" message
  - [x] All files get same batch name
- [x] Test custom `--gap-threshold` - `test_ingest_custom_gap_threshold`
  - [x] Photos 4h 13m apart with 5.0 hour threshold
  - [x] Verifies grouped into 1 batch (not 2)
  - [x] Uses `--dry-run` without `--batch-name`
- [x] Test batch name validation - `test_ingest_validates_batch_name`
  - [x] Verifies CLI rejects invalid batch names
  - [x] Verifies error message shown to user
- [x] Test interactive mode with valid input - `test_ingest_interactive_mode_with_valid_input`
  - [x] Mocks stdin with valid batch names for 2 batches
  - [x] Verifies batch prompts shown
  - [x] Verifies files created with correct batch names
- [x] Test interactive mode with invalid then valid input - `test_ingest_interactive_mode_with_invalid_then_valid_input`
  - [x] Mocks stdin with invalid name first, then valid
  - [x] Verifies re-prompting on validation failure
  - [x] Verifies error message displayed

**Documentation:**
- [ ] Document batch naming interactive workflow - Deferred to Slice 6
- [ ] Document filename format specification - Deferred to Slice 6
- [ ] Document `--batch-name` and `--gap-threshold` flags - Deferred to Slice 6
- [ ] Add examples with different batch scenarios - Deferred to Slice 6

**Notes:**
- Interactive prompts implemented and tested with mocked stdin
- Filename collision handling deferred to future enhancement (low priority)
- Three operating modes working correctly:
  1. Single batch mode (`--batch-name` flag)
  2. Interactive mode (NO flags, prompts for each batch)
  3. Dry-run mode (`--dry-run` flag, uses placeholder names)

---

### Slice 3c: Live Browser Preview Dashboard

**Status:** Not started

**Dependency:** This slice requires `playwright-rust` to be published to crates.io with basic browser automation features. Development of playwright-rust is happening as a separate project and will be driven by the needs of this slice.

**User Value:** Visual confirmation of batch grouping and file naming. See thumbnails of first/last photos in each batch. Real-time feedback on workflow progress.

**Acceptance Criteria:**
- [ ] Automatically opens browser preview when ingestion starts (unless --headless)
- [ ] Shows workflow progress (Scan → Group → Name → Review → Copy)
- [ ] Displays batch cards with thumbnails of first and last photo
- [ ] Shows original filename → new filename transformation for each batch
- [ ] Updates live as user provides batch names in terminal
- [ ] Highlights current batch being named
- [ ] Shows completed batches with green checkmarks
- [ ] Displays batch statistics (file count, size, time range)
- [ ] Read-only (no interaction in browser, all control in terminal)
- [ ] WebSocket connection for real-time updates
- [ ] Graceful handling if browser closed (doesn't break workflow)
- [ ] Cleanup HTML/temp files on exit

**Core Library Implementation (`folio-core`):**
- [ ] Create `src/preview.rs` module
  - [ ] Define `PreviewState` struct with workflow state
  - [ ] Define `BatchPreview` struct with batch info and thumbnail data
  - [ ] `pub fn generate_thumbnail(image_path: &Path, max_size: u32) -> Result<Vec<u8>>` - resize image for preview
  - [ ] `pub fn encode_thumbnail_base64(thumbnail: &[u8]) -> String` - for embedding in HTML
- [ ] Create `src/preview_server.rs` module
  - [ ] Tiny HTTP server using `warp` or `axum`
  - [ ] WebSocket endpoint for state updates
  - [ ] Serves single HTML page with embedded CSS/JS
  - [ ] `pub fn start_preview_server() -> Result<PreviewServer>` - returns URL and handle
  - [ ] `pub fn update_preview(server: &PreviewServer, state: PreviewState) -> Result<()>` - push update
  - [ ] `pub fn shutdown_preview(server: PreviewServer) -> Result<()>`

**Core Library Unit Tests:**
- [ ] `generate_thumbnail()` - creates thumbnail from JPEG
- [ ] `generate_thumbnail()` - handles various image sizes
- [ ] `generate_thumbnail()` - error handling for non-images
- [ ] `encode_thumbnail_base64()` - produces valid base64
- [ ] `PreviewServer::start()` - server starts and listens
- [ ] `PreviewServer::update()` - state updates pushed via WebSocket
- [ ] `PreviewServer::shutdown()` - server stops cleanly

**CLI Implementation (`folio-cli`):**
- [ ] Add `--headless` flag to skip browser preview
- [ ] Start preview server after scanning completes
- [ ] Open default browser to preview URL
- [ ] Generate thumbnails for first/last photo of each batch
- [ ] Update preview state as user names batches
- [ ] Update preview state during review phase
- [ ] Update preview state during copy phase (progress bar)
- [ ] Shutdown preview server on exit (success or error)
- [ ] Handle browser closed gracefully (catch WebSocket disconnect)

**Browser Testing (`folio-cli` integration tests):**
- [ ] Add `playwright-rust` as dev dependency (when available on crates.io)
- [ ] Write E2E test: start ingestion, verify browser opens
- [ ] Test: verify HTML contains correct batch information
- [ ] Test: verify thumbnails load correctly (check img src)
- [ ] Test: simulate naming batch in terminal, verify browser updates
- [ ] Test: verify workflow progress steps update correctly
- [ ] Test: verify batch cards show correct state (pending/active/complete)
- [ ] Test: verify preview works with --headless (no browser opens)
- [ ] CI setup: install chromedriver or geckodriver in GitHub Actions
- [ ] Test: verify cleanup on normal exit
- [ ] Test: verify cleanup on Ctrl-C / SIGINT

**Browser Test Setup (using playwright-rust):**
```rust
// tests/browser_preview_test.rs
use playwright::{Playwright, expect};
use assert_cmd::Command;
use std::time::Duration;

#[tokio::test]
async fn test_browser_preview_opens() {
    // Start Playwright
    let playwright = Playwright::launch().await.unwrap();
    let browser = playwright.chromium().launch().await.unwrap();
    let page = browser.new_page().await.unwrap();

    // Start folio ingest in background thread
    let mut cmd = Command::cargo_bin("folio").unwrap();
    let _child = cmd
        .arg("ingest")
        .arg("--source").arg("test-data/fixtures")
        .arg("--dest").arg(temp_dir.path())
        .spawn()
        .unwrap();

    // Navigate to preview URL
    page.goto("http://localhost:8765").await.unwrap();

    // Verify page title (Playwright-style assertions)
    expect(page.locator("title"))
        .to_have_text("Folio Ingestion Preview")
        .await
        .unwrap();

    // Verify workflow steps exist
    expect(page.locator(".workflow-step"))
        .to_have_count(5)
        .await
        .unwrap();

    // Verify batch cards rendered
    let batches = page.locator(".batch-card");
    expect(batches).to_have_count(3).await.unwrap();

    // Cleanup
    browser.close().await.unwrap();
}

#[tokio::test]
async fn test_browser_updates_on_batch_name() {
    let playwright = Playwright::launch().await.unwrap();
    let browser = playwright.chromium().launch().await.unwrap();
    let page = browser.new_page().await.unwrap();

    // Start folio and navigate...
    page.goto("http://localhost:8765").await.unwrap();

    // Simulate user entering batch name in terminal
    // (Mock stdin or use subprocess communication)

    // Wait for update and verify
    tokio::time::sleep(Duration::from_millis(100)).await;

    expect(page.locator(".batch-card.complete .batch-name"))
        .to_have_text("test-event ✓")
        .await
        .unwrap();
}
```

**CI/CD Setup (.github/workflows/test.yml):**
```yaml
- name: Install Playwright browsers
  run: |
    # playwright-rust will handle Playwright server installation
    # Just ensure browsers are installed
    npx playwright install chromium

- name: Run browser tests
  run: cargo test --test browser_preview_test
```

**Documentation:**
- [ ] Document browser preview feature in README
- [ ] Add screenshots of browser preview to docs
- [ ] Document --headless flag for CI/automation
- [ ] Document WebSocket architecture for future extensions
- [ ] Add troubleshooting guide (firewall, port conflicts, etc.)

**Notes:**
- Keep preview server simple - single HTML page, no complex routing
- Embed CSS/JS inline (no external assets to serve)
- Use WebSocket for real-time updates (simpler than polling)
- Graceful degradation if browser closed mid-workflow
- Preview is enhancement to terminal workflow (terminal still primary)
- Thumbnails should be small (~200px max) to keep page fast

**Dependencies to add:**
- `warp` or `axum` - lightweight web server
- `tokio-tungstenite` or built-in WebSocket support
- `image` - thumbnail generation and resizing
- `base64` - encode thumbnails for HTML embedding
- `playwright-rust` (dev-dependency) - browser testing (BLOCKED: waiting for crates.io publish)
- `tokio` - async runtime (probably already have)

**Minimum playwright-rust requirements for this slice:**
- Launch Chromium browser
- Create page and navigate to URL
- Find elements by CSS selector
- Get text content from elements
- Count elements matching selector
- Playwright-style assertions (`expect().to_have_text()`, `expect().to_have_count()`)

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
- [ ] `extract_metadata()` - from DSLR JPEG with EXIF
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
| Slice 2: Timestamp & Folders | Must Have | Slice 1 | ✅ Completed | <1 day | Outside-in TDD + integration test first approach worked perfectly |
| Slice 3a: Agent Config & Nextest | Infrastructure | None | ✅ Completed | <1 day | Infrastructure slice, supports future development |
| Slice 3b: Temporal Batching & Naming | Must Have | Slice 2 | ✅ Completed | ~2 days | Terminal-based workflow, interactive prompts working perfectly |
| Slice 3c: Live Browser Preview | Must Have | Slice 3b + playwright-rust | **BLOCKED** | Est: 2-3 days | Waiting for playwright-rust v0.1 on crates.io |
| Slice 4: Metadata & XMP | Must Have | Slice 1 | Not Started | Est: 4-5 days | |
| Slice 5: Metadata Merging | Should Have | Slice 4 | Not Started | Est: 2-3 days | |
| Slice 6: Confirmations & Safety | Must Have | Slice 1-4 | Not Started | Est: 2-3 days | |

**Recommended order:** 1 → 2 → 3 → **[PAUSE for playwright-rust]** → 3.5 → 4 → 6 (then 5 if time)

**MVP:** Slices 1-4 + 6 deliver core backlog clearance value (3.5 blocked on playwright-rust)

**Development Strategy:**
- Complete Slice 3 (terminal-only workflow)
- Pause Folio development
- Build playwright-rust (8-10 weeks, separate project)
- Resume Folio with Slice 3.5 once playwright-rust published to crates.io
- Folio Slice 3.5 will drive playwright-rust feature priorities

**Total estimated time (Folio only):** 13-17 days (spread over ~12-14 weeks due to playwright-rust dependency)

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

**Slice 1:**
- **Test fixture generation requires external tools** - The `setup-test-fixtures.sh` script depends on ImageMagick (`convert`), `exiftool`, and `ffmpeg`. These need to be installed before running the setup script. This is acceptable for developer setup but worth documenting.
- **`Command::cargo_bin()` deprecated** - The `assert_cmd` example patterns use deprecated API. Updated to use `cargo_bin!` macro instead. This was caught by compiler warnings.
- **Video fixture files are larger than expected** - Even 1-second video files are ~12KB each (vs ~600 bytes for minimal JPEG). Total fixtures still well under 5MB limit, but worth noting for future fixture planning.

**Slice 2:**
- **Chrono `Datelike` trait import needed** - Initially forgot to import `Datelike` trait for `.year()`, `.month()`, `.day()` methods. Rust compiler provided clear error with suggestion.
- **Test fixtures needed update** - Existing tests expected files in archive root. Updated to use `WalkDir` to find files in date-based folder structure, making tests more flexible.
- **Dev dependency for walkdir in tests** - Had to add `walkdir` to `[dev-dependencies]` in `folio-cli/Cargo.toml` for integration tests.
- **Clippy warning on WalkDir iteration** - Pre-commit hook caught unnecessary `if let` pattern. Fixed by using `.into_iter().filter_map(Result::ok)` which is more idiomatic for filtering out errors from iterator.

### Adjustments Made to Plan

**Slice 1:**
- **Simplified error handling** - Used `anyhow::Result` instead of custom `FolioError` enum. For MVP, the flexibility and ergonomics of `anyhow` outweigh benefits of typed errors. Can add typed errors later if needed.
- **No separate hash module** - Integrated hashing directly into `media.rs`. The `blake3::Hash` type is sufficient; no need for custom wrapper type at this stage.
- **Deferred progress bars** - Simple file copying is fast enough without progress indication for test fixtures. Will add `indicatif` progress bars in later slice when processing larger batches.
- **Changed CLI flag name** - Used `--dest` instead of `--archive` for destination. Shorter and clearer in context of the command.
- **Skipped some planned tests** - Eliminated redundant tests (e.g., "different files produce different hashes" is implicit). Focused on essential test coverage.

**Slice 2:**
- **No separate `timestamp.rs` module** - Integrated timestamp functions directly into `media.rs`. Keeps related functionality together.
- **Simplified video metadata extraction** - Videos return None for timestamp, falling back to file modified date. More sophisticated video metadata extraction deferred to future enhancement.
- **Integration tests over unit tests** - Wrote integration test first (`test_ingest_organizes_by_date`), which drove implementation. Skipped redundant unit tests since integration test provides coverage.
- **Silent fallback for missing timestamps** - Files without EXIF use modified date silently rather than warning user. Better UX for mixed media batches.
- **Updated existing tests** - Made existing tests more flexible by using `WalkDir` to find files in any folder structure, rather than hardcoding paths.
- **Idiomatic iterator patterns** - Used `.into_iter().filter_map(Result::ok)` instead of nested `if let` for cleaner, more idiomatic Rust code that clippy approves of.

**Slice 3 (Completed 2025-11-05):**
- **Started with `--batch-name` flag** - Implemented basic filename generation first, deferring temporal batching. This allows immediate value (manual batch naming) while working toward automatic grouping.
- **No separate `batching.rs` module** - Added `TemporalBatch` struct and `group_by_temporal_proximity()` directly to `media.rs`. Keeps related functionality together.
- **Missing `Timelike` trait initially** - Forgot to import `chrono::Timelike` for `.hour()`, `.minute()`, `.second()` methods. Compiler error was clear and quick to fix.
- **chrono not in CLI dependencies** - Had to add `chrono.workspace = true` to `folio-cli/Cargo.toml` to use `Utc::now()` for timestamp fallback and `Duration` for gap threshold.
- **Simple timestamp fallback** - If no EXIF timestamp, falls back to file modified date, then `Utc::now()` as last resort. Ensures all files get valid filenames.
- **Preserves original extension case** - Uses `.extension().and_then(|e| e.to_str())` to preserve case of original extension (.JPG vs .jpg).
- **Integration test drives implementation** - Wrote `test_ingest_with_batch_name` first with two fixtures having different EXIF timestamps. Test verifies both timestamp extraction and filename format.
- **Temporal batching implementation** - Wrote 4 unit tests first (Red), then implemented `group_by_temporal_proximity()` (Green). Filters items without timestamps, sorts by time, groups by gap threshold.
- **Gap threshold conversion** - Convert hours (f64) to `Duration::seconds()` for chrono compatibility: `Duration::seconds((hours * 3600.0) as i64)`.
- **Flag behavior logic (Option 1)** - Resolved ambiguity between `--batch-name` and `--gap-threshold`:
  - `--batch-name` present → **single batch mode**, temporal batching disabled, all files get same name
  - NO `--batch-name` → **interactive mode**, detects batches with `--gap-threshold`, prompts for each batch name
  - Clear semantics: `--batch-name` = "I know this is one event", NO flag = "detect events for me"
- **Updated all tests** - Fixed 3 tests that were failing because they didn't provide `--batch-name`. All tests now enforce the new logic.
- **Batch name validation** - Implemented `validate_batch_name()` with 9 comprehensive unit tests covering:
  - Valid names (alphanumeric, hyphens, underscores)
  - Invalid names (spaces, special chars, empty, only punctuation)
  - Edge cases (unicode, leading/trailing, mixed case)
- **Interactive prompts implemented** - Created `prompt_for_batch_name()` helper function:
  - Displays batch info (date range, file count, photo/video breakdown)
  - Shows first 3 filenames as samples
  - Prompts user for batch name
  - Validates input and re-prompts on error with helpful message
  - Loops until valid name provided
- **Borrow checker fix** - Captured `batches.len()` as `total_batches` before calling `into_iter()` to avoid borrow after move error.
- **Refactored file copying** - Changed from iterating over individual items to iterating over `Vec<(TemporalBatch, String)>` pairs, enabling each batch to have its own name.
- **Three operating modes** - Successfully implemented and tested:
  1. **Single batch mode**: `--batch-name my-event` (all files get same name)
  2. **Interactive mode**: No flags (prompts for each detected batch)
  3. **Dry-run mode**: `--dry-run` (uses placeholder names like `batch-1`, `batch-2`)
- **Testing interactive CLIs** - Discovered `assert_cmd::Command::write_stdin()` method for mocking user input:
  - Write simulated keyboard input as string (e.g., `"morning\nafternoon\n"`)
  - Test verifies prompts displayed and correct batch names used
  - Can test validation by providing invalid input first, then valid (e.g., `"invalid name\nvalid-name\n"`)
- **Added 2 interactive tests**:
  - `test_ingest_interactive_mode_with_valid_input` - Mocks 2 batch names, verifies both batches named correctly
  - `test_ingest_interactive_mode_with_invalid_then_valid_input` - Tests validation re-prompting
- **Final test count: 32 tests** - All passing:
  - 11 CLI integration tests (including 2 new interactive tests)
  - 18 folio-core unit tests (including 4 temporal batching + 9 validation tests)
  - 2 doc tests
  - 1 other test
- **Deferred filename collision handling** - Decided to defer the `-001`, `-002` sequence appending to future enhancement. It's a rare edge case (same timestamp + same batch name) and low priority for MVP.

**Slice 3a (Infrastructure - Completed 2025-11-14):**
- **Specialized agents accelerate development** - Having domain-specific agents (TDD, Docs, Release) with project context reduces cognitive load and ensures consistency.
- **Agent adaptation from other projects** - Successfully adapted agents from playwright-rust to Folio by:
  - Replacing "phases" with "user stories + implementation plans + slices"
  - Emphasizing vendor neutrality and family-first design in all agents
  - Customizing TDD agent for outside-in approach and real media validation
- **Nextest benefits** - Fast parallel test execution (vs sequential `cargo test`) will accelerate TDD workflow.
- **Nextest retry logic** - Configuring `retries = 2` provides resilience for potentially flaky I/O tests (filesystem, EXIF reading).
- **Doctest strategy clarified** - Module-level consolidation with `no_run` prevents slow doctests while maintaining executable documentation.
- **Nextest doesn't run doctests** - Must run `cargo test --doc` separately. Updated all CLAUDE.md guidance to use both: `cargo nextest run --workspace && cargo test --doc --workspace`.
- **Configuration as code** - Storing agent configurations in `.claude/agents/` and nextest config in `.config/nextest.toml` makes them version-controlled and shareable.

### Lessons for Future Features

- **Start simple, iterate** - The initial plan had more complexity than needed. Starting with the simplest implementation that passes tests was faster and easier to understand.
- **Interactive CLIs are testable** - Using `assert_cmd::Command::write_stdin()` to mock user input enables fully automated testing of interactive prompts. No need to skip testing interactive features.
- **Design flag semantics carefully** - The ambiguity between `--batch-name` and `--gap-threshold` required explicit design discussion. Clear flag semantics (Option 1: `--batch-name` disables temporal batching) prevents user confusion.
- **Validation with re-prompting** - Interactive prompts should validate input and loop until valid, with helpful error messages. This is better UX than failing the entire operation on first invalid input.
- **Defer low-priority edge cases** - Filename collision handling (`-001`, `-002` sequences) is a rare edge case. Deferring to future enhancement keeps focus on core functionality.
- **Trust the test-driven process** - Writing failing tests first really does clarify requirements and drive clean API design. Don't skip the "red" phase.
- **Keep test fixtures committed** - Having test fixtures in git makes tests reproducible across machines and CI environments. The setup script makes regeneration easy if needed.
- **Integration tests are invaluable** - CLI integration tests caught issues that unit tests wouldn't (like output formatting, exit codes, dry-run logic).
- **Defer optimization** - No progress bars, no parallel copying, no custom error types in Slice 1. These can be added when needed. Focus on working end-to-end functionality first.
- **Document setup requirements** - External tool dependencies (ImageMagick, exiftool, ffmpeg) need clear documentation for new contributors.
- **Pre-commit hooks catch issues early** - Clippy warnings caught at pre-commit stage prevent CI failures and maintain code quality. The `.into_iter().filter_map(Result::ok)` pattern is more idiomatic than nested `if let` when you only care about `Ok` values.
- **Windows CI compatibility matters** - Cross-platform testing revealed ImageMagick command differences. Prioritizing `magick` over `convert` fixed Windows issues while maintaining macOS/Linux compatibility.
