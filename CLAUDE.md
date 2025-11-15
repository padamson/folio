# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Folio is a comprehensive, vendor-neutral system for organizing, archiving, and sharing family videos, photos, and other media. Built primarily in **Rust** for performance, safety, and reliability.

### Vision and Design Philosophy

**Problem**: Family media (photos, videos, documents) accumulates across devices, cloud services, and storage locations without consistent organization, backup, or sharing mechanisms. Vendor lock-in (Adobe Lightroom, Apple Aperture) creates fragility when tools change. Families need a unified, tool-agnostic system to preserve memories and share them with loved ones.

**Solution**: Folio provides integrated tools, workflows, and infrastructure for managing the complete lifecycle of family media - from capture to organization, archiving, and sharing - with a vendor-neutral foundation.

**Key Principles:**
- **Vendor-neutral** - Own your data, use any editing tool
- **Family-first design** - Optimized for non-technical family members to access and enjoy
- **Preservation-focused** - Reliable archiving and backup strategies with open formats
- **Privacy-aware** - Control over who accesses what content
- **Performance-first** - Rust-based tools for speed and reliability
- **Open standards** - File-based + sidecar metadata (XMP, JSON)
- **Future-proof** - No proprietary lock-in, survives vendor changes

**Strategic Positioning:**
- **Tool-agnostic foundation**: File-based organization + open metadata standards
- **Rust-first implementation**: Performance-critical tools in Rust (CLI, processing, serving)
- **Hybrid approach**: Use any editing tool (Lightroom, Darktable, Digikam, etc.)
- **Flexible architecture**: Mix of Rust CLI tools, web platform, and scripts
- **Pragmatic evolution**: Start with core tools, evolve to full platform

### Technology Stack

**Primary Development Language: Rust**
- **Why Rust**: Memory safety, performance, concurrency, reliability for family data
- **Use cases**: CLI tools, media processing, metadata management, web services
- **Supporting languages**: Shell scripts (automation), TypeScript (web frontend), Python (AI/ML integrations)

### Project Structure

This project is organized around functional domains:

```
folio/
├── crates/                      # Rust workspace
│   ├── media-core/             # Core library for media operations
│   ├── media-cli/              # CLI for media management
│   ├── media-ingest/           # Ingestion tools (mobile, DSLR, etc.)
│   ├── media-catalog/          # Metadata catalog management
│   ├── media-server/           # Web API server
│   └── media-processor/        # Image/video processing
├── docs/                       # Planning and architecture
│   ├── user-stories/           # User stories and implementation plans
│   ├── templates/              # Planning templates
│   ├── adr/                    # Architecture Decision Records
│   └── architecture/           # Architecture diagrams
├── scripts/                    # Helper scripts (bash, python)
├── web/                        # Web frontend (TypeScript/React)
├── test-data/                  # Test fixtures
└── infrastructure/             # Deployment configs

```

**Workspace Structure:**
- Root `Cargo.toml` defines workspace-wide dependencies
- Each crate has focused responsibility
- `media-core` is the foundation library
- Other crates depend on `media-core` for consistency

**Future extensibility:** New crates can be added as needs grow (e.g., `media-ai` for ML features, `media-mobile` for mobile app, `media-backup` for backup orchestration).

## Development Approach

This project uses an **incremental, value-driven approach** where each component delivers immediate value to the family. Development prioritizes practical solutions over perfectionism, with an emphasis on reliability and maintainability.

### Specialized Development Agents

For complex workflows, use these specialized agents (located in `.claude/agents/`):

1. **TDD Feature Implementation Agent** (`tdd-feature-implementation.md`)
   - Use when: Implementing any new media processing feature
   - Automates: Red → Green → Refactor workflow, outside-in TDD, real media validation
   - Ensures: Tests written first, vendor neutrality maintained, rustdoc complete

2. **Documentation Maintenance Agent** (`documentation-maintenance.md`)
   - Use when: Completing slices/features, updating docs, releasing versions
   - Automates: Just-in-time doc updates, hierarchy enforcement, CHANGELOG generation
   - Ensures: README shows current features only, roadmap stays strategic, implementation plans stay current

