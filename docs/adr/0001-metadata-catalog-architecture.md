# ADR 0001: Metadata and Catalog Architecture

**Status:** Proposed

**Date:** 2025-11-04

**Related Documents:**
- [Key Insights](../key-insights.md)
- [Current State Analysis](../current-state.md)

---

## Context and Problem Statement

The folio system needs a vendor-neutral approach to organizing and cataloging family photos and videos. The current Lightroom-based workflow has broken down due to Adobe's shift to cloud-focus, creating a **3-4 year backlog (~6,000-8,000 unprocessed photos/videos)** and exposing the risks of vendor lock-in.

**Primary Goal:** Establish efficient Rust-based workflows to clear the backlog while building a vendor-neutral foundation for long-term migration.

**Core Problem:** How do we organize, catalog, and search 1.2+ TB of family media (growing 50-100 GB/year) in a way that:
- **Priority 1:** Enables efficient workflow automation to clear 3-4 year backlog
- **Priority 2:** Preserves all metadata in open, standard formats (XMP sidecars)
- **Priority 3:** Doesn't lock us into any specific vendor or tool
- Allows using multiple tools (Lightroom today, alternatives tomorrow)
- Is maintainable by a solo developer
- Survives tool/vendor changes over decades

### Requirements Summary

**Functional Requirements:**
- Must preserve photo/video metadata from multiple sources (D800 DSLR, 4 mobile devices)
- Must support both local storage (NAS) and cloud backup
- Must handle deduplication across Aperture/Lightroom migration overlaps
- Must enable search/browse by date, person, event, location
- Must support custom metadata fields (family-specific tagging)
- Must allow metadata to be read/written by multiple tools

**Non-Functional Requirements:**
- **Performance:** Ingest 2,000 photos in <5 minutes, catalog search <1 second
- **Compatibility:** macOS primary, iOS/Android mobile devices
- **API Ergonomics:** Simple CLI for common tasks, Rust library for custom automation
- **Safety:** Never lose photo data, atomic operations, verify data integrity
- **Maintainability:** Single developer, minimal complex dependencies
- **Vendor Neutrality:** Metadata in open standards (XMP, JSON), no proprietary formats

**Constraints:**
- Must work with existing NAS network storage (SMB/NFS)
- Solo developer - minimal maintenance burden required
- Must be usable by non-technical family members (web interface)
- Must handle 1.2+ TB existing archive + 50-100 GB/year growth
- Must enable migration from existing Lightroom/Aperture catalogs

### Current Architecture Context

**Existing System:**
- Adobe Lightroom catalog (proprietary SQLite format)
- Legacy Aperture library (deprecated, proprietary)
- Photos on NAS
- Unknown overlap/duplication between Pictures/ and Lightroom Photos/
- No mobile device integration (4 devices, ~2,000 photos/year uncaptured)

**Pain Points:**
- Lightroom catalog workflow unclear after Adobe's cloud pivot
- Aperture → Lightroom migration incomplete
- Vendor lock-in risk (already experienced with Aperture deprecation)
- Cannot easily use multiple tools (Motion, Final Cut, open source alternatives)
- Metadata trapped in proprietary catalog formats

---

## Decision Drivers

Prioritized factors influencing this decision:

1. **Backlog Clearance Efficiency** - Must enable fast workflow automation to process 6,000-8,000 photos/videos (highest priority)
2. **Vendor Independence** - Must own our data and catalog, survive vendor changes (experienced Aperture deprecation)
3. **Rust-First Development** - Build workflow automation in Rust for performance, safety, and learning
4. **Tool Flexibility** - Want to use Lightroom today, other tools tomorrow, without losing data
5. **Data Longevity** - Family archive must last decades, outlive any specific software
6. **Solo Developer Maintainability** - Cannot maintain complex systems; focus time on workflows not UIs
7. **Migration Path** - Must enable gradual transition without disrupting current backlog work

---

## Options Considered

### Option 1: Pure Filesystem + Sidecar Metadata

**Description:**
File-based organization using folder structure for hierarchy and XMP/JSON sidecar files for all metadata. No central database.

