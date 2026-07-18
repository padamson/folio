# Folio roadmap

**Date:** 2026-07-18

**Status:** Living document. Updated as stories complete and decisions change.

This roadmap captures the conclusions of the 2026-07-18 planning session that
resumed development after the playwright-rust pause. It records the full set of
user stories, their priorities, and the tool-integration strategy. Individual
story files (`docs/user-stories/NNN-*.md`) are written just-in-time when a
story's work begins; this document is the strategic index, and
[story-seeds.md](user-stories/story-seeds.md) holds the working-level design
notes each story file will be authored from. Architectural
decisions made in the same session are recorded in
[ADR-0002](adr/0002-server-and-channel-strategy.md),
[ADR-0003](adr/0003-editing-toolchain.md), and
[ADR-0004](adr/0004-upstream-tooling-strategy.md).

## Where things stand

Story 001 (backlog ingestion) is in progress. Slices 1–3c are complete:
discovery, BLAKE3 dedup, EXIF timestamps, `YYYY/MM/DD/` organization, temporal
batching, interactive naming, and the browser preview dashboard. Slice 3d
(AI-generated example data, the `folio-examples` crate) is largely built but
uncommitted. Slices 4–6 (XMP sidecars, metadata-merging dedup,
safety/verification) are not started. The working tree also carries a stale
`playwright-rs` version bump that will be redone against git `main`
(see ADR-0004).

## Personas

- **The archivist** (me): technical, owns ingestion, curation, and the
  infrastructure. Shoots a Nikon D800, sometimes NEF and sometimes JPEG.
- **Family viewers**: non-technical family and friends, browsing the archive
  on whatever devices they have.
- **Family contributors**: family and friends adding media from their phones.
- **The future maintainer**: me in ten years, or my kids. Everything must be
  understandable and recoverable without proprietary tools.

Folio's aim is to meet family and friends at the edge, on the devices and
apps they already use. The first target edge is iOS and Apple's photo/video
ecosystem (stories 009 and 016); other ecosystems follow as needed.

## Story catalog

### Tier 1 — foundation: a trustworthy archive

| # | Story | Notes |
|---|-------|-------|
| 001 | Backlog media ingestion | In progress. Gains slice 3e: NEF as a first-class format with RAW+JPEG pairing (`DSC_0042.NEF` + `DSC_0042.JPG` from one shutter press become one logical asset — batched, renamed, and counted as a unit). The backlog cards likely contain NEF, so 3e lands before the real backlog run. |
| 002 | Metadata model and XMP sidecars | The vendor-neutrality contract. Authored as a LinkML schema; panschema generates the Rust types, HTML docs, and JSON Schema (ADR-0004). The model includes from day one: assets with multiple representations (RAW+JPEG pairs), provenance (source device, card, where-from), reactions and comments (author, timestamp, text, source channel), and a visibility/audience slot that gates public publishing. Folio writes one sidecar naming convention and tolerates both common ones on read (`DSC_0042.xmp` from Lightroom, `DSC_0042.NEF.xmp` from darktable). |
| 003 | Archive integrity and trust | Slice 6 grown up: checksum manifest, `folio verify`, "safe to reformat this card" confirmation, periodic scrub with a report. Pair-aware — a re-ingested card matches or skips a whole RAW+JPEG pair, never half of it. |
| 004 | Backup and restore | 3-2-1 with offsite, backup-age monitoring, and tested restores. Fidelity is promised for files + sidecars only; macOS extended attributes are harvested at ingest (story 018) and never load-bearing. |
| 005 | Incremental ingestion | Post-backlog steady state: new cards and watch-folders dedupe against the whole archive, fast and resumable. This is where the stubbed `folio-ingest` crate becomes a real library API (`IngestConfig`, `Ingester`). |

### Tier 2 — family value