3. **Release Preparation Agent** (`release-preparation.md`)
   - Use when: Preparing a version release (version bump, CHANGELOG, verification)
   - Automates: Pre-release checks, test verification (nextest), version management, validation
   - Ensures: All quality gates pass, CHANGELOG is complete, release process is smooth

#### Automatic Agent Invocation

**IMPORTANT**: Proactively use agents when user requests match these patterns:

**TDD Feature Implementation Agent** - Use automatically when user:
- Says "implement {feature}" or "add {functionality}"
- Mentions implementing media processing (metadata extraction, ingestion, XMP generation, etc.)
- Asks to "create a new feature" or "add support for"
- Example triggers: "implement EXIF extraction", "add video metadata support", "create XMP sidecar generation"

**Documentation Maintenance Agent** - Use automatically when user:
- Says "Slice X complete", "Feature Y done", or "finished Slice Z"
- Asks to "update documentation" or "update docs"
- Mentions "release" or "preparing for release"
- Says "feature complete" or "slice finished"
- Example triggers: "Slice 3 complete", "backlog ingestion done", "update docs for metadata extraction"

**Release Preparation Agent** - Use automatically when user:
- Says "prepare release" or "release v{X.Y.Z}"
- Asks to "publish to crates.io" or "publish version {X.Y.Z}"
- Says "ready to release" or "let's release"
- Mentions "version bump" in context of releasing
- Example triggers: "prepare release for v0.1.0", "publish v0.1.0 to crates.io", "ready to release"

**Don't use agents for**:
- Simple single-file edits
- Reading files or searching code
- Running a single test
- Quick bug fixes (< 10 lines)
- Formatting or clippy fixes

**General rule**: If the task requires 3+ steps or involves multiple components (core library, CLI, tests, docs), proactively use the appropriate agent.

### Planning and Documentation Structure

This project uses structured planning documents to guide development:

1. **User Stories** (`docs/user-stories/story-*.md`)
   - Define WHAT family members need and WHY
   - Written from family user perspective (both technical and non-technical users)
   - Include acceptance criteria and technical requirements
   - Use template: `docs/templates/TEMPLATE_USER_STORIES.md`

2. **Implementation Plans** (`docs/user-stories/plan-*.md`)
   - Define HOW to implement features
   - Break work into incremental, deliverable components
   - Track progress with checklists
   - Include "Definition of Done" for each feature
   - Use template: `docs/templates/TEMPLATE_IMPLEMENTATION_PLAN.md`

3. **Architecture Decision Records** (`docs/adr/####-*.md`)
   - Document significant architectural and infrastructure decisions
   - Compare options with trade-off analysis (cost, complexity, maintainability)
   - Record rationale for future reference
   - Use template: `docs/templates/TEMPLATE_ADR.md`

4. **Workflow Diagrams** (`docs/workflows/`)
   - Visual representations of media workflows (ingestion, processing, backup, sharing)
   - Document automation sequences and dependencies
   - Include error handling and recovery procedures

### Working on Features

**IMPORTANT**: Always check `docs/user-stories/` for current implementation status and requirements.

**When starting work:**
1. **Check `docs/user-stories/`** to identify the current feature being worked on
2. **Read the user story** (`story-*.md`) for requirements and acceptance criteria
3. **Follow the implementation plan** (`plan-*.md`) for incremental approach and technical details
4. **Check architecture diagrams** in `docs/architecture/` for system context
5. **Update status** in the plan as you complete each item (check boxes, update status fields)

**When implementing features:**
1. Start with the **minimum viable implementation** that delivers value
2. Each **increment** should be testable and independently useful
3. Follow the implementation plan's sequence when specified
4. Consider dependencies between components (storage, processing, sharing)
5. Test with **real family media** (photos, videos) in safe test environments
6. Document setup and usage for family members

