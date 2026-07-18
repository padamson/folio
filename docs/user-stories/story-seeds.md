# Story seeds

**Date:** 2026-07-18

Working notes captured while the 2026-07-18 planning session was fresh. Each
section holds enough detail to author the full story file
(`docs/user-stories/NNN-*.md`, from the template) without re-deriving the
design: the user-story sentence, acceptance-criteria themes, design decisions
already made, dependencies, and open questions. These are seeds, not stories —
no slice plans, no full acceptance criteria.

Lifecycle: when a story graduates to its own file, move anything still useful
into that file and delete the section here, so there is never two places to
update. Numbering matches [the roadmap](../roadmap.md).

---

## 001 — Backlog media ingestion (delta: slice 3e, RAW+JPEG)

The story and plan exist; this seed covers only the new slice 3e, which lands
before the real backlog run because the backlog cards likely contain NEF.

- `detect_media_type` gains `Photo(PhotoFormat::Nef)`; `scan_directory` picks
  up `.NEF`/`.nef`. NEF is TIFF-based, so `kamadak-exif` should read
  `DateTimeOriginal` directly — the slice's first test verifies that.
- Pairing rule: same basename + capture timestamps within tolerance → one
  logical asset. Renamed as a unit sharing one generated basename
  (`20221124-143052-thanksgiving.nef` + `.jpg`), counted once in batch
  summaries and the preview dashboard.
- Both single-format cases (JPEG-only day, NEF-only day) stay ordinary paths,
  not exceptions.
- Dedup and the slice-6 verification become pair-aware: a re-ingested card
  matches or skips whole pairs, and "safe to reformat" counts both members.
- Fixtures: a minimal TIFF-with-EXIF written with a `.NEF` extension for unit
  tests; one tiny real CC0 NEF for integration if the fake proves too clean.
  `folio-examples` needs NEF output for the generated datasets.
- Note for story 016: Live Photos (HEIC + MOV) are the same pairing shape —
  design the pairing machinery to generalize.

## 002 — Metadata model and XMP sidecars

As the archivist, I want every asset's metadata in open, tool-readable
sidecars generated from a single schema, so the archive outlives any tool.

- LinkML schema in-repo; panschema generates Rust serde types, HTML docs with
  the interactive graph, and JSON Schema (`panschema.toml` with a `path:`
  source and a `[generate.folio]` block; output committed; `panschema verify`
  in CI). See ADR-0004 for the pin mechanics.
- Model slots decided this session:
  - **Asset with multiple representations** — original(s) (RAW, JPEG), plus
    derivatives, each with hash, format, and role. RAW+JPEG pairs and future
    Live Photos hang off this.
  - **Provenance** — source device, card/batch, macOS where-from, channel of
    arrival (SD card, folder, Apple Photos album, with contributor for shared
    albums).
  - **Reactions and comments** — author, timestamp, text, source channel.
    Family comments in shared albums are archive data in their own right.
  - **Visibility/audience** — something like `private | family | public`.
    Gates outbound publishing; only `public` may appear on the blog (story
    011).
  - Standard fields map to standard XMP: `xmp:Rating`, `dc:subject`
    (keywords), `dc:description`.
- Sidecar convention: folio picks one naming to write and tolerates both on
  read — `DSC_0042.xmp` (Lightroom style) and `DSC_0042.NEF.xmp` (darktable
  style).
- The XMP packet (RDF/XML) serialization stays hand-written in folio for now;
  panschema owns the model and the Rust/JSON types. Folio-specific fields that
  have no standard XMP home go in a JSON sidecar (or a folio namespace —
  decide during implementation).
- Acceptance-criteria themes: Lightroom, darktable, and digiKam each read a
  folio-written sidecar; a rating written by darktable is read back by folio
  without loss; schema docs published and browsable.
- Open questions (carried from ADR-0001): which Rust XMP library, the folio
  namespace URI, exactly where the XMP/JSON boundary sits, merge semantics
  when both a sidecar and embedded EXIF disagree.

## 003 — Archive integrity and trust

As the archivist, I never lose a file and can prove it.

