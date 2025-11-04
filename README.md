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

**Phase 0: Foundation** (Current)
- [x] Architecture Decision Record (ADR-0001)
- [x] Rust workspace setup
- [ ] XMP compatibility proof-of-concept
- [ ] Define folder structure conventions

**Phase 1: Backlog Clearance** (Next - 2-4 weeks)
- [ ] SD card → archive ingestion
- [ ] Hash-based deduplication
- [ ] EXIF extraction → XMP sidecar generation

## Development

### Build

```bash
cargo build
```

### Test

```bash
cargo test --workspace
```

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