**Status tracking - you MUST update the planning documents:**
- Check off completed acceptance criteria in the user story
- Mark components as "In Progress" or "Completed" in the implementation plan
- Check off completed implementation items within each component
- Update the "Definition of Done" checklist as items are completed
- Document any infrastructure changes or new dependencies

### Testing and Validation

Testing for this project varies by component type:

**For Scripts (Python, Bash, etc.):**
1. **Manual Testing** - Run scripts with sample media files
2. **Dry-run modes** - Implement `--dry-run` or `--test` flags where possible
3. **Unit tests** - Test utility functions and core logic
4. **Integration tests** - Test with realistic media files in test directories
5. **Validation** - Verify outputs, check logs, confirm files created/modified

**For Infrastructure (Docker, configs, etc.):**
1. **Local testing** - Deploy to local environment first
2. **Smoke tests** - Verify services start and are accessible
3. **Validation scripts** - Check configurations are applied correctly
4. **Rollback plan** - Document how to revert changes if needed

**For Rust Code (Primary Development):**
1. **Outside-in TDD** - Integration test first, then unit tests (see below)
2. **Unit tests** - Test individual functions, error paths
3. **Integration tests** - Test crate interactions
4. **CLI tests** - Use `assert_cmd` for end-to-end CLI testing
5. **Property-based tests** - Use `proptest` for edge cases

**For Web Applications (future):**
1. **Unit tests** - Test components, utilities, business logic
2. **Integration tests** - Test API endpoints, database operations
3. **E2E tests** - Test user workflows with real browsers
4. **Manual testing** - Family members test features before release

**Testing Principles:**
- Test with **real family media** (copies in test directories)
- **Never test on original files** - always work on copies
- **Document test procedures** - make it easy for future you to validate
- **Version control configs** - track infrastructure changes in git
- **Incremental rollout** - test locally, then staging, then production

### Outside-in TDD Workflow for Rust

**This project follows strict Test-Driven Development (TDD) for all Rust code.**

For each feature, follow this workflow:

1. **Integration Test First (Red)** - Write failing test defining user interaction
   - For CLI: Use `assert_cmd` to test command usage, output, exit codes
   - For Library: Write test showing public API usage
   - Always test: happy path, error cases, help text (for CLI), exit codes

2. **Unit Tests (Red)** - Write failing unit tests for internal modules
   - Test edge cases
   - Test error conditions
   - Test validation logic

3. **Implement Core Logic (Green)** - Make unit tests pass with minimal code
   - Write simplest code that makes tests pass
   - Avoid over-engineering
   - Focus on one test at a time

4. **Wire Up Integration (Green)** - Make integration tests pass
   - Connect components
   - Verify end-to-end behavior

5. **Refactor** - Clean up code, extract modules, improve design
   - Maintain test coverage
   - Improve code structure without changing behavior
   - Run `cargo clippy` for linting

6. **Document** - Update docs, add examples
   - Rustdoc comments for public APIs
   - `# Examples` sections
   - Update README if API changed

7. **Commit** - Small, focused commits
   - Each commit should make one test pass
   - Clear commit messages

**Red-Green-Refactor Discipline:**
- **Red**: Write the smallest failing test that captures the requirement
- **Green**: Write the simplest code to make the test pass (avoid over-engineering)
- **Refactor**: Improve code structure without changing behavior
- Run `cargo nextest run` frequently (after every small change)
- Run doctests separately: `cargo test --doc`
- Run `cargo clippy` and `cargo fmt` before committing
- **Never write code without a failing test first**
- Keep each commit small and focused on making one test pass

**Example Test Patterns:**