**Key Implementation Details:**
- Folder structure: `/archive/YYYY/YYYY-MM-DD_event-name/`
- XMP sidecar files for standard metadata (compatible with Lightroom, Darktable)
- JSON sidecar files for custom metadata (family-specific fields)
- Filesystem as source of truth
- Git-trackable metadata (text files)
- Custom Rust catalog builder scans filesystem to create search index

**Code Example:**
```rust
// Directory structure
// /archive/2024/2024-11-04_thanksgiving/
//   ├── IMG_1234.jpg
//   ├── IMG_1234.xmp          # Standard XMP (Lightroom compatible)
//   ├── IMG_1234.metadata.json # Custom fields
//   └── .catalog/             # Auto-generated search index (gitignored)

pub struct MediaItem {
    pub path: PathBuf,
    pub xmp_metadata: XmpMetadata,    // Standard fields
    pub custom_metadata: CustomMeta,   // Family-specific
    pub hash: Blake3Hash,              // For deduplication
}

pub fn scan_archive(root: &Path) -> Result<Catalog> {
    // Walk filesystem, read sidecars, build in-memory catalog
    // Persist to SQLite for fast search (auto-regenerated)
}
```

**Pros:**
- **Maximum vendor independence** - any tool can read/write XMP
- **No lock-in** - metadata lives next to photos, survives any tool change
- **Git-trackable** - can version control metadata changes
- **Simple mental model** - filesystem is the truth
- **Tool-agnostic** - Lightroom, Darktable, custom scripts all work
- **Portable** - easy to migrate storage, cloud sync
- **Transparent** - can inspect/edit metadata with any text editor

**Cons:**
- Slower search without index (need to build catalog on demand)
- More file system overhead (3 files per photo: jpg + xmp + json)
- Need custom tooling to generate search index
- XMP format somewhat complex (XML-based)

**Dependencies Required:**
- `walkdir` for filesystem traversal
- `quick-xml` or XMP library for XMP reading/writing
- `serde_json` for custom metadata
- `blake3` for content hashing (deduplication)
- Optional: `sqlite` or `sled` for generated search index

---

### Option 2: Open Source DAM (Digital Asset Management)

**Description:**
Use existing open-source photo management application as the catalog system.

**Candidates:**
- **Digikam**: KDE photo manager, SQLite database, XMP sidecar support
- **PhotoPrism**: Self-hosted web app, Go-based, automatic AI tagging
- **Darktable**: RAW processor with catalog, XMP-based metadata

**Key Implementation Details:**
- Use DAM tool's database as primary catalog
- Configure to write XMP sidecars for portability
- Build custom Rust tools to augment DAM functionality
- Web interface provided by DAM (PhotoPrism)

**Code Example:**
```rust
// Read Digikam's SQLite database
pub fn query_digikam_catalog(db_path: &Path) -> Result<Vec<MediaItem>> {
    let conn = Connection::open(db_path)?;
    // Query Digikam's schema
    conn.query_row("SELECT * FROM Images WHERE ...")?
}

// Or use XMP sidecars that Digikam writes
pub fn read_xmp_sidecar(photo_path: &Path) -> Result<Metadata> {
    // Digikam writes standard XMP files
}
```

**Pros:**
- **Full-featured UI** - mature photo browsing, tagging, search
- **Battle-tested** - Digikam has 20+ years development
- **Web interface** - PhotoPrism provides family-accessible web UI
- **AI features** - PhotoPrism has face detection, object recognition
- **Active development** - community-driven improvements
- **XMP support** - most write sidecars for portability

**Cons:**
- **Still vendor dependency** - relies on specific tool's database schema
- **Complex migration** - if DAM project dies, need to migrate catalog
- **Less flexible** - constrained by DAM's data model and workflows
- **Learning curve** - need to learn DAM's architecture to extend
- **Database lock-in** - even with XMP, losing full-text search/tags if tool dies
- **Maintenance burden** - need to keep DAM updated, working with dependencies

**Dependencies Required:**
- Digikam, PhotoPrism, or Darktable application
- Application's dependencies (Qt for Digikam, Go for PhotoPrism)
- Database (SQLite for Digikam, varies by tool)

---

### Option 3: Hybrid Approach (Filesystem + XMP + Custom Rust Workflows + Interchangeable Viewers)

