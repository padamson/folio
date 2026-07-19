# ADR 0005: Split synthetic-media generation from folio's test harness

**Status:** Proposed

**Date:** 2026-07-18

**Related Documents:**
- [Roadmap](../roadmap.md)
- [ADR-0004: Upstream tooling strategy](0004-upstream-tooling-strategy.md)
- Implementation plan: [001 backlog ingestion](../implementation-plans/001-backlog-ingestion-plan.md), slice 3d

---

## Context and problem statement

Folio needs synthetic photo/video collections to exercise ingestion in tests
(slice 3d). The `folio-examples` crate that provides them conflates three
different concerns in one place:

1. **A generation engine** — turn a prompt into images via a local or remote
   model, stamp EXIF capture times, synthesize videos, spread items across a
   timeline.
2. **Folio's specific test scenario** — the Thanksgiving prompts, the
   `DCIM/100NIKON` SD-card layout, D800 EXIF, `DSC_####` naming, and the
   dedup fixtures.
3. **Folio's prebuilt-data distribution** — generate → tar → publish to the
   `folio-example-data` GitHub release, and the `download-examples` side that
   fetches, verifies, and extracts, keyed to the folio workspace version.

Only the first is general. There is a standing want for a plain Rust tool that
generates collections of AI photos and/or videos from a prompt — using either
a local model or a remote/frontier one — with uses beyond folio. Trapping that
engine inside folio's test harness makes it unreusable, and the coupling would
be expensive to unwind later. The engine also has real defects the slice 3d
review found (videos carry no capture metadata, ComfyUI stall detection is
broken) that are engine-level, not folio-specific.

## Decision drivers

1. **Reuse** — a "prompt → collection of AI photos/videos" tool is useful on
   its own; the engine should not depend on folio.
2. **Avoid tech debt that blocks future publishing** — structural coupling is
   cheap to prevent now and expensive to unwind after a crate is published.
   The inverse — publishing *machinery* — is overhead we don't need yet.
3. **No premature publishing** — installable folio apps are the only near-term
   distribution goal; nobody is consuming folio's libraries externally yet.
4. **Finish slice 3d properly** — the split is the natural shape for the
   cleanup, and the engine's defects belong in the engine.

## Decision outcome

Split into two workspace crates, both `publish = false` for now.

**`folio-generator`** (new) — the general engine, a library plus a standalone
CLI so it is usable as a tool:

- Input: a prompt (or a set of prompts / a small scene spec) plus a count.
- Output: a collection of photos and/or videos written to a caller-chosen
  directory, with realistic EXIF capture timestamps spread across a
  configurable timeline (optionally in multiple temporal batches with gaps).
- Backends behind an abstraction: **local** (ComfyUI / Stable Diffusion over
  HTTP) and **remote/frontier** (OpenAI DALL-E today, extensible to other
  providers). Video is frame-synthesized via ffmpeg for now, with the backend
  seam left open to a real video-generation model later.
- No folio coupling: **no dependency on `folio-core`**, no folio scenario, no
  GitHub distribution. The caller supplies output layout, naming, and the
  scenario.

**`folio-examples`** (stays) — folio's test harness, consuming
`folio-generator` and adding only the folio-specific parts:

- The Thanksgiving scenario, the `DCIM/100NIKON` layout, D800 EXIF, `DSC_####`
  naming, and the dedup fixtures.
- The distribution glue: build the tarball, publish it to the
  `folio-example-data` release, and `download-examples` to fetch/verify/extract
  it, keyed to the folio workspace version.

**Dependency direction:** `folio-examples` → `folio-generator`, never the
reverse. `folio-generator` never depends on `folio-core`. Folio produces test
data with the generator and reads it back through its normal ingestion path;
the two never share types.

**Publishing posture (near term):** `folio-cli` and `folio-server` are the
installable deliverables (`cargo install`). `folio-core`, `folio-ingest`, and
`folio-generator` are libraries that *may* be published later but carry no
near-term external-API commitment. Versions stay lockstep on the workspace
version until a real external consumer needs otherwise.

**Deferred deliberately** (cheap to defer, expensive to over-build now):

- Publishing `folio-generator` to crates.io — revisit when a concrete external
  consumer appears. Until then, other projects reuse it locally (path or git
  dependency).
- A neutral, non-`folio` crate name — a folio-branded name is a poor fit for a
  general tool, but renaming an *unpublished* crate is trivial. Keeping the
  crate dependency-clean is what makes future extraction cheap; the name is not.
  Revisit at publish time.
- Independent per-crate SemVer and any multi-crate release machinery — lockstep
  until there's a reason not to.
- Public-API stability guarantees — `publish = false` means the engine's API
  can iterate freely.

**Timing:** execute the split as part of completing slice 3d. The engine-level
items on the 3d remaining-work list (video capture metadata, exiftool
fail-fast, ComfyUI stall-detection fix) land in `folio-generator`; the
distribution-hardening items (safe extraction, checksum verification) stay in
`folio-examples`.

## Consequences

- `folio-generator` becomes a genuinely reusable tool; the other
  "generate batches of photos/videos" use cases consume it locally with no
  publish step.
- `folio-examples` shrinks to scenario + distribution, and its defects get
  fixed at the right layer.
- One monorepo with mixed `publish` flags — no new repository, no
  `git subtree split`. (This is the lower-overhead alternative to the
  incubate-then-spin-out pattern; the repo home and the future publish decision
  are independent.)
- Slice 3d's definition of done expands to "engine extracted and decoupled";
  the 001 implementation plan is updated accordingly.
- Architecturally `folio-generator` is test-data *tooling*, a producer that
  sits outside folio's runtime and outside the core hexagon — like
  `folio-examples`, not part of the domain/application/adapter layering.

## Validation

- [ ] `folio-generator` builds and its tests pass with **no** dependency on
      `folio-core` (`cargo tree -p folio-generator | grep folio-core` is empty).
- [ ] A minimal, folio-agnostic example — a prompt and a count producing a
      directory of photos/videos with correct EXIF capture timestamps — works
      using only `folio-generator`'s public API.
- [ ] `folio-examples` reproduces the existing Thanksgiving dataset via
      `folio-generator`, with the correct two-batch temporal structure and
      video capture metadata.
- [ ] No folio-specific strings (`Thanksgiving`, `DCIM`, `DSC_`,
      `folio-example-data`) remain anywhere in `folio-generator`.
