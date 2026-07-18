# ADR 0002: Server and channel strategy

**Status:** Proposed

**Date:** 2026-07-18

**Related Documents:**
- [ADR-0001: Metadata and Catalog Architecture](0001-metadata-catalog-architecture.md)
- [Roadmap](../roadmap.md) (stories 007–013)

---

## Context and problem statement

ADR-0001 chose a hybrid architecture: filesystem + XMP/JSON sidecars as the
source of truth, custom Rust workflow tools, and interchangeable third-party
viewers. The reference viewer in practice has been a self-hosted Immich
instance (documented in the separate `home-media-management` repo), fed by a
Lightroom publish service. Two problems have emerged with Immich in that role:

1. **It violates the architecture's own principle.** Everything family members
   do in Immich — albums, favorites, faces — lands in Immich's Postgres
   database, which becomes a second metadata authority that folio's sidecars
   know nothing about. That is the same lock-in shape ADR-0001 exists to
   reject, just self-hosted.
2. **The operational record is poor.** The setup guide documents database
   corruption risk (Postgres must never live on the NAS), fragile macOS
   Docker/firewall networking with a multi-step troubleshooting checklist, and
   major upgrades risky enough to require a permanent dev-sandbox environment.
   It also has no remote access, no automated backup, and no per-member
   permission model.

Separately, folio wants to meet family and friends at the edge, on the
devices and apps they already use. The first target edge is iOS and Apple's
photo/video ecosystem, where shared albums are the native way people
contribute and view photos. Any server strategy has to account for that, not
fight it.

The question: what serves the family-facing role — browsing, sharing,
contribution — and how do vendor ecosystems like Apple Photos fit?

## Decision drivers

1. **Vendor independence** — no second metadata authority; sidecars remain the
   only source of truth (ADR-0001, driver 2).
2. **Family adoption** — contribution and viewing must work with the apps
   family and friends already use, with zero setup on their devices.
3. **Operational simplicity** — solo maintainer; the Immich container stack
   has cost real debugging time and imposes a permanent upgrade-testing burden.
4. **Remote sharing** — family beyond home Wi-Fi was explicitly out of scope
   for the Immich setup and remains unmet.
5. **Rust-first** — a folio-server on axum reuses the stack already proven by
   the preview dashboard, including its playwright-rust E2E testing.

## Options considered

### Option 1: keep Immich as the family-facing backbone

Continue publishing into Immich; folio remains the archival layer underneath.

- **Pros:** polished UI exists today; mobile apps with auto-backup; mature ML
  (faces, search).
- **Cons:** second metadata authority (driver 1 fails); the documented
  operational fragility continues; upgrades keep requiring a sandbox; remote
  access and per-member permissions stay unsolved; folio-server never gets a
  reason to exist.

### Option 2: folio-server as backbone, vendor apps as channels at the edges

Build `folio-server` (axum) as the owned family-facing surface, incrementally:
gallery first, accounts later. Treat Apple Photos not as a rival but as a
transport channel — shared albums are both an ingest source (family
contributions, harvested with comments/likes/attribution) and a publish target
(curated sets, giving remote family viewing over Apple's infrastructure before
folio-server has accounts or remote access). Channels are declared in a
manifest (`folio-channels.toml`) with per-album direction; "two-way" is
defined as two independent one-way flows joined by dedup, never state
reconciliation. Immich is sunset — at most a transitional viewer, not a
supported target.

- **Pros:** one metadata authority; the whole stack is Rust and testable with
  the existing E2E setup; the container stack, its Postgres anxiety, and the
  upgrade sandbox all go away; family contribution and remote viewing work
  through an app the family already has; each folio-server increment replaces
  one Immich capability at a time.
- **Cons:** folio-server's gallery, accounts, and eventually ML are real work
  that Immich gives away free; the Apple Photos channel only covers the Apple
  ecosystem and rides a reverse-engineered library reader (see ADR-0004);
  shared albums downscale images (acceptable — published derivatives are
  already resized).

### Option 3: full replacement, no vendor channels

folio-server does everything, including its own mobile contribution path (PWA
upload or sync-folder), with no Apple Photos integration.

- **Pros:** maximum independence; no reverse-engineering dependency.
- **Cons:** discards the lowest-friction contribution and remote-sharing path
  iOS users will actually use; the mobile story becomes the hardest part of
  the whole roadmap and blocks family value for a long time.

## Decision outcome

**Chosen option: Option 2** — folio-server as the owned backbone, Apple Photos
as a supported vendor channel at the edges, Immich sunset.

This amends ADR-0001's posture: "interchangeable third-party viewers" becomes
"folio-server is the viewer; third-party tools are optional channels and
legacy apps." The architectural test for any channel is that it never becomes
an authority — originals and sidecars remain truth, and anything a channel
produces that is worth keeping (family comments, likes, contributor
attribution) is harvested *into* sidecars.

A small trait boundary keeps this honest in code: ingest sources (SD card,
folder, Apple Photos album) and publish targets (folio-server library, static
export, Apple Photos album) are adapters. Adapters may shell out to
platform-specific tools; `folio-core` stays pure.

**Trade-offs accepted:**

- folio-server's capabilities arrive incrementally; the family lives with
  shared albums as the interim surface.
- The Apple Photos channel inherits the osxphotos maintenance treadmill
  (ADR-0004 records how that risk is contained).
- Non-Apple family members wait for folio-server accounts (story 012) or use
  static exports (story 022).

## Consequences

- Story 007's publish targets are folio-server, static export, and Apple
  Photos shared albums. No Immich bridge is built.
- Stories 009–011 (bridge, reactive publishing, digests) become the near-term
  family-value arc; stories 012–013 (accounts, remote access) lose urgency.
- The `home-media-management` Immich stack is retired once the bridge and
  gallery cover its role; its Lightroom-era curation loop is superseded by the
  `xmp:Rating` gate (ADR-0003).
- Story 002's schema gains channel-shaped structures: provenance, reactions
  and comments, and a visibility/audience slot.
- Explicitly out of backup scope: channel-side state. Backup fidelity (story
  004) is promised for files + sidecars only.

## Validation

- [ ] Bridge inbound: a photo added to a shared album by a family member lands
      in the archive with contributor, comments, and likes in its sidecar.
- [ ] Bridge outbound: a curated set publishes to a shared album and is
      viewable by remote family with no folio infrastructure exposed.
- [ ] folio-server gallery (story 008) browsable from a family member's device
      on home Wi-Fi, E2E-tested with playwright-rust.
- [ ] Immich instance retired with nothing lost: all metadata that ever lived
      only in its Postgres either harvested or consciously abandoned.