**Description:**
Combine filesystem-as-source-of-truth with XMP sidecars and custom Rust workflow automation. Focus development on workflow tools (ingestion, deduplication, metadata management), not UI. Use existing viewers/editors (Lightroom, digiKam, Urocissa, etc.) that read XMP.

**Key Implementation Details:**
- File-based organization (folders + XMP/JSON sidecars) as source of truth
- **Rust workflow tools** (what you build): ingestion, deduplication, metadata extraction, backup verification, export automation
- **Viewers** (use existing tools): Lightroom (current), digiKam, Urocissa (Rust-based DAM), or simple static HTML generator
- Optional: Rust catalog builder generates SQLite search index from sidecars (if existing viewers insufficient)
- XMP sidecars ensure any tool can read metadata
- Focus development time on backlog-clearing workflows, not UI development

**Code Example:**
```rust
// Source of truth: filesystem + sidecars
// /archive/2024/2024-11-04_thanksgiving/IMG_1234.jpg
// /archive/2024/2024-11-04_thanksgiving/IMG_1234.xmp
// /archive/2024/2024-11-04_thanksgiving/IMG_1234.metadata.json

// Auto-generated catalog (disposable, can rebuild anytime)
// .catalog/search.db (SQLite full-text search)

pub struct CatalogBuilder {
    archive_root: PathBuf,
    catalog_db: PathBuf,
}

impl CatalogBuilder {
    pub fn rebuild(&self) -> Result<()> {
        // 1. Walk filesystem
        // 2. Read all XMP + JSON sidecars
        // 3. Generate SQLite database with full-text search
        // 4. Index by date, person, event, location, hash
        Ok(())
    }

    pub fn incremental_update(&self, changed_files: &[PathBuf]) -> Result<()> {
        // Update only changed entries for performance
        Ok(())
    }
}

// CLI usage
// $ media ingest --source /sdcard/DCIM --dest /archive/2025/
// $ media catalog rebuild  # Regenerate search index
// $ media search "thanksgiving 2024"
// $ media dedupe --dry-run  # Find duplicates by hash

// Any tool can edit:
// $ darktable /archive/2024/2024-11-04_thanksgiving/IMG_1234.jpg
//   → Darktable writes IMG_1234.xmp
// $ media catalog update  # Picks up XMP changes
```

**Pros:**
- **Maximum flexibility** - filesystem + XMP is truth, everything else is replaceable
- **Vendor independence** - can use ANY viewer (Lightroom, Urocissa, digiKam, custom)
- **Workflow automation focus** - build Rust tools for backlog clearance (highest value)
- **Don't build UIs** - leverage existing mature viewers instead of building from scratch
- **Gradual migration** - keep using Lightroom today while building workflows
- **Learning Rust** - focus on domain logic (media workflows) not UI complexity
- **No viewer lock-in** - if viewer tool dies, XMP preserves all metadata
- **Performance control** - optimize workflows for your specific backlog/volume

**Cons:**
- **Rust workflow development time** - still need to build ingestion, deduplication, etc.
- **XMP learning curve** - need to understand metadata standards (but worth it for vendor neutrality)
- **Solo maintenance** - no community to share workflow tool development
- **Viewer evaluation needed** - must test which viewer works best with your XMP workflow

**Dependencies Required:**
- `walkdir` - filesystem traversal
- `kamadak-exif` - EXIF reading
- XMP library (or `quick-xml` for custom parsing)
- `serde_json` - JSON metadata
- `rusqlite` - generated catalog database
- `blake3` - content hashing
- `clap` - CLI
- Web: `axum` or `actix-web`, TypeScript frontend

---

## Comparison Matrix

### Feature Comparison

| Capability | Filesystem+Sidecar | Open Source DAM | Hybrid | Weight | Notes |
|-----------|----------|----------|----------|--------|-------|
| **Vendor Independence** | 5 | 3 | 5 | **Critical** | Can survive any tool change |
| **Tool Flexibility** | 5 | 3 | 5 | **Critical** | Use Lightroom AND Darktable AND custom |
| **Data Longevity** | 5 | 3 | 5 | **Critical** | Will metadata survive 50 years? |
| **Search Performance** | 2 | 5 | 4 | High | Without index, search is slow |
| **Web Interface** | 1 | 5 | 3 | High | Family needs to browse |
| **Ease of Implementation** | 3 | 5 | 2 | Medium | DAM is ready now, custom takes time |
| **Maintenance Burden** | 4 | 2 | 3 | High | Solo developer constraint |
| **Migration Complexity** | 5 | 2 | 4 | Medium | Moving from Lightroom |