| # | Story | Notes |
|---|-------|-------|
| 006 | Catalog and query | `folio find --year 2019 --keyword beach`, stats, counts. A read model built from sidecars — rebuildable, never authoritative. Uses the panschema-generated types. |
| 007 | Curation and publish pipeline | The curation gate lives in folio, not in any editor: folio reads `xmp:Rating` from sidecars (written by darktable, digiKam, or anything else) and generates derivatives. Named derivative profiles in config (`shared-2048`, `print-full`, ...) are referenced by channels, triggers, and digests. Publish targets: folio-server library, static export, Apple Photos shared albums. |
| 008 | Family web gallery | `folio-server`: timeline browse, pre-generated thumbnails, works on a tablet on home Wi-Fi. The backbone viewer per ADR-0002, and the flagship playwright-rust E2E story. |
| 009 | Apple Photos bridge | Declarative channel manifest (`folio-channels.toml`): which shared albums sync, each as `in`, `out`, or `both` — where `both` means two independent one-way flows joined by dedup, never state reconciliation. Inbound harvest captures originals plus comments, likes, and contributor attribution into sidecars (family reactions are archive data in their own right). Outbound publishes curated sets at a named profile. Implemented as adapters shelling out to `osxphotos` (ADR-0004). |
| 010 | Reactive publishing | Event → rule → action: a scheduled sync run diffs comments/likes against known state and evaluates trigger rules ("a comment containing `print` emails the full-resolution render"; "3+ likes adds to `family-favorites`"). Events are channel-agnostic so the same rules later fire from folio-server. Guardrails from day one: recipient allowlist, fire-once idempotency, `--dry-run`, rate limiting, audit log. |
| 011 | Periodic digests with review | Weekly or monthly digest of archive activity — new ingests with contributor attribution, family reactions and comments, milestones, on-this-day retrospectives — generated as a markdown draft with images rendered at a chosen profile. I review and edit the draft, then an explicit approve step publishes it: email to the family allowlist, or a blog post. Email is the private default; a blog post is public, so only photos whose sidecar visibility slot is explicitly public may appear, and the review step is the final gate. This is the human-in-the-loop pattern from ingestion applied to publishing. |
| 012 | Family accounts and permissions | Per-member logins, albums, who-sees-what in folio-server. The Apple Photos bridge substitutes in the interim. |
| 013 | Remote access | For family beyond home Wi-Fi. Shared albums (story 009) cover the Apple-ecosystem case early; this story handles the rest (likely Tailscale or a reverse proxy — needs its own ADR when it comes up). |

### Tier 3 — migration and interop

| # | Story | Notes |
|---|-------|-------|
| 014 | Lightroom legacy app support | Not an exit — Adobe allows month-to-month reactivation, so Lightroom becomes a dormant legacy app (ADR-0003). While a subscription window is active: flush all ratings/keywords/captions to XMP, export full-resolution renders of edited keepers as derivatives, then archive the `.lrcat` catalog (plus previews and settings) as a preserved artifact and document the reactivation procedure. Edit recipes stay accessible forever at the cost of a month's subscription. |
| 015 | Editor and viewer interop validation | Prove sidecar round-trips against darktable and digiKam (the ADR-0001 validation plan, retargeted). Per-tool quirks documented as they surface. |
| 016 | Mobile and modern formats | HEIC/HEIF, HEVC, Live Photos, screenshots — the formats story 001 deliberately excluded. The primary mobile contribution path is the shared-album harvest (story 009). |
| 017 | Video processing | Transcoded derivatives, poster frames, duration/codec metadata into sidecars. Video editing itself stays out of scope. |

### Tier 4 — stewardship

| # | Story | Notes |
|---|-------|-------|
| 018 | macOS metadata harvest and sync | Harvest Finder tags, comments, and download origins (`kMDItemWhereFroms`) into sidecars at ingest (rescuing xattr metadata that dies on SMB copies), and optionally sync `xmp:Rating`/keywords back out as Finder tags so Spotlight can search the archive. A small native module using the `xattr` and `plist` crates, with `osxmetadata` as the reference spec and audit oracle (ADR-0004). A five-minute `osxmetadata --list` sweep over the MacBook backlog folders will size how much harvest matters. |
| 019 | Archive-wide dedup and near-duplicates | The stubbed `folio dedupe` command: exact (BLAKE3) plus perceptual hashing for resized/re-encoded copies, with metadata-merging resolution (absorbs old slice 5). |
| 020 | Health monitoring and alerting | Storage headroom, backup age, scrub results, sync-run status — a green/yellow/red summary, email on red. The scheduled runner here is the same cadence infrastructure the channel sync (009) rides. |
| 021 | Local AI enrichment | Faces, captions, semantic search — local-only, results written to sidecars so they stay portable. Deliberately late: the archive must be trustworthy before it gets smart. |
| 022 | Offline and portable exports | Self-contained album exports ("a USB stick of 2026 for grandma") for people beyond any server's reach. |

## Prioritization rationale

The ordering applies the project's value-driven principle to the actual pain:

1. The 6,000–8,000-file backlog is rotting on SD cards — finish 001/002/003 so
   ingestion is safe and metadata is durable. RAW+JPEG support (slice 3e) goes
   in first because the backlog cards likely contain NEF.
2. Irreplaceable data with no tested backup is the biggest real risk — 004
   before features.