- Checksum manifest (BLAKE3) maintained per archive tree; `folio verify`
  checks archive against manifest, and a card against the archive (the
  "safe to reformat" report compares the card's full content set before
  giving a verdict).
- Periodic scrub (scheduled, story 020 runner) with a written report; a
  deliberately corrupted test file must be detected.
- Pair-aware throughout (see 001/3e).
- Atomic operations discipline: write to temp, verify, rename — already the
  project rule; this story makes it testable.

## 004 — Backup and restore

As the future maintainer, I can restore the archive after losing any one
storage location.

- 3-2-1: NAS primary, second local copy, offsite. Backup age surfaced by
  story 020; email on staleness.
- Restore testing is part of the story, not an afterthought — a documented,
  periodically-run restore drill (quarterly cadence per project conventions).
- Scope: files + sidecars only. macOS xattrs are harvested at ingest (018)
  and not preserved; catalog/read-model state (006) is rebuildable and
  excluded.
- Open question: backend (restic, borg, rclone, plain rsync + snapshots) —
  wants a small ADR with a cost/complexity comparison.

## 005 — Incremental ingestion

As the archivist, ingesting a new card after the backlog is cleared is fast,
safe, and resumable.

- This is where the stubbed `folio-ingest` crate becomes the real library API
  from story 001 (`IngestConfig`, `Ingester`, batch naming strategy) — CLI
  logic moves down into the library.
- Dedup against the *whole archive* (manifest/index from 003), not just the
  current batch. Interrupted runs resume without duplicating work.
- Watch-folder mode for drop-in ingestion (also the fallback mobile path).
- Boomerang prevention: folio records the hashes of derivatives it publishes
  (007/009), so its own published copies coming back through a channel are
  recognized and skipped, not re-ingested.
- Architecturally this crate is the application layer of the hexagonal shape
  (roadmap, "Architecture direction"): use cases over ports, tested with
  hand-written fakes; concurrency (hashing, walking) stays in the adapters.

## 006 — Catalog and query

As the archivist, I can find things: `folio find --year 2019 --keyword beach`.

- A read model built from sidecars, rebuildable from scratch
  (`folio catalog rebuild`), never authoritative. Uses the story 002 generated
  types.
- Query by date range, keyword, rating, media type, batch/event name;
  `folio stats` for counts and storage by year/type.
- Storage choice open (SQLite is the obvious candidate); panschema's Postgres
  DDL writer becomes interesting if/when folio-server wants a shared DB —
  its current gaps (inheritance, multivalued slots) are known upstream
  findings to file.

## 007 — Curation and publish pipeline

As the archivist, rating a photo in any XMP-writing tool is enough to publish
it; folio does the rest.

- Gate: `xmp:Rating >= threshold`, threshold per target. The curation gate is
  folio's, not any editor's (ADR-0003).
- Named derivative profiles in config, referenced everywhere (channels,
  triggers, digests): e.g. `shared-2048` (max-edge 2048, JPEG q85),
  `print-full` (original resolution, JPEG q95). Derived from the old
  Lightroom-publish spec (sRGB JPEG 85%).
- Derivatives cached, regenerable, traceable to the original's hash.
- Delete propagation: un-rating removes from targets — to a trash/pending
  state, never a hard delete (mirrors the old smart-collection semantics).
- Publish targets behind a trait: folio-server library, static export, Apple
  Photos album (ADR-0002).

## 008 — Family web gallery

As a family viewer, I open a link on my tablet and browse the archive.

- `folio-server` (axum): timeline browse, pre-generated thumbnails (profiles
  from 007), lazy loading. v1 is read-only and LAN-trusted — accounts arrive
  in 012.
- The flagship playwright-rust E2E story: multi-browser tests, visual
  regression on gallery layouts via `to_have_screenshot`
  (`screenshot-diff` feature). One browser launch per test;
  `leak-timeout = "1s"` in nextest config.
- Performance bar: smooth scrolling on a mid-range tablet over Wi-Fi.
- Lift the `rust-service-template` axum patterns when this starts: `router()`
  factored from `main` for in-process tests, health/readiness endpoints
  doubling as probes, graceful shutdown on SIGTERM, code-first OpenAPI
  (utoipa + Scalar), and the composition-root-before-runtime `main` shape if
  a blocking DB adapter ever appears.

