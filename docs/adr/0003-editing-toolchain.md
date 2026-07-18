# ADR 0003: Editing toolchain and Lightroom legacy support

**Status:** Proposed

**Date:** 2026-07-18

**Related Documents:**
- [ADR-0001: Metadata and Catalog Architecture](0001-metadata-catalog-architecture.md)
- [Roadmap](../roadmap.md) (stories 007, 014, 015)

---

## Context and problem statement

ADR-0001 assumed Lightroom would remain the editing tool "today" with
alternatives possible "tomorrow." Resuming development is the natural point to
decide whether tomorrow has arrived: should the workflow move to open-source
tools for raw developing, cataloguing, and publishing?

Lightroom currently bundles three roles:

1. **Raw develop** — non-destructive editing of NEF (and JPEG) captures.
2. **Cataloguing** — ratings, keywords, collections in the `.lrcat` database.
3. **Publish services** — the smart-collection loop (rating ≥ 3★ → export
   sRGB JPEG to the NAS) that fed the family library.

Two facts shape the decision. First, non-destructive edit recipes are
proprietary in *every* tool — Lightroom develop settings and darktable's
history stack each live in their own XMP namespace. No choice of editor makes
edit recipes portable; the durable artifacts are originals, standard XMP
metadata, and rendered derivatives. Second, Adobe allows cancelling and
reactivating a subscription month-to-month, so dropping Lightroom is not a
one-way door: any old edit recipe stays accessible for the cost of one month's
subscription, provided the catalog file is preserved.

## Decision drivers

1. **Vendor independence** — curation state (ratings, keywords) must live in
   sidecars, not in any editor's database.
2. **Cost and openness** — an open-source toolchain removes the subscription
   and aligns with the project's principles.
3. **No data loss** — existing Lightroom ratings, keywords, and edits must
   survive the transition.
4. **Capture reality** — captures are sometimes NEF, sometimes JPEG (Nikon
   D800), so the raw tool matters.
5. **Solo maintainability** — the learning-curve cost falls only on the
   archivist, not the family.

## Options considered

### Option 1: stay on Lightroom

- **Pros:** no learning curve; edit recipes stay in the active tool.
- **Cons:** ongoing subscription; curation stays gated behind a proprietary
  app; the publish loop remains a Lightroom feature rather than a folio
  capability; the vendor-dependence ADR-0001 set out to end persists.

### Option 2: open-source toolchain, Lightroom retained as a legacy app

darktable for raw develop (XMP sidecars are its native state storage — it
writes them by default, with ratings/tags/labels in standard XMP fields);
digiKam as the interim desktop DAM (strong XMP read/write, local face
detection) until `folio-catalog` matures; folio itself takes over the publish
role (story 007) by reading `xmp:Rating` from sidecars — making the curation
gate tool-agnostic and folio-owned. Lightroom is kept as a *supported legacy
app*: metadata flushed to XMP while a subscription window is active, keeper
renders exported, and the `.lrcat` catalog archived with a documented
reactivation procedure (story 014).

- **Pros:** subscription ends without burning the bridge; curation moves into
  the open (any XMP-writing tool can rate a photo and folio's gate sees it);
  darktable's sidecar-native workflow matches folio's architecture better than
  Lightroom's ever did; folio gains the publish pipeline as a first-class
  capability.
- **Cons:** darktable's learning curve is real; digiKam overlaps with what
  `folio-catalog` will become (acceptable — it is explicitly interim);
  Lightroom edit recipes are frozen unless reactivated; folio must ingest NEF
  as a first-class format (story 001 slice 3e), including RAW+JPEG pairing and
  tolerating both sidecar naming conventions (`basename.xmp` from Lightroom,
  `basename.ext.xmp` from darktable).

### Option 3: hard exit from Lightroom

Same toolchain as Option 2, but treat the migration as final: export
everything, discard the catalog.

- **Pros:** simplest end state.
- **Cons:** needlessly destroys the edit-recipe archive when month-to-month
  reactivation makes preserving it nearly free. Rejected on the "never lose
  data" principle alone.

## Decision outcome

**Chosen option: Option 2** — darktable primary for raw develop, digiKam as
interim DAM, folio owns publishing, Lightroom maintained as a legacy app.

The general pattern this establishes: a **legacy application archive** —
proprietary tool state preserved safely alongside the vendor-neutral truth,
accessible on demand, never load-bearing. Lightroom is the first instance;
the same pattern would apply to any future tool the workflow moves away from.

**Trade-offs accepted:**

- Edit recipes made in Lightroom render only in Lightroom; the exported
  keeper renders are the working copies going forward.
- darktable's edit stack is likewise darktable-only (though at least it lives
  as plain text in the sidecar); the publish pipeline is what makes any edit
  permanent.
- Video editing stays out of scope entirely (Kdenlive or DaVinci Resolve ad
  hoc; folio handles video cataloguing only).

## Consequences

- Story 001 gains slice 3e (NEF + RAW+JPEG pairing) ahead of the backlog run.
- Story 002's schema models assets with multiple representations and both
  sidecar naming conventions from the start.
- Story 007's curation gate reads `xmp:Rating` — no editor-specific coupling.
- Story 014 (Lightroom legacy support) has a sequencing constraint: the XMP
  flush and keeper-render export happen while a subscription window is active.
  There is no deadline pressure beyond that, since reactivation is always
  possible if the catalog file is preserved.
- Story 015 validates sidecar round-trips against darktable and digiKam
  specifically.

## Validation

- [ ] A photo rated in darktable is picked up by folio's publish gate with no
      Lightroom involvement.
- [ ] A sidecar written by folio is read correctly by darktable and digiKam
      (and vice versa), per story 015.
- [ ] The archived `.lrcat` opens successfully in a reactivated Lightroom and
      renders a historical edit (test once, before archiving).
- [ ] All Lightroom ratings/keywords verified present in sidecars before the
      subscription lapses.