```rust
// CLI integration test - tests user experience end-to-end
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_ingest_photos_from_directory() {
    let mut cmd = Command::cargo_bin("media-cli").unwrap();

    cmd.arg("ingest")
        .arg("--source")
        .arg("test-data/fixtures/sample-photos/")
        .arg("--destination")
        .arg("test-data/output/")
        .assert()
        .success()
        .stdout(predicate::str::contains("Ingested 5 photos"))
        .stdout(predicate::str::contains("0 duplicates skipped"));
}

// Library unit test - tests individual function behavior
#[test]
fn test_parse_jpeg_exif_metadata() {
    let metadata = parse_exif("test-data/fixtures/test-photo.jpg").unwrap();
    assert_eq!(metadata.camera_model, "NIKON D800");
    assert_eq!(metadata.date_taken, "2024-01-15");
}

// Error case test - verify error handling
#[test]
fn test_parse_invalid_image_returns_error() {
    let result = parse_exif("test-data/fixtures/corrupt.jpg");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ParseError::InvalidFormat(_)));
}
```

**Test Organization:**

```
crates/media-core/
├── src/
│   ├── lib.rs           # Inline unit tests with #[cfg(test)]
│   ├── metadata.rs      # Inline unit tests
│   └── catalog.rs       # Inline unit tests
├── tests/
│   ├── fixtures/        # Test media files
│   │   ├── test-photo.jpg
│   │   ├── test-photo.xmp
│   │   └── test-video.mp4
│   └── integration.rs   # Integration tests
└── benches/
    └── benchmarks.rs    # Performance benchmarks
```

### Test Data Strategy - Three Tiers

Folio uses a three-tier test data strategy to balance speed, realism, and safety:

**1. Unit Tests - Minimal Fixtures (Always)**
- **Location**: `crates/*/tests/fixtures/`
- **Created by**: `scripts/create_test_fixtures.sh`
- **Size**: Tiny (KB) - minimal valid media files + metadata
- **Purpose**: Fast, deterministic unit tests
- **When**: Every commit, all CI runs
- **Example**:
  - `test-photo.jpg` (10x10 pixel JPEG)
  - `test-photo.xmp` (sample XMP metadata)
  - `test-video.mp4` (1-second video clip)

**2. Integration Tests - Realistic Samples (Local Dev)**
- **Location**: `test-data/integration/`
- **Created by**: `scripts/setup_test_data.sh`
- **Size**: Small (MB) - realistic but small media files
- **Purpose**: Test with realistic formats and sizes
- **When**: Local integration testing
- **Example**:
  - Various JPEG sizes (2-20 MB)
  - Different video formats (MP4, MOV, M4V)
  - Multiple metadata formats (XMP, EXIF, JSON)

**3. Manual Tests - Sanitized Family Media (Optional)**
- **Location**: `test-data/personal/` (gitignored, never committed)
- **Content**: Copies of real family media for realistic testing
- **Purpose**: Test with actual photo library characteristics
- **When**: Manual testing only, before major releases
- **Example**: Copy of 100 photos from family archive

**Safety Principles:**
- **Never commit real family media** to version control
- **Always use copies** for testing, never originals
- **Gitignore sensitive paths**: `test-data/personal/`, `*.personal.*`
- **Automate fixture generation**: Scripts to create valid test files
- **Document test data**: Sources and recreation procedures

**Test Pyramid:**

```
        / \
       /E2E\     E2E Tests (full workflow with fixtures)
      /_____\      - CLI command workflows
     / Int   \    Integration Tests (realistic files)
    /_________\     - Multi-crate interactions
   /   Unit    \   Unit Tests (minimal fixtures)
  /_____________\    - Individual functions, error cases
```

### Documentation Testing Strategy (Doctests)

**Philosophy: Executable, Verified Documentation**

Folio uses **executable doctests with assertions** to ensure documentation stays synchronized with implementation:

1. **Doctests must run** - No `no_run` annotation unless absolutely necessary (e.g., destructive operations)
2. **Doctests must assert** - Include assertions to verify behavior, not just demonstrate usage
3. **Use test fixtures** - Reference files in `test-data/fixtures/` so doctests work reliably
4. **Prevent documentation drift** - If the API changes, doctests fail, forcing docs to update
5. **Separate execution** - Use `cargo test --doc` to run doctests (nextest doesn't run them)

**Doctest Structure (GOOD - runs and verifies):**

```rust
//! Media metadata extraction module
//!
//! Extracts EXIF metadata from photos and videos for vendor-neutral organization.
//!
//! # Example
//!
//! ```
//! use folio_core::metadata::extract_metadata;
//!
//! // Use test fixtures so doctest can actually run
//! let jpeg_meta = extract_metadata("test-data/fixtures/sample.jpg")?;
//!
//! // Assert expected behavior to prevent drift
//! assert_eq!(jpeg_meta.camera_model, "NIKON D800");
//! assert!(jpeg_meta.date_taken.is_some());
//! assert_eq!(jpeg_meta.width, 800);
//! assert_eq!(jpeg_meta.height, 600);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::path::Path;

pub fn extract_metadata(path: impl AsRef<Path>) -> Result<Metadata> {
    // implementation...
}
```

**Doctest Structure (BAD - doesn't run or verify):**

```rust
// ❌ DON'T USE `no_run` unless absolutely necessary
//! ```no_run
//! let meta = extract_metadata("some/path.jpg")?;
//! println!("{:?}", meta);  // ❌ No assertions
//! ```
```

**Key principles:**
- **Run by default** - Only use `no_run` for destructive operations or external dependencies
- **Assert behavior** - Use `assert_eq!`, `assert!`, etc. to verify correctness
- **Use fixtures** - Reference `test-data/fixtures/` files that exist in the repository
- **Show realistic usage** - Demonstrate actual media processing workflows
- **Vendor neutrality** - Examples should emphasize XMP, open formats

**Running Doctests:**

```bash
# Run all doctests
cargo test --doc --workspace

# Run doctests for specific crate
cargo test --doc -p folio-core

# Run both nextest and doctests
cargo nextest run --workspace && cargo test --doc --workspace
```

## Documentation

### Documentation Philosophy

**Separation of Concerns:**
- **Markdown documentation** explains the **WHY** (rationale, strategy, decisions)
- **Self-documenting code/scripts** capture the **WHAT** and **HOW** (implementation details)

**Best Practices:**
- Keep markdown docs high-level and focused on reasoning
- Point to actual implementations from docs
- Use comments liberally in scripts and configs
- Keep docs up-to-date with implementation via checklists
- Update docs when infrastructure or workflows change
- **Document for future you** - assume you'll forget details in 6 months

**Example Structure:**
```
docs/architecture/storage-strategy.md   ← WHY: Strategy, reasoning, benefits
    ↓ points to
infrastructure/storage/setup.sh         ← WHAT/HOW: Actual implementation
    ↓ which is
Self-documenting with comments          ← HOW: Implementation details
```

### Documentation Structure

- **User stories and implementation plans** - `docs/user-stories/`
- **Planning templates** - `docs/templates/`
- **Architecture decisions** - `docs/adr/` (infrastructure, storage, tech choices)
- **Architecture diagrams** - `docs/architecture/` (system diagrams, workflows)
- **Workflow documentation** - `docs/workflows/` (process flows, automation sequences)
- **Setup guides** - `docs/setup/` (how to deploy, configure, maintain)
- **Family user guides** - `docs/user-guides/` (for non-technical family members)

### README Files

Each major directory should have a README explaining:
- **Purpose** - What this component does
- **Dependencies** - What's required to run it
- **Setup** - How to configure and deploy
- **Usage** - How to use it (with examples)
- **Troubleshooting** - Common issues and solutions
- **Maintenance** - Regular tasks, monitoring, backups

Examples:
- `scripts/README.md` - Overview of automation scripts
- `infrastructure/README.md` - Infrastructure setup and management
- `docs/workflows/README.md` - Overview of documented workflows

### Code/Script Documentation

**For Python scripts:**
- Docstrings for all functions and classes
- Inline comments for complex logic
- Type hints where helpful
- Usage examples in module docstring

**For Bash scripts:**
- Header comment explaining purpose and usage
- Comments for each major section
- Variable descriptions
- Example usage in header

**For Configuration files:**
- Comments explaining each setting
- Examples of valid values
- Links to relevant documentation
- Warnings about critical settings

## Versioning and Change Management

### Version Control Strategy

**What to track in git:**
- All source code (scripts, web applications, etc.)
- Infrastructure as code (Docker configs, deployment scripts)
- Configuration files (sanitized - no secrets!)
- Documentation
- Workflow diagrams and architecture documents

**What NOT to track in git:**
- Media files (photos, videos)
- Secrets and credentials
- Large binary files
- Temporary test data
- Personal/sensitive information

### Rollback Strategy

Since this is infrastructure and automation (not published packages):

**For Scripts:**
1. **Git revert** - Undo problematic commits
2. **Test rollback** - Verify older version works
3. **Document incident** - What went wrong, how you fixed it

**For Infrastructure:**
1. **Snapshot before changes** - Take backups before deploying
2. **Keep previous configs** - Version control or backup old configs
3. **Test restore procedure** - Verify you can roll back
4. **Document rollback steps** - Write them down while deploying

**For Data Operations:**
1. **Never modify originals** - Always work on copies
2. **Keep audit logs** - Track what changed when
3. **Backup before bulk operations** - Snapshot before reorganizing
4. **Test restore** - Verify backups work regularly

## Deployment Strategy

### Environments

1. **Local Development**
   - Test scripts on local machine
   - Use test data, never production media
   - Iterate quickly with small changes

2. **Staging/Test Environment**
   - Isolated environment with copy of production data
   - Test workflows end-to-end before production
   - Validate backups and restore procedures

3. **Production Environment**
   - Live family media system
   - Monitored and backed up
   - Changes deployed carefully after staging validation

### Deployment Principles

**Incremental deployment:**
1. **Test locally** with synthetic data
2. **Test in staging** with production copy
3. **Deploy to production** during low-activity times
4. **Monitor** for issues after deployment
5. **Validate** everything works as expected

**Infrastructure changes:**
- **Document before deploying** - Write setup guide as you go
- **Backup before changing** - Snapshot VMs, backup configs
- **Deploy incrementally** - One component at a time
- **Keep old version running** - Don't tear down until new works
- **Test thoroughly** - Verify all workflows still work

## Monitoring and Observability

### What to Monitor

**System Health:**
- Storage space (disk usage on media storage)
- Backup status (last successful backup, backup age)
- Service uptime (web platform, automation services)
- Resource usage (CPU, memory, network for media servers)

**Media Workflows:**
- Ingestion success/failure rates
- Processing times (video transcoding, photo optimization)
- Backup completion status
- Sharing activity (who's accessing what)

**Infrastructure:**
- Container/VM health
- Network connectivity
- Certificate expiration (for HTTPS)
- Security alerts

### Logging Strategy

**What to log:**
- All automated operations (start, success, failure)
- File modifications (what changed, when, why)
- Errors and warnings with context
- User actions (for family platform access logs)
- Backup operations and results

**Where to log:**
- **Script logs**: Write to files in `logs/` directory
- **System logs**: Use syslog for infrastructure events
- **Centralized logging**: Consider aggregating to single location
- **Retention**: Keep logs for reasonable period (30-90 days)

**Best practices:**
- **Timestamp everything** - Use ISO 8601 format
- **Include context** - Script name, operation, file being processed
- **No sensitive data** - Don't log full file paths if privacy concern
- **Rotate logs** - Don't fill up disk with old logs
- **Make searchable** - Use consistent format for easy grep/parsing

### Alerting

**When to alert:**
- Backup failures
- Storage space low (< 10% free)
- Service downtime
- Security events
- Unusual activity patterns

**How to alert:**
- Email notifications for critical issues
- Log files for routine events
- Consider simple monitoring dashboard
- SMS/push for urgent issues (optional)

### Health Checks

**Regular checks:**
- Daily: Verify backups completed successfully
- Weekly: Check storage space trends
- Monthly: Test restore procedures
- Quarterly: Review security and update dependencies

**Automation:**
- Create health check scripts that can run on schedule
- Report status summary (green/yellow/red)
- Log check results for trend analysis

## Performance Considerations

### Performance Goals

| Area | Goal | Why |
|------|------|-----|
| **Ingestion** | Handle batch photo uploads efficiently | Family uploads 100s of photos at once |
| **Backup** | Complete without impacting family usage | Run backups during off-hours |
| **Web Platform** | Fast browsing, smooth thumbnail loading | Non-technical family members expect responsive UI |
| **Video Processing** | Reasonable transcoding times | Don't make family wait days for processed videos |
| **Storage** | Efficient space usage | Balance quality vs. storage costs |

### Optimization Strategies

**For Scripts:**
- **Batch operations** - Process multiple files together
- **Parallel processing** - Use multiple cores for independent tasks
- **Incremental processing** - Only process new/changed files
- **Efficient formats** - Use appropriate compression for media type
- **Skip unnecessary work** - Check if output already exists

**For Storage:**
- **Tiered storage** - Frequently accessed on fast storage, archives on slow/cheap
- **Deduplication** - Identify and remove duplicate media files
- **Compression** - Use appropriate compression (lossless for originals, lossy for derivatives)
- **Cleanup old derivatives** - Remove old thumbnails/transcodes when regenerating

**For Web Platform:**
- **CDN/caching** - Serve static media efficiently
- **Lazy loading** - Load images as user scrolls
- **Progressive images** - Show low-res first, then high-res
- **Optimized thumbnails** - Pre-generate appropriate sizes
- **Database indexing** - Fast metadata queries

## Security Considerations

### Access Control

**For Family Web Platform:**
- **Authentication** - Secure login for family members only
- **Authorization** - Control who can view/upload/delete media
- **Privacy settings** - Control visibility of sensitive content
- **Session management** - Secure session tokens, reasonable timeouts
- **HTTPS only** - Encrypt all traffic

**For Storage:**
- **File permissions** - Restrict access to media files
- **Network security** - Firewall rules for storage servers
- **Encryption at rest** - Consider encrypting sensitive media
- **Backup encryption** - Encrypt backup media

### Input Validation

**For Scripts:**
- **Validate file paths** - Prevent path traversal attacks
- **Check file types** - Verify files are actually media before processing
- **Size limits** - Prevent processing of unreasonably large files
- **Sanitize filenames** - Handle special characters safely
- **Validate metadata** - Don't trust EXIF/metadata blindly

**For Web Platform:**
- **File upload validation** - Check file types, sizes, content
- **User input sanitization** - Prevent XSS, SQL injection
- **Rate limiting** - Prevent abuse of upload/processing
- **CSRF protection** - Use tokens for state-changing operations

### Secrets Management

- **Never commit secrets** - Use `.gitignore` for credentials
- **Environment variables** - Store credentials outside code
- **Secret managers** - Consider using proper secret management tools
- **Rotate credentials** - Change passwords/keys periodically
- **Minimal permissions** - Grant only necessary access

### Data Protection

**For Family Media:**
- **Backup originals** - Never lose irreplaceable memories
- **Immutable archives** - Prevent accidental deletion/modification
- **Test restores** - Verify backups actually work
- **Offsite backups** - Protect against local disasters
- **Privacy** - Keep family media private and secure

**For Operations:**
- **Never modify originals** without explicit confirmation and backup
- **Atomic operations** - Write to temp, verify, then rename
- **Audit logs** - Track who did what when
- **Error handling** - Fail safely without data loss
- **Secure deletion** - When deleting, actually delete

## Development Tools and Conventions

### Pre-commit Hooks (Optional)

Consider using [pre-commit](https://pre-commit.com/) for code quality checks:

**Useful checks for this project:**
- YAML/JSON syntax validation
- Shell script syntax (shellcheck)
- Python formatting (black) and linting (flake8)
- Trailing whitespace removal
- Large file detection (prevent accidentally committing media)
- Secret detection (prevent committing credentials)

### Code Style

**Python:**
- Follow PEP 8
- Use type hints
- Docstrings for functions and classes
- Keep functions focused and small

**Bash:**
- Use `set -euo pipefail` for safety
- Quote variables
- Use functions for repeated code
- Include help text (`--help` flag)

**JavaScript/TypeScript (for web platform):**
- Use consistent formatter (Prettier)
- Linter (ESLint)
- Type safety (TypeScript)
- Modern ES6+ syntax

### Git Commit Messages

**Format:**
```
<type>: <short summary>

<optional longer description>

<optional footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `infra`: Infrastructure/deployment changes
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

**Examples:**
```
feat: add photo deduplication script

Implements perceptual hashing to identify duplicate photos across
different filenames and folders.

infra: configure automated backup to cloud storage

Sets up daily incremental backups with 30-day retention.
```

## Useful Commands and Scripts

### Rust Development Commands

**Project Setup:**
```bash
# Clone repository
git clone <repo-url>
cd folio

# Create test data directories
mkdir -p test-data/{fixtures,integration,personal}

# Create Cargo workspace (if not exists)
cargo init --lib crates/media-core
cargo init --bin crates/media-cli
```

**Development:**
```bash
# Run specific crate tests (with nextest)
cargo nextest run -p media-core

# Run all workspace tests (nextest for speed)
cargo nextest run --workspace

# Also run doctests (nextest doesn't run doctests)
cargo test --doc --workspace

# Run specific test
cargo nextest run test_parse_exif_metadata

# Watch mode (requires cargo-watch)
cargo watch -x "nextest run"

# Run with verbose output
cargo nextest run --workspace --no-capture
```

**Building:**
```bash
# Debug build (fast compilation)
cargo build

# Release build (optimized)
cargo build --release

# Build specific binary
cargo build -p media-cli --release

# Run CLI locally
cargo run -p media-cli -- ingest --help

# Install CLI locally for testing
cargo install --path crates/media-cli
```

**Quality Checks:**
```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Run clippy (linter)
cargo clippy

# Strict clippy
cargo clippy --workspace -- -D warnings

# Check for security vulnerabilities
cargo audit

# Generate documentation
cargo doc --no-deps --open

# Check documentation
cargo doc --workspace --no-deps
```

**Testing:**
```bash
# Run unit tests only (with nextest)
cargo nextest run --lib

# Run integration tests only
cargo nextest run --test '*'

# Run doctests (nextest doesn't run doctests)
cargo test --doc --workspace

# Run ALL tests (nextest + doctests)
cargo nextest run --workspace && cargo test --doc --workspace

# Run benchmarks
cargo bench -p media-core

# Generate coverage report (requires cargo-tarpaulin)
cargo tarpaulin --workspace --out Html

# Run with specific features
cargo nextest run --features "exif xmp"
```

**Performance:**
```bash
# Profile with flamegraph (requires cargo-flamegraph)
cargo flamegraph -p media-cli -- ingest test-data/integration/

# Benchmark specific function
cargo bench --bench metadata_parsing

# Check binary size
ls -lh target/release/media-cli
```

### Script Commands

**Python/Bash Scripts:**
```bash
# Run a specific script
python scripts/ingest_photos.py --source /path/to/photos --dry-run

# Check script syntax
shellcheck scripts/*.sh

# Run Python tests (if any)
pytest tests/

# Run with verbose logging
LOGLEVEL=DEBUG python scripts/process_media.py
```

### Infrastructure Commands

**Docker/Services:**
```bash
# Start services (if using Docker Compose)
docker-compose up -d

# Check service status
docker-compose ps

# View logs
docker-compose logs -f media-server

# Stop services
docker-compose down
```

**Backup and Restore:**
```bash
# Create backup
./scripts/backup.sh --destination /backup/path

# Test restore
./scripts/restore.sh --source /backup/path --destination /test/restore

# Verify backup integrity
./scripts/verify_backup.sh /backup/path
```