## 009 — Apple Photos bridge

As a family contributor, I add photos to a shared album and they end up in
the archive; as the archivist, I publish curated sets the family sees in the
app they already use.

- `folio-channels.toml`: `[[channel]]` entries with `name`, `kind`
  (`apple-photos` first), `album`, `direction` (`in` | `out` | `both`),
  `batch-name` (inbound), `source` query + `profile` (outbound). `both` is
  strictly two one-way flows joined by dedup — no state reconciliation.
- Inbound: shell out to `osxphotos export --album ... --sidecar XMP`, then
  run normal ingest (005) over the export; harvest `comments`, `likes`, and
  `owner` per photo into the sidecar reaction/provenance slots (002).
- Outbound: `osxphotos import` (or AppleScript) of derivatives at the
  channel's profile.
- Scheduled sync runs (launchd/cron, shared with 020) with a state file of
  seen photos/comments; runs are idempotent.
- Known dependency risk: osxphotos reads the Photos SQLite database via
  reverse engineering and is currently broken for shared albums on macOS 26 —
  pin the archivist Mac's macOS upgrades to osxphotos compatibility
  (ADR-0004).
- Validation: full round-trip — family member adds a photo + comment; both
  land in the archive; a curated set publishes back and is visible remotely.

## 010 — Reactive publishing

As a family member, hearting or commenting on a photo makes things happen
without anyone running a command.

- Normalized events with the channel as a field (`photo.commented`,
  `photo.liked`, `album.updated`) so the same rules later fire from
  folio-server interactions.
- `[[trigger]]` rules in the manifest: `on` (comment/like), `match` (regex on
  comment text) or `threshold` (like count), `action`
  (`email` | `add-to-collection` | `publish` | `flag`), `profile`, `to`.
  Comment conventions can grow into lightweight commands (`@email dad`,
  `#8x10`).
- Guardrails from day one: recipient allowlist in config (triggers can only
  email pre-approved addresses), fire-once idempotency keyed on
  (photo, rule, event), `--dry-run`, rate limiting, audit log of every action.
- Email transport (SMTP config) shared with story 020's alerting.

## 011 — Periodic digests with review

As the archivist, folio drafts a family update; I edit and approve it before
anything is sent or posted.

- Cadence configurable (weekly/monthly). Content sources: new ingests with
  contributor attribution, reactions and comments since the last digest,
  milestones, on-this-day retrospectives. A state file tracks what has been
  digested.
- Output: a markdown draft plus images rendered at a chosen profile, written
  to a review directory. I edit the draft in any editor; an explicit
  `folio digest approve` (name TBD) publishes.
- Targets: email to the family allowlist (private default); blog post
  (public — likely a generated post/PR against `padamson.github.io`,
  mechanics open). Blog eligibility is double-gated: only assets whose
  visibility slot (002) is `public`, and the human review is the final gate.
- This is the ingestion human-in-the-loop pattern applied to publishing;
  reuses events (010), profiles (007), and channels (009).

## 012 — Family accounts and permissions

As a family viewer, I log in and see what's shared with me.

- Per-member logins, albums, who-sees-what on folio-server. Auth approach
  open (passkeys are attractive for non-technical users — no passwords to
  support). Visibility slots from 002 are the underlying data.
- Urgency reduced by the bridge (009) serving as the interim sharing surface.

## 013 — Remote access

As a remote family member, I can reach the gallery without being on home
Wi-Fi.

- Interim answer is shared albums (009). The real story needs an ADR:
  Tailscale (simplest, per-device), reverse proxy + TLS (open to anyone,
  larger attack surface), or hosted. Not before 008/012 exist.

## 014 — Lightroom legacy app support

As the archivist, my Lightroom years stay accessible without a subscription.