### Performance Comparison

| Metric | Filesystem+Sidecar | Open Source DAM | Hybrid | Requirement | Notes |
|--------|----------|----------|----------|-------------|-------|
| **Ingestion Speed** | 400 photos/min | 300 photos/min | 400 photos/min | > 400 photos/min | Simple file copy vs DB insert |
| **Search Speed** | 10+ seconds | <1 second | <1 second | < 1 second | With/without index |
| **Memory Usage** | 200 MB | 500 MB - 1 GB | 300 MB | < 1 GB | Catalog in memory |
| **Storage Overhead** | 10-15% | 5-10% | 10-15% | Acceptable | Sidecar files vs DB |

### Maintenance & Ecosystem

| Factor | Filesystem+Sidecar | Open Source DAM | Hybrid | Weight | Notes |
|--------|----------|----------|----------|--------|-------|
| **Maturity** | 5 (XMP is standard) | 5 | 2 | Medium | XMP is 20+ years old |
| **Community Support** | 3 | 5 | 1 | Medium | DAMs have communities |
| **Maintenance Burden** | Low | Medium | Medium | **Critical** | Solo developer |
| **Breaking Changes Risk** | Low | Medium | Low | High | Filesystem very stable |
| **Learning Curve** | Low | Medium | High | Medium | Need to learn Rust + domain |

---

## Decision Outcome

**Chosen Option:** Option 3 - Hybrid Approach (Filesystem + XMP + Custom Rust Workflows + Interchangeable Viewers)

**Rationale:**

We chose the Hybrid approach because it best aligns with our critical decision drivers:

1. **Backlog Clearance Efficiency (Critical Driver #1)** - Focus Rust development on workflow automation (ingestion, deduplication) not UI building. Leverage existing viewers to get value faster. This directly addresses the 3-4 year backlog.

2. **Vendor Independence (Critical Driver #2)** - Filesystem + XMP/JSON sidecars means we own our data completely. If Lightroom stops working, switch to digiKam or Urocissa without losing anything. Already experienced pain from Aperture deprecation.

3. **Rust-First Development (Critical Driver #3)** - Build what matters in Rust (workflows, automation) while using existing tools for viewing/editing. Learn Rust for domain logic, not UI frameworks.

4. **Tool Flexibility (Critical Driver #4)** - Can use Lightroom for photo editing today, Urocissa (Rust DAM) for viewing tomorrow, Motion/Final Cut for video. All tools read/write standard XMP.

5. **Solo Developer Maintainability (Critical Driver #6)** - Don't build UI from scratch. Spend development time on high-value workflows. Simple architecture: files + sidecars + Rust CLI tools.

6. **Gradual Migration (Critical Driver #7)** - Keep using Lightroom during backlog clearance. Build Rust workflows incrementally. Evaluate viewers (Urocissa, digiKam) after workflows are stable.

**Trade-offs Accepted:**

- **Rust workflow development time** - Need to build ingestion, deduplication, metadata tools ourselves. Mitigated by focusing only on backlog-clearing workflows first (not building a full DAM).

- **XMP learning curve** - Need to understand metadata standards. Acceptable because this knowledge provides vendor independence and is reusable across tools.

- **Viewer evaluation overhead** - Must test which existing viewer works best (Lightroom, Urocissa, digiKam). Mitigated by XMP compatibility ensuring multiple options work.

**Why not Option 1 (Pure Filesystem)?**
- Very similar to Option 3, but Option 3 clarifies the viewer strategy explicitly
- Option 3 emphasizes using existing viewers (don't reinvent) while Option 1 was ambiguous

**Why not Option 2 (Open Source DAM)?**
- PhotoPrism/digiKam can still be USED as viewers in Option 3
- But Option 2 would make them the "primary" system, risking lock-in to their database
- We want to own the workflows and metadata, not depend on DAM's internal organization
- Option 3 lets us evaluate DAMs (including Urocissa) as interchangeable viewing layers

---

## Consequences

### Positive Consequences

- **Complete data ownership** - Metadata lives in human-readable text files next to photos
- **Tool freedom** - Can use Lightroom, Darktable, Motion, Final Cut, or custom Rust scripts interchangeably
- **Future-proof** - Filesystem + text files will be readable for decades
- **Version controllable** - Can track metadata changes in Git if desired
- **Learning valuable skills** - Deep understanding of media management, Rust, web development
- **Tailored workflows** - Build exactly what family needs, no unnecessary complexity

### Negative Consequences

- **Development time** - Will take weeks/months to build full feature set
- **Gradual feature rollout** - Web interface, advanced search, etc. come later
- **Solo responsibility** - All bugs and maintenance fall on one developer

### Risks and Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|-----------|------------|
| Development takes too long, backlog grows | Medium | Medium | Start with MVP (CLI ingest + dedupe), deliver value quickly |
| Rust catalog generator has bugs, corrupts data | High | Low | Never modify source files, catalog is read-only generated artifact, extensive tests |
| Solo developer abandons project | High | Low | Filesystem + XMP is readable by any tool, can switch to Digikam/PhotoPrism later |
| Performance insufficient for 1.2 TB | Medium | Low | Incremental catalog updates, SQLite FTS is fast, benchmark early |

---

## Validation

### How This Decision Will Be Validated

**Phase 1: XMP Compatibility Proof-of-Concept (1-2 days)**
- [ ] Create 10 sample photos with XMP sidecars using Rust
- [ ] Test if Lightroom reads/writes XMP sidecars correctly
- [ ] Test if digiKam reads/writes XMP sidecars correctly
- [ ] Test if Urocissa reads/writes XMP sidecars correctly
- [ ] Validate round-trip: Rust creates XMP → viewer edits → Rust reads changes

**Phase 2: Backlog Workflow MVP (2-3 weeks)**
- [ ] Implement CLI ingestion tool (SD card → archive with deduplication)
- [ ] Implement EXIF extraction → XMP sidecar generation
- [ ] Process 100 photos from actual backlog
- [ ] Verify metadata preservation and file organization

### Success Criteria

**Immediate (XMP PoC):**
- XMP sidecars created by Rust are readable by Lightroom, digiKam, and Urocissa
- Edits made in viewers are readable back in Rust
- No metadata loss in round-trip

**Short-term (Backlog MVP):**
- CLI can ingest 2,000 photos in <5 minutes (backlog clearance speed)
- Deduplication accuracy >99% (hash-based, no false positives)
- Zero data loss (all photos preserved with correct EXIF → XMP metadata)
- Can view ingested photos in any compatible viewer immediately

### Benchmark Needed?

**Decision:** Yes

**Why:** Need to validate that SQLite-based catalog can handle 100,000+ photos with sub-second search.

**Scope:**
- Catalog rebuild time for 10K, 50K, 100K photos
- Search query performance (by date, keyword, person tag)
- Memory usage during catalog operations

**Methodology:**
- Use criterion for benchmarks
- Generate synthetic metadata for large-scale testing
- Test on representative hardware (MacBook)

**Timeline:** After MVP implementation, before committing to web interface development

---

## Implementation Notes

### Migration Path

**Phase 0: Foundation (Completed - 2025-11-04)**
- [x] Document architecture decision (this ADR)
- [x] Set up Rust workspace (`folio-core`, `folio-cli`, `folio-ingest` crates)
- [x] Test data strategy documented (`docs/testing/test-data-strategy.md`)
- [x] Test fixtures created (`test-data/fixtures/` with photos and videos)
- [ ] XMP compatibility proof-of-concept (test with Lightroom, digiKam, Urocissa) - Moved to Phase 1, Slice 4
- [ ] Define folder structure conventions - Moved to Phase 1, Slice 2
- [ ] Research Rust XMP libraries - Moved to Phase 1, Slice 4

**Phase 1: Backlog Clearance Workflows (In Progress - Started 2025-11-04)**
**Goal:** Process 3-4 year backlog efficiently
- [x] **Slice 1 Complete:** Basic ingestion CLI with mixed media (photos + videos) support and hash-based deduplication
- [ ] **Slice 2:** Timestamp extraction and YYYY/MM/DD folder organization
- [ ] **Slice 3:** Temporal batching and interactive file naming
- [ ] **Slice 4:** EXIF extraction → XMP sidecar generation
- [ ] **Slice 5:** Intelligent metadata merging for duplicates
- [ ] **Slice 6:** Human-in-the-loop confirmations and verification
- [ ] Process photos/videos from MacBook/SD cards (2022-2025 backlog)
- [ ] Verify with existing viewer (Lightroom or evaluate Urocissa)

**Phase 2: Historical Migration (2-3 weeks)**
**Goal:** Consolidate and deduplicate existing archives
- [ ] Deduplicate Pictures/ vs Lightroom Photos/ folders
- [ ] Migrate iPhone_paul (2017) backup
- [ ] Export existing Lightroom metadata to XMP sidecars
- [ ] Reorganize to standard folder structure

**Phase 3: Mobile Integration (3-4 weeks)**
**Goal:** Capture ongoing 2,000 photos/year from 4 devices
- [ ] Implement automatic mobile device ingestion
- [ ] iCloud Photo Library export or alternative
- [ ] Automatic metadata tagging (device, person)

**Phase 4: Viewer Evaluation & Selection (1-2 weeks)**
**Goal:** Select best viewer for family use
- [ ] Evaluate Urocissa (Rust-based, AI features)
- [ ] Evaluate digiKam (mature, desktop)
- [ ] Evaluate PhotoPrism (web-based)
- [ ] Optional: Build simple static HTML gallery generator if needed

### Code Changes Required

- Create `media-core` crate (metadata extraction, catalog building)
- Create `media-cli` crate (CLI commands)
- Create `media-catalog` crate (SQLite catalog management)
- Create `media-ingest` crate (device ingestion, deduplication)
- Future: `media-server` crate (web API)

### Documentation Updates

- [x] Create ADR-0001 (this document)
- [ ] Document folder structure conventions in README
- [ ] Document XMP/JSON metadata schema
- [ ] Create user story for mobile ingestion
- [ ] Create implementation plan for CLI MVP

### Testing Strategy

- Unit tests for metadata extraction
- Integration tests for CLI workflows
- Property-based tests for deduplication (proptest)
- Benchmark catalog performance (criterion)
- Test XMP compatibility with Lightroom/Darktable

### Rollback Plan

If custom Rust approach doesn't work:
- Filesystem + XMP sidecars are already industry standard
- Can switch to Digikam or PhotoPrism at any time
- All metadata will be preserved in XMP files
- No vendor lock-in means low risk of failure

---

## References

- **XMP Specification:** https://www.adobe.com/devnet/xmp.html
- **Urocissa (Rust-based DAM):** https://github.com/hsa00000/Urocissa - Candidate for viewer evaluation
- **Digikam:** https://www.digikam.org/
- **PhotoPrism:** https://photoprism.app/
- **Darktable:** https://www.darktable.org/
- **Rust Image Crate:** https://github.com/image-rs/image
- **kamadak-exif:** https://github.com/kamadak/exif-rs
- **Related ADRs:** None yet (first ADR)

---

## Notes

**Open Questions:**
- Should we use existing Rust XMP library or build custom parser with quick-xml? (Need to research available crates)
- What additional custom metadata fields do we need beyond standard XMP? (family member tags, event types, privacy levels)
- Does Urocissa write metadata to XMP sidecars or only internal database? (Critical for compatibility)

**Future Considerations:**
- **Evaluate Urocissa as viewer** - Test if it reads/writes XMP sidecars created by our Rust workflow tools
- **Proof of concept:** Create 10 sample photos with XMP sidecars, test compatibility with Lightroom, digiKam, Urocissa
- Add AI-powered face detection (Urocissa may provide this if compatible)
- Add automatic geo-tagging from iPhone location data
- Add video thumbnail generation and preview support
- Consider IPFS for distributed backup (beyond commercial cloud backup service)

---

**Author:** Paul Adamson (via Claude Code)

**Reviewers:** N/A (solo developer)

**Last Updated:** 2025-11-04