3. Family value arrives fastest through the Apple Photos bridge (009): outbound
   publishing to shared albums gives remote family viewing over Apple's
   infrastructure years before folio-server accounts and remote access exist,
   and inbound harvest is the lowest-friction contribution path on that first
   edge.
4. Migration and interop stories then convert the vendor-neutral claim from
   architecture into evidence.

## Tool integration summary

Full rationale in ADR-0004. In one line each:

- **playwright-rust** — E2E backbone throughout; pinned to git `main` in
  active-iteration dogfood mode; leaves `folio-core`'s runtime dependencies
  (browser auto-open moves to the `webbrowser` crate).
- **panschema** — enters at story 002 as the schema toolchain (LinkML → Rust
  types, HTML docs, JSON Schema); CLI install pinned to `main` via the
  established SHA-pin pattern.
- **osxphotos** — wrapped as a CLI adapter for the Apple Photos bridge; not
  ported to Rust.
- **osxmetadata** — reference spec for a small native xattr module; not ported.

## Architecture direction

The workspace converges on the hexagonal (ports-and-adapters) shape from
`rust-service-template`, adapted to folio's phases:

- **Domain** stays pure and synchronous: the asset model (story 002 generated
  types), pairing, batching, naming, dedup, and the curation gate, plus the
  port traits (ingest sources, publish targets, metadata stores). No
  filesystem, no network, no async — concurrency lives in adapters.
- **Application** holds the use cases (ingest, verify, publish, sync) driven
  through ports with hand-written fakes in tests; this is what the
  `folio-ingest` stub grows into (story 005).
- **Adapters** sit at the edges: filesystem walker, EXIF/XMP read/write, the
  Apple Photos channel (osxphotos), SMTP. `folio-cli` and later
  `folio-server` are driving adapters and composition roots.
- The dependency rule is enforced mechanically with a guppy-based
  architecture fitness test (copied from the template), not by review.
- Two placements are queued rather than immediate: the browser preview moves
  out of `folio-core` (it is a driving adapter, not core), and the crate
  split/rename lands with story 002 when the generated types arrive — no
  big-bang restructure before the backlog work.
- Deferred to the folio-server phase: the template's axum/OpenAPI wiring,
  health and readiness probes, graceful shutdown, and its two-planes testing
  model. The deploy tree (k8s/tofu) is adopted selectively, if at all — a
  single container or launchd unit may be all a family server needs.

ADR-0005 records the decision when the restructure lands.

## Resume checklist

One-time cleanup queued before new feature work:

- [ ] Commit slice 3d: `crates/folio-examples/`, `docs/technical/comfyui-setup.md`,
      `.gitignore` additions; mark 3d complete in the 001 plan.
- [ ] Migrate `playwright-rs` to git `main` (the uncommitted 0.6.1 → 0.7.0 bump
      is stale; published is 0.14.1 with 0.15 on `main`). Take the 0.15-era
      `CLAUDE_SNIPPET.md`/skill copy, not an older one.
- [ ] Replace the runtime playwright browser-open in `folio-core::preview` with
      the `webbrowser` crate; move `playwright-rs` to dev-dependencies.
- [ ] Fix CLAUDE.md crate names (`media-*` → `folio-*`, bin `folio`).
- [ ] Fix the stale story 001 header (slice 3c is complete) and the dangling
      doc references (`docs/current-state.md`, `docs/key-insights.md`, the
      ADR-0001 test-data-strategy path).
- [ ] Reconcile `docs/technical/example-data-strategy.md` with the implemented
      code (OpenAI/ComfyUI backends and `--headless`, not Replicate/`--preview`).
- [ ] Wire the CI SHA-pin pattern for the panschema install and a shell alias
      to its local debug build for fast iteration.
- [ ] Dry-run `folio ingest --dry-run` against one real backlog card to confirm
      what formats are actually present (expecting NEF; watch for surprises).
- [ ] Run `osxmetadata --list` over the MacBook backlog folders to size the
      Finder-metadata harvest.
- [ ] Add slice 3e (RAW+JPEG) to the 001 implementation plan.
- [ ] Draft the story 002 LinkML schema.
- [ ] Audit quality gates against `rust-project-template` (lints table,
      deny/audit/vet, mutation testing scoped to logic crates, CI parity) and
      adopt what's missing.
- [ ] Add the guppy architecture fitness test and declare each current
      crate's layer (see architecture direction above).
- [ ] Make `tracing` real in the CLI (structured logs for scans/ingests; the
      dependency is already present but underused).
