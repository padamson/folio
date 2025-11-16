# Folio

**Family media archive workflow tools**

Folio is a Rust-based toolkit for managing a vendor-neutral family photo and video archive. Built to help clear digital asset backlogs while establishing efficient, tool-agnostic workflows.

## Philosophy

- **Workflows over UIs** - Build automation tools, use existing viewers
- **Vendor neutrality** - Metadata in open standards (XMP sidecars), filesystem as source of truth
- **Tool flexibility** - Use Lightroom, digiKam, Urocissa, or any XMP-compatible tool
- **Rust-first** - Performance and safety

## Architecture

See [ADR-0001: Metadata and Catalog Architecture](docs/adr/0001-metadata-catalog-architecture.md) for the complete architectural decision.

**Key principles:**
- Filesystem + XMP sidecars as source of truth
- Custom Rust workflows for ingestion, deduplication, metadata management
- Interchangeable viewers (Lightroom, Urocissa, digiKam, PhotoPrism)
- No vendor lock-in

## Project Structure

```
folio/
├── crates/
│   ├── folio-core/      # Core library (metadata, types, utilities)
│   ├── folio-cli/       # CLI binary
│   └── folio-ingest/    # Photo/video ingestion workflows
├── docs/
│   ├── adr/             # Architecture Decision Records
│   ├── current-state.md # Current system analysis
│   └── key-insights.md  # Strategic insights and priorities
└── test-data/
    └── fixtures/        # Safe sample files for testing
```

## Installation

```bash
cargo build --release
```

The `folio` binary will be at `target/release/folio`.

## Usage

```bash
# Ingest photos from SD card
folio ingest --source /Volumes/SD_CARD/DCIM --dest /archive/2025/2025-01-01_event

# Find duplicates (dry run)
folio dedupe --archive /archive/2024 --dry-run

# Show help
folio --help
```

## Current Status

**Phase 1: Backlog Ingestion** (In Progress - Slices 1-3c Complete)

Completed:
- [x] Media discovery and copy (scan directories, hash-based deduplication)
- [x] EXIF timestamp extraction and date-based folder organization
- [x] Temporal batch grouping (group photos by time gaps)
- [x] Interactive batch naming workflow
- [x] Live browser preview dashboard with real-time WebSocket updates

Next:
- [ ] XMP sidecar metadata generation
- [ ] Metadata merging strategies
- [ ] User confirmation and safety checks

See [implementation plan](./docs/implementation-plans/001-backlog-ingestion-plan.md) for details.

## Development

### Build

```bash
cargo build
```

### Test

```bash
cargo test --workspace
```

**First-time setup for browser tests:**

Browser preview tests require Playwright browsers. After your first `cargo build`, install them once:

```bash
# Linux
~/.cache/playwright-rust/drivers/playwright-1.56.1-linux/node \
  ~/.cache/playwright-rust/drivers/playwright-1.56.1-linux/package/cli.js \
  install chromium firefox webkit --with-deps

# macOS (Intel)
~/Library/Caches/playwright-rust/drivers/playwright-1.56.1-mac/node \
  ~/Library/Caches/playwright-rust/drivers/playwright-1.56.1-mac/package/cli.js \
  install chromium firefox webkit

# macOS (ARM64)
~/Library/Caches/playwright-rust/drivers/playwright-1.56.1-mac-arm64/node \
  ~/Library/Caches/playwright-rust/drivers/playwright-1.56.1-mac-arm64/package/cli.js \
  install chromium firefox webkit

# Windows (PowerShell)
& "$env:LOCALAPPDATA\playwright-rust\drivers\playwright-1.56.1-win32_x64\node.exe" `
  "$env:LOCALAPPDATA\playwright-rust\drivers\playwright-1.56.1-win32_x64\package\cli.js" `
  install chromium firefox webkit
```

This downloads ~500MB of browser binaries to your OS cache directory (one-time per machine).

### Run CLI

```bash
cargo run --bin folio -- --help
```

### Format and Lint

```bash
cargo fmt
cargo clippy -- -D warnings
```

## Documentation

- [ADR-0001: Metadata and Catalog Architecture](docs/adr/0001-metadata-catalog-architecture.md)
- Backlog ingestion [user story](./docs/user-stories/001-backlog-ingestion.md) and [implementation plan](./docs/implementation-plans/001-backlog-ingestion-plan.md)
- [CLAUDE.md](CLAUDE.md) - Development guidance for Claude Code

## License

MIT OR Apache-2.0