- Sequence (while a subscription window is active): enable
  write-XMP-to-files; verify parity (spot-check ratings/keywords/captions in
  sidecars against the catalog, counts per collection); export full-res
  renders of edited keepers (criteria roughly: has edits AND rating above
  threshold) as derivatives linked to their originals; archive `.lrcat` +
  previews + presets as a preserved artifact; document the reactivation
  procedure; test reactivation once before letting the subscription lapse.
- No deadline pressure beyond the flush — month-to-month reactivation is
  always available if the catalog is preserved (ADR-0003).
- Establishes the "legacy application archive" pattern for any future tool.

## 015 — Editor and viewer interop validation

As the archivist, I know exactly which tools round-trip folio's sidecars.

- A tool × operation matrix: read/write rating, keywords, caption; sidecar
  naming tolerance — against darktable and digiKam (Lightroom covered by
  014's parity check).
- Automate where the tool has a CLI (`darktable-cli`, digiKam's batch tools);
  otherwise a written manual procedure. Quirks documented as found.
- Cheap and high-confidence; de-risks everything built on sidecars.

## 016 — Mobile and modern formats

As a family contributor, my iPhone photos ingest as cleanly as the D800's.

- HEIC/HEIF and HEVC decode (check `image` crate coverage; may need
  libheif bindings), Live Photos as HEIC+MOV pairs (generalize the 3e pairing
  machinery), PNG screenshots.
- Primary arrival path is the bridge (009); watch-folder (005) is the
  fallback for non-Apple devices.

## 017 — Video processing

As a family viewer, videos play smoothly in the gallery.

- Shell out to `ffmpeg`: web-friendly derivatives (H.264 first), poster
  frames, duration/codec/resolution into sidecars.
- Video profiles join the 007 profile system. Editing stays out of scope.

## 018 — macOS metadata harvest and sync

As the archivist, Finder tags and comments I've made over the years survive
into the archive.

- Read-only harvest at ingest: Finder tags → `dc:subject`, Finder comment →
  `dc:description`, download origins (`kMDItemWhereFroms`) → provenance
  (002). Native module on the
  `xattr` + `plist` crates (Finder tags are a binary plist in
  `com.apple.metadata:_kMDItemUserTags`); `osxmetadata` CLI as reference spec
  and test oracle. Writing Finder comments needs AppleScript — out of scope
  for the Rust path.
- Optional reverse sync: `xmp:Rating`/keywords → Finder tags so Spotlight can
  search the archive. Strictly derived data, regenerable.
- First action is the audit: `osxmetadata --list` sweep over the MacBook
  backlog folders to size whether harvest matters at all (resume checklist).

## 019 — Archive-wide dedup and near-duplicates

As the archivist, resized and re-encoded copies of the same shot get found
and merged.

- Exact dedup exists (BLAKE3). This adds perceptual hashing for
  near-duplicates, candidate review (CLI list or preview dashboard), and
  metadata-merging resolution (absorbs the original slice 5 design).
- Policy: never auto-delete — losers move to a quarantine/trash area;
  keep-best-representation rules are explicit and logged.
- Fills in the stubbed `folio dedupe` subcommand.

## 020 — Health monitoring and alerting

As the archivist, I get an email when something needs me, and silence
otherwise.

- Checks: storage headroom, backup age, last scrub result, channel sync-run
  status. Green/yellow/red summary; email on red (allowlisted SMTP shared
  with 010).
- One scheduled-runner substrate (launchd/cron) shared by scrub (003), sync
  (009), triggers (010), and digests (011) — build it once here or in 009,
  whichever lands first.

## 021 — Local AI enrichment

As a family viewer, I can search for "beach" or "grandma" and find things.

- Local-only inference (the ComfyUI/Flux setup proves the muscle exists);
  faces, captions, embeddings for semantic search. All results written to
  sidecars under the folio namespace so they stay portable; the catalog
  indexes them.
- Opt-in and deliberately late — the archive must be trustworthy before it
  gets smart.

## 022 — Offline and portable exports

As a remote relative without any of this infrastructure, I get a USB stick
that just works.

- Self-contained export: chosen album/query rendered at a profile, plus a
  static HTML index (possibly reusing gallery components) that opens from the
  filesystem, plus a checksum manifest for verification.
- Also the answer for photo-book prep and archival gifts.
